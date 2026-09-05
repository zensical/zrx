// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Typed, transferable, and batched external input sessions.

use ahash::HashMap;
use crossbeam::channel::{self, Receiver, Select, Sender, TryRecvError};
use std::mem;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use thiserror::Error as ThisError;

use zrx_executor::Strategy;

use crate::scheduler::action::{Port, Segment};
use crate::scheduler::event::{Change, Event, Kind, Revision};
use crate::scheduler::plan::InputId;
use crate::scheduler::runtime::Runtime;
use crate::scheduler::{Admit, RevisionId};
use crate::scheduler::{Id, Value};

// Bootstrap transport choices owned by sessions. They affect batching and
// backpressure only; later feedback may tune them without changing framing.
const BOOTSTRAP_EVENT_CAPACITY: usize = 64;
const BOOTSTRAP_BATCH_ITEMS: usize = 1_024;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

trait Receive<I>: Send
where
    I: Id,
{
    fn event(&self) -> Result<Envelope<I>, TryRecvError>;
    fn abort(&self) -> Result<Revision, TryRecvError>;
    fn register<'a>(
        &'a self, select: &mut Select<'a>, events: bool, aborts: bool,
        accept: &mut dyn FnMut(usize),
    );
}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid session acquisition or event submission.
#[derive(Clone, Debug, PartialEq, Eq, ThisError)]
pub enum Error {
    /// The selected input is not installed.
    #[error("external input {0:?} is not installed")]
    Input(InputId),
    /// The selected input carries another value type.
    #[error("external input {0:?} carries another value type")]
    Port(InputId),
    /// The selected input already has an authoritative session.
    #[error("external input {0:?} already has a session")]
    Installed(InputId),
    /// The attached plan was detached.
    #[error("scheduler session is disconnected")]
    Disconnected,
    /// The session exhausted its source revision identity space.
    #[error("scheduler session revision identity exhausted")]
    RevisionExhausted,
}

// ----------------------------------------------------------------------------

enum Envelope<I> {
    Begin(Revision),
    Changes(Revision, Segment<I>),
    End(Revision),
    Abort(Revision),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Typed transferable capability for one installed scheduler input.
#[must_use]
pub struct Session<I, V> {
    events: Sender<SessionEvent<I, V>>,
    aborts: Sender<Revision>,
    next_revision: u64,
    batch_items: usize,
    // Must drop after both sender fields, to publish disconnection readiness.
    notify: Notify,
}

// ----------------------------------------------------------------------------

// Marks both publication and final sender teardown for the receiving inbox.
struct Notify(Arc<AtomicBool>);

// ----------------------------------------------------------------------------

/// Affine source revision writer that batches individual changes.
///
/// A source must submit at most one final net change for each key in a
/// revision. The scheduler preserves submission order and does not retain or
/// consolidate a revision-wide key map to enforce this provider contract.
#[must_use = "an open session writer must be sealed or aborted"]
pub struct Writer<I, V> {
    session: Option<Session<I, V>>,
    revision: Revision,
    items: Vec<Change<I, V>>,
    closed: bool,
}

// ----------------------------------------------------------------------------

struct ReceiverFor<I, V> {
    events: Receiver<SessionEvent<I, V>>,
    aborts: Receiver<Revision>,
}

// ----------------------------------------------------------------------------

struct State<I>
where
    I: Id,
{
    input: InputId,
    receiver: Box<dyn Receive<I>>,
    pending: Option<Envelope<I>>,
    open: Option<(Revision, RevisionId)>,
    implicit_abort: Option<Revision>,
    events_open: bool,
    aborts_open: bool,
    ready: Arc<AtomicBool>,
}

// ----------------------------------------------------------------------------

struct Selected<I>
where
    I: Id,
{
    index: usize,
    input: InputId,
    event: Envelope<I>,
}

// ----------------------------------------------------------------------------

pub(super) struct Inboxes<I>
where
    I: Id,
{
    states: Vec<State<I>>,
    by_input: HashMap<InputId, usize>,
    next: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, V> Session<I, V>
where
    I: Id,
    V: Value,
{
    /// Opens one affine writer for a fresh source revision.
    ///
    /// Submission blocks only while the session's bounded event channel is
    /// full. Moving the session to a provider thread keeps scheduler memory
    /// bounded without blocking scheduler orchestration.
    ///
    /// # Errors
    ///
    /// Returns an error if the plan was detached or revision identities are
    /// exhausted.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "session.begin",
            skip_all,
            fields(revision = self.next_revision)
        )
    )]
    pub fn begin(mut self) -> Result<Writer<I, V>, Error> {
        let revision = Revision::new(self.next_revision);
        self.next_revision = self
            .next_revision
            .checked_add(1)
            .ok_or(Error::RevisionExhausted)?;
        self.publish(Event::new(revision, Kind::Begin))
            .map_err(|_| Error::Disconnected)?;
        Ok(Writer {
            session: Some(self),
            revision,
            items: Vec::new(),
            closed: false,
        })
    }

    fn publish(
        &self, event: SessionEvent<I, V>,
    ) -> Result<(), channel::SendError<SessionEvent<I, V>>> {
        let result = self.events.send(event);
        // Publish readiness only after the bounded channel owns the event.
        self.notify.mark();
        result
    }
}

// ----------------------------------------------------------------------------

// Cross-crate inlining of the incremental writer path recovered measured
// throughput without changing its code or allocation shape.
#[allow(clippy::inline_always)]
impl<I, V> Writer<I, V>
where
    I: Id,
    V: Value,
{
    /// Consumes at most one scheduler-native batch from an iterator.
    ///
    /// The iterator retains every unconsumed item. A nonempty partial batch is
    /// flushed before returning, so one call submits at most one changes
    /// event without staging another collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch can no longer reach its detached plan.
    pub fn emit_from<C>(&mut self, changes: &mut C) -> Result<usize, Error>
    where
        C: Iterator<Item = Change<I, V>>,
    {
        let remaining = self.session().batch_items - self.items.len();
        let mut emitted = 0;
        for change in changes.take(remaining) {
            self.emit(change)?;
            emitted += 1;
        }
        self.flush()?;
        Ok(emitted)
    }

    /// Records an exact-size batch without staging it in another collection.
    ///
    /// Changes are split at the scheduler's internal segment size.
    ///
    /// # Errors
    ///
    /// Returns an error if a full segment can no longer reach its detached
    /// plan.
    ///
    /// # Panics
    ///
    /// Panics if the iterator violates its [`ExactSizeIterator`] contract.
    pub fn emit_batch<C>(&mut self, changes: C) -> Result<(), Error>
    where
        C: IntoIterator<Item = Change<I, V>>,
        C::IntoIter: ExactSizeIterator,
    {
        let mut changes = changes.into_iter();
        let count = changes.len();
        if count == 0 {
            return Ok(());
        }

        self.flush()?;
        let batch = self.session().batch_items;
        let mut emitted = 0;
        while emitted < count {
            let take = (count - emitted).min(batch);
            self.items.reserve_exact(batch);
            self.items.extend(changes.by_ref().take(take));
            emitted += take;
            if self.items.len() == batch {
                self.flush()?;
            }
        }
        debug_assert!(changes.next().is_none());
        Ok(())
    }

    /// Records one change and flushes when the scheduler's internal batch is
    /// full.
    ///
    /// # Errors
    ///
    /// Returns an error if a full batch can no longer reach its detached plan.
    #[inline(always)]
    pub fn emit(&mut self, change: Change<I, V>) -> Result<(), Error> {
        if self.items.is_empty() {
            self.items.reserve_exact(self.session().batch_items);
        }
        self.items.push(change);
        if self.items.len() == self.session().batch_items {
            self.flush()?;
        }
        Ok(())
    }

    /// Records one insertion or replacement.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::emit`].
    #[inline(always)]
    pub fn insert(&mut self, key: I, value: V) -> Result<(), Error> {
        self.emit(Change::Insert(key, value))
    }

    /// Records one removal.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::emit`].
    #[inline(always)]
    pub fn remove(&mut self, key: I) -> Result<(), Error> {
        self.emit(Change::Remove(key))
    }

    /// Sends the currently accumulated changes as one owned batch.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the plan was detached.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "session.flush",
            skip_all,
            fields(
                revision = ?self.revision,
                batch_items = self.items.len(),
            )
        )
    )]
    pub fn flush(&mut self) -> Result<(), Error> {
        if self.items.is_empty() {
            return Ok(());
        }
        let items = mem::take(&mut self.items);
        match self
            .session()
            .publish(Event::new(self.revision, Kind::Changes(items)))
        {
            Ok(()) => Ok(()),
            Err(error) => {
                let (_, Kind::Changes(items)) = error.0.into_parts() else {
                    unreachable!("flush sent one changes event")
                };
                self.items = items;
                Err(Error::Disconnected)
            }
        }
    }

    /// Flushes pending changes, seals the revision, and returns its session.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the plan was detached.
    ///
    /// # Panics
    ///
    /// Panics only if internal writer ownership was corrupted.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "session.seal",
            skip_all,
            fields(revision = ?self.revision)
        )
    )]
    pub fn seal(mut self) -> Result<Session<I, V>, Error> {
        self.flush()?;
        self.send(Kind::End)?;
        self.closed = true;
        Ok(self.session.take().expect("open writer owns its session"))
    }

    /// Discards locally pending changes, aborts the revision, and returns its
    /// session.
    ///
    /// Changes already flushed to the scheduler are fenced if undispatched,
    /// but already dispatched action work remains committed. Abort is not a
    /// transactional rollback of action-owned state.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the plan was detached.
    ///
    /// # Panics
    ///
    /// Panics only if internal writer ownership was corrupted.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "session.abort",
            skip_all,
            fields(revision = ?self.revision)
        )
    )]
    pub fn abort(mut self) -> Result<Session<I, V>, Error> {
        self.items.clear();
        self.send(Kind::Abort)?;
        self.closed = true;
        Ok(self.session.take().expect("open writer owns its session"))
    }

    fn send(&self, kind: Kind<Vec<Change<I, V>>>) -> Result<(), Error> {
        self.session()
            .publish(Event::new(self.revision, kind))
            .map_err(|_| Error::Disconnected)
    }

    #[inline(always)]
    fn session(&self) -> &Session<I, V> {
        self.session.as_ref().expect("open writer owns its session")
    }
}

// ----------------------------------------------------------------------------

impl<I> Inboxes<I>
where
    I: Id,
{
    pub(super) fn admit<S>(&mut self, runtime: &mut Runtime<I, S>) -> bool
    where
        S: Strategy,
    {
        let (selected, progressed) = self.select();
        let Some(selected) = selected else {
            return progressed;
        };
        let Selected { index, input, event } = selected;
        match event {
            Envelope::Begin(source) => {
                assert!(
                    self.revision(index, source).is_none(),
                    "session began an already open source revision"
                );
                match runtime
                    .begin(input)
                    .expect("installed session input remains valid")
                {
                    Admit::Accepted(revision) => {
                        self.open(index, source, revision);
                        true
                    }
                    Admit::Full(_) => {
                        self.restore(Selected {
                            index,
                            input,
                            event: Envelope::Begin(source),
                        });
                        progressed
                    }
                }
            }
            Envelope::Changes(source, segment) => {
                let revision = self
                    .revision(index, source)
                    .expect("session changes belong to its open revision");
                match runtime
                    .ingress(revision, segment)
                    .expect("typed session emits its installed port")
                {
                    Admit::Accepted(()) => true,
                    Admit::Full(segment) => {
                        self.restore(Selected {
                            index,
                            input,
                            event: Envelope::Changes(source, segment),
                        });
                        progressed
                    }
                }
            }
            Envelope::End(source) => {
                let revision = self
                    .revision(index, source)
                    .expect("session end belongs to its open revision");
                match runtime
                    .seal(revision)
                    .expect("session seals one current revision")
                {
                    Admit::Accepted(()) => {
                        self.close(index, source);
                        true
                    }
                    Admit::Full(_) => {
                        self.restore(Selected {
                            index,
                            input,
                            event: Envelope::End(source),
                        });
                        progressed
                    }
                }
            }
            Envelope::Abort(source) => {
                let revision = self
                    .revision(index, source)
                    .expect("session abort belongs to its open revision");
                match runtime
                    .abort(revision)
                    .expect("session aborts one current revision")
                {
                    Admit::Accepted(()) => {
                        self.close(index, source);
                        true
                    }
                    Admit::Full(_) => {
                        self.restore(Selected {
                            index,
                            input,
                            event: Envelope::Abort(source),
                        });
                        progressed
                    }
                }
            }
        }
    }

    pub(super) fn install<V>(
        &mut self, input: InputId, port: Port,
    ) -> Result<Session<I, V>, Error>
    where
        V: Value,
    {
        if self.by_input.contains_key(&input) {
            return Err(Error::Installed(input));
        }
        if port != Port::of::<I, V>() {
            return Err(Error::Port(input));
        }
        let (events, event_receiver) =
            channel::bounded(BOOTSTRAP_EVENT_CAPACITY);
        let (aborts, abort_receiver) = channel::bounded(1);
        let index = self.states.len();
        let ready = Arc::new(AtomicBool::new(false));
        self.states.push(State {
            input,
            receiver: Box::new(ReceiverFor::<I, V> {
                events: event_receiver,
                aborts: abort_receiver,
            }),
            pending: None,
            open: None,
            implicit_abort: None,
            events_open: true,
            aborts_open: true,
            ready: Arc::clone(&ready),
        });
        assert!(self.by_input.insert(input, index).is_none());
        Ok(Session {
            events,
            aborts,
            next_revision: 0,
            batch_items: BOOTSTRAP_BATCH_ITEMS,
            notify: Notify(ready),
        })
    }

    fn select(&mut self) -> (Option<Selected<I>>, bool) {
        let len = self.states.len();
        let mut progressed = false;
        for _ in 0..len {
            let index = self.next;
            self.next = (self.next + 1) % len;
            let state = &mut self.states[index];
            // Clear before polling so concurrent publication stays marked.
            // A restored event must be retried even without another send.
            if state.pending.is_none()
                && !state.ready.swap(false, Ordering::AcqRel)
            {
                continue;
            }

            if state.aborts_open {
                match state.receiver.abort() {
                    Ok(revision) => {
                        assert!(
                            state.implicit_abort.replace(revision).is_none()
                        );
                    }
                    Err(TryRecvError::Disconnected) => {
                        state.aborts_open = false;
                        progressed = true;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }

            if let Some(event) = state.pending.take() {
                state.ready.store(true, Ordering::Release);
                return (
                    Some(Selected {
                        index,
                        input: state.input,
                        event,
                    }),
                    progressed,
                );
            }

            if state.events_open {
                match state.receiver.event() {
                    Ok(event) => {
                        // A send may have queued several events under one mark.
                        state.ready.store(true, Ordering::Release);
                        return (
                            Some(Selected {
                                index,
                                input: state.input,
                                event,
                            }),
                            progressed,
                        );
                    }
                    Err(TryRecvError::Disconnected) => {
                        state.events_open = false;
                        progressed = true;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
            if !state.events_open
                && let Some(revision) = state.implicit_abort.take()
            {
                state.ready.store(true, Ordering::Release);
                return (
                    Some(Selected {
                        index,
                        input: state.input,
                        event: Envelope::Abort(revision),
                    }),
                    progressed,
                );
            }
        }
        (None, progressed)
    }

    fn restore(&mut self, selected: Selected<I>) {
        let state = &mut self.states[selected.index];
        assert!(state.pending.replace(selected.event).is_none());
    }

    fn open(&mut self, index: usize, event: Revision, revision: RevisionId) {
        let state = &mut self.states[index];
        assert!(state.open.replace((event, revision)).is_none());
    }

    fn revision(&self, index: usize, event: Revision) -> Option<RevisionId> {
        self.states[index]
            .open
            .filter(|(current, _)| *current == event)
            .map(|(_, revision)| revision)
    }

    fn close(&mut self, index: usize, event: Revision) {
        let state = &mut self.states[index];
        assert_eq!(state.open.take().map(|(current, _)| current), Some(event));
    }

    pub(super) fn register<'a>(
        &'a self, select: &mut Select<'a>, mut accept: impl FnMut(usize),
    ) {
        for state in &self.states {
            state.receiver.register(
                select,
                state.events_open,
                state.aborts_open,
                &mut accept,
            );
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, V> Drop for Writer<I, V> {
    fn drop(&mut self) {
        if !self.closed
            && let Some(session) = &self.session
        {
            // One affine writer can produce at most one implicit abort, so
            // the dedicated one-slot control path cannot fill.
            let _ = session.aborts.try_send(self.revision);
            session.notify.mark();
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, V> Receive<I> for ReceiverFor<I, V>
where
    I: Id,
    V: Value,
{
    fn event(&self) -> Result<Envelope<I>, TryRecvError> {
        let (revision, kind) = self.events.try_recv()?.into_parts();
        Ok(match kind {
            Kind::Begin => Envelope::Begin(revision),
            Kind::Changes(items) => {
                Envelope::Changes(revision, Segment::new(items))
            }
            Kind::End => Envelope::End(revision),
            Kind::Abort => Envelope::Abort(revision),
        })
    }

    fn abort(&self) -> Result<Revision, TryRecvError> {
        self.aborts.try_recv()
    }

    fn register<'a>(
        &'a self, select: &mut Select<'a>, events: bool, aborts: bool,
        accept: &mut dyn FnMut(usize),
    ) {
        if events {
            accept(select.recv(&self.events));
        }
        if aborts {
            accept(select.recv(&self.aborts));
        }
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Inboxes<I>
where
    I: Id,
{
    fn default() -> Self {
        Self {
            states: Vec::new(),
            by_input: HashMap::default(),
            next: 0,
        }
    }
}

// ----------------------------------------------------------------------------

impl Notify {
    fn mark(&self) {
        self.0.store(true, Ordering::Release);
    }
}

// ----------------------------------------------------------------------------

impl Drop for Notify {
    fn drop(&mut self) {
        self.mark();
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type SessionEvent<I, V> = Event<Vec<Change<I, V>>>;

#[cfg(test)]
mod tests {
    use super::{BOOTSTRAP_EVENT_CAPACITY, Inboxes};
    use crate::scheduler::action::Port;
    use crate::scheduler::plan::InputId;
    use crossbeam::channel::Select;
    use std::time::Duration;

    #[test]
    fn full_session_requires_a_receiver_credit_for_the_next_flush() {
        let mut inboxes = Inboxes::<u64>::default();
        let session = inboxes
            .install::<u64>(InputId::new(1), Port::of::<u64, u64>())
            .unwrap();
        let mut writer = session.begin().unwrap();
        // Begin occupies one position; flush each item as a separate event.
        for value in 1..BOOTSTRAP_EVENT_CAPACITY {
            writer.insert(value as u64, value as u64).unwrap();
            writer.flush().unwrap();
        }
        assert!(writer.session().events.is_full());
        let value = BOOTSTRAP_EVENT_CAPACITY as u64;
        writer.insert(value, value).unwrap();
        {
            let mut select = Select::new();
            select.send(&writer.session().events);
            assert!(select.ready_timeout(Duration::ZERO).is_err());
        }
        // Consume exactly one event, independently of thread scheduling.
        assert!(inboxes.states[0].receiver.event().is_ok());
        assert_eq!(writer.session().events.len(), BOOTSTRAP_EVENT_CAPACITY - 1);
        writer.flush().unwrap();
        assert!(writer.session().events.is_full());
        assert!(writer.items.is_empty());
        let mut select = Select::new();
        select.send(&writer.session().events);
        assert!(select.ready_timeout(Duration::ZERO).is_err());
    }
}
