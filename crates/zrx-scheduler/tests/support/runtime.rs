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

//! Scheduler runtime test support.

#![allow(dead_code)]

use crossbeam::channel::Select;
use std::any::Any;
use std::collections::HashMap;

use zrx_executor::Strategy;
use zrx_executor::strategy::Immediate;

use zrx_scheduler::Change;
use zrx_scheduler::plan::InputId;
use zrx_scheduler::{
    CurrentError, Egress, Error, Id, Plan, Readiness, Report, Scheduler,
    Session, SessionError, Value, Writer,
};

const BATCH_ITEMS: usize = 1_024;
const SESSION_ENTRIES: usize = 64;

pub struct Batch<I, V>(Vec<Change<I, V>>);

impl<I, V> Batch<I, V> {
    pub fn new(changes: Vec<Change<I, V>>) -> Self {
        Self(changes)
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Revision(u64);

trait ErasedSession<I>: Send
where
    I: Id,
{
    fn begin(self: Box<Self>)
    -> Result<Box<dyn ErasedWriter<I>>, SessionError>;
}

trait ErasedWriter<I>: Send
where
    I: Id,
{
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn seal(self: Box<Self>)
    -> Result<Box<dyn ErasedSession<I>>, SessionError>;
    fn abort(
        self: Box<Self>,
    ) -> Result<Box<dyn ErasedSession<I>>, SessionError>;
}

struct TypedSession<I, V>(Session<I, V>);

impl<I, V> ErasedSession<I> for TypedSession<I, V>
where
    I: Id,
    V: Value,
{
    fn begin(
        self: Box<Self>,
    ) -> Result<Box<dyn ErasedWriter<I>>, SessionError> {
        let writer = self.0.begin()?;
        Ok(Box::new(TypedWriter(writer)))
    }
}

struct TypedWriter<I, V>(Writer<I, V>);

impl<I, V> ErasedWriter<I> for TypedWriter<I, V>
where
    I: Id,
    V: Value,
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn seal(
        self: Box<Self>,
    ) -> Result<Box<dyn ErasedSession<I>>, SessionError> {
        Ok(Box::new(TypedSession(self.0.seal()?)))
    }

    fn abort(
        self: Box<Self>,
    ) -> Result<Box<dyn ErasedSession<I>>, SessionError> {
        Ok(Box::new(TypedSession(self.0.abort()?)))
    }
}

struct Open<I>
where
    I: Id,
{
    input: InputId,
    writer: Box<dyn ErasedWriter<I>>,
}

pub struct Tick {
    progressed: bool,
    report: Report,
}

impl Tick {
    pub const fn progressed(&self) -> bool {
        self.progressed
    }

    pub fn into_report(self) -> Report {
        self.report
    }
}

pub struct Runtime<I, S = Immediate>
where
    I: Id,
    S: Strategy,
{
    scheduler: Scheduler<I, S>,
    plan: zrx_scheduler::PlanId,
    sessions: HashMap<InputId, Box<dyn ErasedSession<I>>>,
    revisions: HashMap<Revision, Open<I>>,
    pending_progress: bool,
    pending_report: Report,
    batch_items: usize,
    channel_entries: usize,
    inline: bool,
    next_revision: u64,
}

impl<I> Runtime<I, Immediate>
where
    I: Id,
{
    pub fn new(plan: Plan<I>) -> Self {
        let mut scheduler = Scheduler::inline();
        let plan = scheduler.attach(plan);
        Self {
            scheduler,
            plan,
            sessions: HashMap::new(),
            revisions: HashMap::new(),
            pending_progress: false,
            pending_report: Report::default(),
            batch_items: BATCH_ITEMS,
            channel_entries: SESSION_ENTRIES,
            inline: true,
            next_revision: 0,
        }
    }
}

impl<I, S> Runtime<I, S>
where
    I: Id,
    S: Strategy,
{
    pub fn with_strategy(plan: Plan<I>, strategy: S) -> Self {
        let channel_entries = SESSION_ENTRIES;
        let mut scheduler = Scheduler::new(strategy);
        let plan = scheduler.attach(plan);
        Self {
            scheduler,
            plan,
            sessions: HashMap::new(),
            revisions: HashMap::new(),
            pending_progress: false,
            pending_report: Report::default(),
            batch_items: BATCH_ITEMS,
            channel_entries,
            inline: false,
            next_revision: 0,
        }
    }

    pub fn begin(&mut self, input: InputId) -> Result<Revision, Error> {
        let session = match self.sessions.remove(&input) {
            Some(session) => session,
            None => self.install(input)?,
        };
        let writer = session.begin()?;
        let revision = Revision(self.next_revision);
        self.next_revision = self
            .next_revision
            .checked_add(1)
            .expect("test revision identity exhausted");
        assert!(
            self.revisions
                .insert(revision, Open { input, writer })
                .is_none(),
            "session revision identity was reused while open"
        );
        self.advance();
        Ok(revision)
    }

    pub fn ingress<V>(
        &mut self, revision: Revision, batch: Batch<I, V>,
    ) -> Result<(), Error>
    where
        V: Value,
    {
        let batches = batch.0.len().div_ceil(self.batch_items);
        let advance_batches = batches >= self.channel_entries;
        for (index, change) in batch.0.into_iter().enumerate() {
            {
                let open = self
                    .revisions
                    .get_mut(&revision)
                    .expect("test revision remains open");
                let writer = open
                    .writer
                    .as_any_mut()
                    .downcast_mut::<TypedWriter<I, V>>()
                    .ok_or(SessionError::Port(open.input))?;
                writer.0.emit(change)?;
            }
            if advance_batches
                && (index + 1) % (self.batch_items * self.channel_entries) == 0
            {
                self.advance_all();
            }
        }
        {
            let open = self
                .revisions
                .get_mut(&revision)
                .expect("test revision remains open");
            let writer = open
                .writer
                .as_any_mut()
                .downcast_mut::<TypedWriter<I, V>>()
                .ok_or(SessionError::Port(open.input))?;
            writer.0.flush()?;
        }
        self.advance();
        Ok(())
    }

    pub fn seal(&mut self, revision: Revision) -> Result<(), Error> {
        let open = self
            .revisions
            .remove(&revision)
            .expect("test revision remains open");
        let session = open.writer.seal()?;
        assert!(self.sessions.insert(open.input, session).is_none());
        self.advance();
        Ok(())
    }

    pub fn abort(&mut self, revision: Revision) -> Result<(), Error> {
        let open = self
            .revisions
            .remove(&revision)
            .expect("test revision remains open");
        let session = open.writer.abort()?;
        assert!(self.sessions.insert(open.input, session).is_none());
        self.advance();
        Ok(())
    }

    fn install(
        &mut self, input: InputId,
    ) -> Result<Box<dyn ErasedSession<I>>, Error> {
        let session =
            self.scheduler.attachment(self.plan)?.session::<u64>(input);
        match session {
            Ok(session) => Ok(Box::new(TypedSession(session))),
            Err(Error::Session(SessionError::Port(_))) => {
                let session = self
                    .scheduler
                    .attachment(self.plan)?
                    .session::<usize>(input)?;
                Ok(Box::new(TypedSession(session)))
            }
            Err(error) => Err(error),
        }
    }

    fn advance(&mut self) {
        if let Some(tick) = self.scheduler.tick() {
            self.pending_progress |= tick.progressed();
            self.pending_report.append(tick.into_report());
        }
    }

    fn advance_all(&mut self) {
        while let Some(tick) = self.scheduler.tick() {
            self.pending_progress |= tick.progressed();
            self.pending_report.append(tick.into_report());
        }
    }

    pub fn egress(&mut self) -> Option<Egress<I>> {
        self.scheduler.attachment(self.plan).unwrap().egress()
    }

    pub fn errors(&self) -> &[CurrentError<I>] {
        self.scheduler.errors(self.plan).unwrap()
    }

    pub fn register<'a>(&'a self, select: &mut Select<'a>) -> Readiness {
        self.scheduler.register(select)
    }

    pub fn tick(&mut self) -> Tick {
        let mut progressed = std::mem::take(&mut self.pending_progress);
        let mut report = std::mem::take(&mut self.pending_report);
        if let Some(tick) = self.scheduler.tick() {
            progressed |= tick.progressed();
            report.append(tick.into_report());
        }
        Tick { progressed, report }
    }

    pub fn run_until_idle(&mut self) -> Report {
        self.pending_progress = false;
        let mut report = std::mem::take(&mut self.pending_report);
        loop {
            while let Some(tick) = self.scheduler.tick() {
                report.append(tick.into_report());
            }
            if self.inline {
                break;
            }
            let waiting = {
                let mut select = Select::new();
                let readiness = self.scheduler.register(&mut select);
                if readiness.pending() {
                    let operation = select.ready();
                    assert!(readiness.contains(operation));
                    true
                } else {
                    false
                }
            };
            if !waiting {
                break;
            }
        }
        report
    }
}
