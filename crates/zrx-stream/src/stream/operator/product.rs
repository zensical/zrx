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

//! Stateful Cartesian products over independently arriving input lanes.

use std::collections::BTreeSet;

use ahash::HashMap;
use anyhow::anyhow;

use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context, Emitter, InputChange};
use zrx_scheduler::{RevisionId, Value};

use crate::stream::Id;
use crate::stream::{Change, Key, Stream};

use super::Operator;
use super::publication::Publication;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Product<I, T, U>
where
    I: Id,
{
    left: HashMap<Key<I>, T>,
    right: HashMap<Key<I>, U>,
    owners: HashMap<Key<I>, Owner<I>>,
    publication: Publication<Key<I>, u8>,
    published: BTreeSet<Key<I>>,
}

// ----------------------------------------------------------------------------

struct LeftEndpoint;

// ----------------------------------------------------------------------------

struct RightEndpoint;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Forms the Cartesian product with another stream.
    ///
    /// Output keys concatenate the complete left and right keys. Ambiguous
    /// concatenations are reported as ordinary action failures.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::{run, Change};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let names = scope.iter([(1, "one"), (2, "two")]);
    ///     let values = scope.iter([(3, 30_u64)]);
    ///     names.product(&values)
    /// })?
    /// .collect();
    ///
    /// let values: Vec<_> = changes?
    ///     .into_iter()
    ///     .filter_map(|change| match change {
    ///         Change::Insert(_, value) => Some(value),
    ///         Change::Remove(_) => None,
    ///     })
    ///     .collect();
    /// assert_eq!(values.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the streams belong to different workflows, construction has
    /// already ended, or operator construction reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn product<U>(&self, stream: &Stream<I, U>) -> Stream<I, (T, U)>
    where
        U: Value,
    {
        (self.clone(), stream.clone()).subscribe_progress(Product::new())
    }
}

// ----------------------------------------------------------------------------

impl<I, T, U> Product<I, T, U>
where
    I: Id,
    T: Value,
    U: Value,
{
    fn mark(
        publication: &mut Publication<Key<I>, u8>, revision: RevisionId,
        key: Key<I>, lane: u8,
    ) {
        publication.mark(revision, key, |lanes| *lanes |= lane);
    }

    fn update_left(
        &mut self, change: InputChange<'_, Key<I>, T>, revision: RevisionId,
    ) -> zrx_scheduler::action::Result {
        match change {
            Change::Insert(key, value) => {
                for right_key in self.right.keys() {
                    let output_key = key.concat(right_key);
                    let owner = (key.clone(), right_key.clone());
                    if self
                        .owners
                        .get(&output_key)
                        .is_some_and(|existing| existing != &owner)
                    {
                        return Err(
                            anyhow!("product output key is ambiguous").into()
                        );
                    }
                }

                self.left.insert(key.clone(), value.into_owned());
                for right_key in self.right.keys() {
                    let output_key = key.concat(right_key);
                    self.owners.insert(
                        output_key.clone(),
                        (key.clone(), right_key.clone()),
                    );
                    Self::mark(
                        &mut self.publication,
                        revision,
                        output_key,
                        0b01,
                    );
                }
            }
            Change::Remove(key) => {
                if self.left.remove(&key).is_none() {
                    return Ok(());
                }
                for right_key in self.right.keys() {
                    let output_key = key.concat(right_key);
                    let owner = (key.clone(), right_key.clone());
                    if self.owners.get(&output_key) == Some(&owner) {
                        self.owners.remove(&output_key);
                        Self::mark(
                            &mut self.publication,
                            revision,
                            output_key,
                            0b01,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn update_right(
        &mut self, change: InputChange<'_, Key<I>, U>, revision: RevisionId,
    ) -> zrx_scheduler::action::Result {
        match change {
            Change::Insert(key, value) => {
                for left_key in self.left.keys() {
                    let output_key = left_key.concat(&key);
                    let owner = (left_key.clone(), key.clone());
                    if self
                        .owners
                        .get(&output_key)
                        .is_some_and(|existing| existing != &owner)
                    {
                        return Err(
                            anyhow!("product output key is ambiguous").into()
                        );
                    }
                }

                self.right.insert(key.clone(), value.into_owned());
                for left_key in self.left.keys() {
                    let output_key = left_key.concat(&key);
                    self.owners.insert(
                        output_key.clone(),
                        (left_key.clone(), key.clone()),
                    );
                    Self::mark(
                        &mut self.publication,
                        revision,
                        output_key,
                        0b10,
                    );
                }
            }
            Change::Remove(key) => {
                if self.right.remove(&key).is_none() {
                    return Ok(());
                }
                for left_key in self.left.keys() {
                    let output_key = left_key.concat(&key);
                    let owner = (left_key.clone(), key.clone());
                    if self.owners.get(&output_key) == Some(&owner) {
                        self.owners.remove(&output_key);
                        Self::mark(
                            &mut self.publication,
                            revision,
                            output_key,
                            0b10,
                        );
                    }
                }
            }
        }
        Ok(())
    }

    fn new() -> Self {
        Self {
            left: HashMap::default(),
            right: HashMap::default(),
            owners: HashMap::default(),
            publication: Publication::new(),
            published: BTreeSet::new(),
        }
    }
}

impl<I, T, U> Product<I, T, U>
where
    I: Id,
    T: Value,
    U: Value,
{
    fn complete_key(
        &mut self, key: &Key<I>, emit: &mut Emitter<'_, Key<I>, (T, U)>,
    ) {
        if let Some((left, right)) = self.owners.get(key) {
            let left = self.left.get(left).expect("product owner lost left");
            let right =
                self.right.get(right).expect("product owner lost right");
            self.published.insert(key.clone());
            emit.insert(key.clone(), (left.clone(), right.clone()));
        } else if self.published.remove(key) {
            emit.remove(key.clone());
        }
    }

    fn complete_ready(
        &mut self, revision: RevisionId, emit: &mut Emitter<'_, Key<I>, (T, U)>,
    ) {
        for (key, _) in self
            .publication
            .take_ready(revision, |lanes| *lanes == 0b11)
        {
            self.complete_key(&key, emit);
        }
    }

    fn complete(
        &mut self, revision: RevisionId, emit: &mut Emitter<'_, Key<I>, (T, U)>,
    ) {
        for (key, _) in self.publication.finish(revision) {
            self.complete_key(&key, emit);
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, U> Action<Key<I>> for Product<I, T, U>
where
    I: Id,
    T: Value,
    U: Value,
{
    type Inputs = (T, U);
    type Output = (T, U);

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "product", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context {
            revision,
            inputs: (left, right),
            output,
            events,
        } = context;

        left.for_each(output, |change, emit| {
            let key = match &change {
                Change::Insert(key, _) | Change::Remove(key) => key.clone(),
            };
            let result = self.update_left(change, revision);
            match result {
                Ok(()) => {
                    emit.resolve_at::<LeftEndpoint>(key);
                    self.complete_ready(revision, emit);
                }
                Err(error) => {
                    emit.reject_at::<LeftEndpoint>(key, error);
                }
            }
            Ok(())
        });

        right.for_each(output, |change, emit| {
            let key = match &change {
                Change::Insert(key, _) | Change::Remove(key) => key.clone(),
            };
            let result = self.update_right(change, revision);
            match result {
                Ok(()) => {
                    emit.resolve_at::<RightEndpoint>(key);
                    self.complete_ready(revision, emit);
                }
                Err(error) => {
                    emit.reject_at::<RightEndpoint>(key, error);
                }
            }
            Ok(())
        });

        events.for_each(output, |event, emit| match event {
            Event::Progress(ProgressEvent::End) => {
                self.complete(revision, emit);
                Ok(())
            }
            Event::Progress(ProgressEvent::Abort) => {
                self.publication.abort(revision);
                Ok(())
            }
            Event::Progress(ProgressEvent::Begin) => Ok(()),
            Event::Wake { .. } => {
                unreachable!("progress-only operator received a wake")
            }
        });
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type Owner<I> = (Key<I>, Key<I>);
