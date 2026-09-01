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

//! Revision-settled global reductions.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use ahash::HashMap;

use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_scheduler::{RevisionId, Value};
use zrx_store::{Collection, Key as StoreKey, Value as StoreValue};

use crate::stream::Id;
use crate::stream::function::{Arguments, ReduceFn, Scope as CallbackScope};
use crate::stream::{Change, Key, Signal, Stream};

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Reduce<I, T, F, A, U>
where
    I: Id,
{
    function: F,
    key: Key<I>,
    members: BTreeMap<Key<I>, T>,
    dirty: HashMap<RevisionId, u64>,
    version: u64,
    deferred: bool,
    initialized: bool,
    published: bool,
    marker: PhantomData<fn(A) -> U>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Reduces the complete relation at relevant revision terminals.
    ///
    /// The reducer receives the current authoritative relation as a read-only
    /// [`Collection`]. The first current completed revision evaluates even an
    /// empty relation. Returning `Some` publishes the scalar value and
    /// returning `None` makes the signal absent.
    ///
    /// # Panics
    ///
    /// Panics if construction has already ended or operator construction
    /// reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn reduce<F, A, U>(&self, function: F) -> Signal<I, U>
    where
        F: ReduceFn<A, I, T, U>,
        A: Arguments,
        U: Value,
        Key<I>: StoreKey,
        T: StoreValue,
    {
        Signal::new(self.subscribe_progress(Reduce {
            function,
            key: std::iter::empty().collect(),
            members: BTreeMap::new(),
            dirty: HashMap::default(),
            version: 0,
            deferred: false,
            initialized: false,
            published: false,
            marker: PhantomData,
        }))
    }
}

// ----------------------------------------------------------------------------

impl<I, T, F, A, U> Reduce<I, T, F, A, U>
where
    I: Id,
    T: Value,
    F: ReduceFn<A, I, T, U>,
    A: Arguments,
    U: Value,
    Key<I>: StoreKey,
    T: StoreValue,
{
    fn mark(&mut self, revision: RevisionId) {
        self.version = self
            .version
            .checked_add(1)
            .expect("reduce mutation version exhausted");
        self.dirty.insert(revision, self.version);
        self.deferred = false;
    }

    fn flush(
        &mut self, revision: RevisionId, emit: &mut Emitter<'_, Key<I>, U>,
    ) {
        if self
            .dirty
            .remove(&revision)
            .is_none_or(|version| version != self.version)
        {
            return;
        }
        self.initialized = true;
        let mut scope = CallbackScope::new(&self.key, emit);
        match self
            .function
            .execute(&mut scope, &self.members as &dyn Collection<Key<I>, T>)
        {
            Ok(Some(value)) => {
                emit.resolve(self.key.clone());
                self.published = true;
                emit.insert(self.key.clone(), value);
            }
            Ok(None) => {
                emit.resolve(self.key.clone());
                if self.published {
                    self.published = false;
                    emit.remove(self.key.clone());
                }
            }
            Err(error) => {
                emit.reject(self.key.clone(), error);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A, U> Action<Key<I>> for Reduce<I, T, F, A, U>
where
    I: Id,
    T: Value,
    F: ReduceFn<A, I, T, U>,
    A: Arguments,
    U: Value,
    Key<I>: StoreKey,
    T: StoreValue,
{
    type Inputs = (T,);
    type Output = U;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "reduce", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context {
            revision,
            inputs: input,
            output,
            events,
        } = context;
        input.for_each(output, |change, _| {
            match change {
                Change::Insert(key, value) => {
                    self.members.insert(key, value.into_owned());
                    self.mark(revision);
                }
                Change::Remove(key) => {
                    if self.members.remove(&key).is_some() {
                        self.mark(revision);
                    }
                }
            }
            Ok(())
        });
        events.for_each(output, |event, emit| match event {
            Event::Progress(ProgressEvent::Begin) => {
                if !self.initialized || self.deferred {
                    self.dirty.insert(revision, self.version);
                    self.deferred = false;
                }
                Ok(())
            }
            Event::Progress(ProgressEvent::End) => {
                self.flush(revision, emit);
                Ok(())
            }
            Event::Progress(ProgressEvent::Abort) => {
                if self
                    .dirty
                    .remove(&revision)
                    .is_some_and(|version| version == self.version)
                {
                    self.deferred = true;
                }
                Ok(())
            }
            Event::Wake { .. } => {
                unreachable!("progress-only operator received a wake")
            }
        });
    }
}
