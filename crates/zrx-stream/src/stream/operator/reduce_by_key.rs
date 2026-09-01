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

//! Revision-settled keyed reductions.

use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;

use ahash::HashMap;

use zrx_scheduler::Value;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_store::{Collection, Key as StoreKey, Value as StoreValue};

use crate::stream::Id;
use crate::stream::function::{
    Arguments, MapFn, ReduceFn, Scope as CallbackScope,
};
use crate::stream::{Change, Key, Stream};

use super::{Operator, Terminal, Tickets};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct ReduceByKey<I, T, G, F, A, B, U>
where
    I: Id,
{
    group: G,
    reduce: F,
    assignments: HashMap<Key<I>, Key<I>>,
    members: BTreeMap<Key<I>, BTreeMap<Key<I>, T>>,
    terminal: Terminal<Key<I>>,
    published: BTreeSet<Key<I>>,
    marker: PhantomData<fn(A, B) -> U>,
}

// ----------------------------------------------------------------------------

struct Reduction;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Reduces complete derived groups at their relevant revision terminals.
    ///
    /// Input changes update authoritative group state immediately. The
    /// reducer runs once per dirty group after the corresponding source
    /// revision has drained through this operator. It receives the current
    /// member store as a read-only [`Collection`], while the group key is
    /// available through the callback scope. Failed replacements preserve the
    /// prior accepted group assignment or aggregate. Reevaluation occurs only
    /// after a relevant source change.
    ///
    /// # Panics
    ///
    /// Panics if construction has already ended or operator construction
    /// reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn reduce_by_key<G, F, A, B, U>(
        &self, group: G, reduce: F,
    ) -> Stream<I, U>
    where
        G: MapFn<A, I, T, Key<I>>,
        F: ReduceFn<B, I, T, U>,
        A: Arguments,
        B: Arguments,
        U: Value,
        Key<I>: StoreKey,
        T: StoreValue,
    {
        self.subscribe_progress(ReduceByKey {
            group,
            reduce,
            assignments: HashMap::default(),
            members: BTreeMap::new(),
            terminal: Terminal::new(),
            published: BTreeSet::new(),
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------

impl<I, T, G, F, A, B, U> ReduceByKey<I, T, G, F, A, B, U>
where
    I: Id,
    T: Value,
    G: MapFn<A, I, T, Key<I>>,
    F: ReduceFn<B, I, T, U>,
    A: Arguments,
    B: Arguments,
    U: Value,
    Key<I>: StoreKey,
    T: StoreValue,
{
    fn flush(
        &mut self, groups: Tickets<Key<I>>, emit: &mut Emitter<'_, Key<I>, U>,
    ) {
        for ticket in groups {
            let result = if let Some(members) = self.members.get(ticket.key()) {
                let mut scope = CallbackScope::new(ticket.key(), emit);
                self.reduce
                    .execute(&mut scope, members as &dyn Collection<Key<I>, T>)
            } else {
                Ok(None)
            };
            match result {
                Ok(Some(value)) => {
                    emit.resolve_at::<Reduction>(ticket.key().clone());
                    let key = self.terminal.applied(ticket);
                    self.published.insert(key.clone());
                    emit.insert(key, value);
                }
                Ok(None) => {
                    emit.resolve_at::<Reduction>(ticket.key().clone());
                    let key = self.terminal.applied(ticket);
                    if self.published.remove(&key) {
                        emit.remove(key);
                    }
                }
                Err(error) => {
                    emit.reject_at::<Reduction>(ticket.key().clone(), error);
                    self.terminal.rejected(ticket);
                }
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, G, F, A, B, U> Action<Key<I>> for ReduceByKey<I, T, G, F, A, B, U>
where
    I: Id,
    T: Value,
    G: MapFn<A, I, T, Key<I>>,
    F: ReduceFn<B, I, T, U>,
    A: Arguments,
    B: Arguments,
    U: Value,
    Key<I>: StoreKey,
    T: StoreValue,
{
    type Inputs = (T,);
    type Output = U;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "reduce_by_key", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context {
            revision,
            inputs: input,
            output,
            events,
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(source, value) => {
                    let result = {
                        let mut scope = CallbackScope::new(&source, emit);
                        self.group.execute(&mut scope, value.as_ref())
                    };
                    let group = match result {
                        Ok(group) => group,
                        Err(error) => {
                            emit.reject(source, error);
                            return Ok(());
                        }
                    };
                    emit.resolve(source.clone());
                    if let Some(previous) = self.assignments.get(&source)
                        && previous != &group
                    {
                        let previous = previous.clone();
                        if let Some(members) = self.members.get_mut(&previous) {
                            members.remove(&source);
                            if members.is_empty() {
                                self.members.remove(&previous);
                            }
                        }
                        self.terminal.mark(revision, previous);
                    }
                    self.assignments.insert(source.clone(), group.clone());
                    self.members
                        .entry(group.clone())
                        .or_default()
                        .insert(source, value.into_owned());
                    self.terminal.mark(revision, group);
                }
                Change::Remove(source) => {
                    emit.resolve(source.clone());
                    let Some(group) = self.assignments.remove(&source) else {
                        return Ok(());
                    };
                    if let Some(members) = self.members.get_mut(&group) {
                        members.remove(&source);
                        if members.is_empty() {
                            self.members.remove(&group);
                        }
                    }
                    self.terminal.mark(revision, group);
                }
            }
            Ok(())
        });
        events.for_each(output, |event, emit| match event {
            Event::Progress(ProgressEvent::End) => {
                let groups = self.terminal.finish(revision);
                self.flush(groups, emit);
                Ok(())
            }
            Event::Progress(ProgressEvent::Abort) => {
                self.terminal.abort(revision);
                Ok(())
            }
            Event::Progress(ProgressEvent::Begin) => {
                self.terminal.begin(revision);
                Ok(())
            }
            Event::Wake { .. } => {
                unreachable!("progress-only operator received a wake")
            }
        });
    }
}
