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

//! Ordered range windows.

use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::ops::Bound::{Excluded, Unbounded};

use zrx_scheduler::Value;
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_store::Value as StoreValue;

use crate::stream::Id;
use crate::stream::{Change, Key, Stream};

use super::Operator;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

trait RangePolicy {
    fn left(length: usize, size: usize) -> usize;

    fn visible(side: Side) -> bool;
}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct RangeWindow<I, T, P>
where
    I: Id,
{
    size: usize,
    state: BTreeMap<Key<I>, T>,
    split: Option<Key<I>>,
    left: usize,
    candidates: Vec<(Key<I>, Option<Side>)>,
    marker: PhantomData<P>,
}

// ----------------------------------------------------------------------------

struct Take;

// ----------------------------------------------------------------------------

struct TakeLast;

// ----------------------------------------------------------------------------

struct Skip;

// ----------------------------------------------------------------------------

struct SkipLast;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: StoreValue + Value,
{
    /// Retains the first `count` items in key order.
    ///
    /// The window is differential: insertions and removals at its boundary
    /// retract and admit only the affected keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::{run, Change};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     scope.iter([(3, "three"), (1, "one"), (2, "two")]).take(2)
    /// })?
    /// .collect();
    /// let values: Vec<_> = changes?
    ///     .into_iter()
    ///     .filter_map(|change| match change {
    ///         Change::Insert(_, value) => Some(value),
    ///         Change::Remove(_) => None,
    ///     })
    ///     .collect();
    /// assert!(values.contains(&"one"));
    /// assert!(values.contains(&"two"));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn take(&self, count: usize) -> Stream<I, T> {
        self.subscribe(RangeWindow::<I, T, Take>::new(count))
    }

    /// Retains the last `count` items in key order.
    #[inline]
    #[must_use]
    pub fn take_last(&self, count: usize) -> Stream<I, T> {
        self.subscribe(RangeWindow::<I, T, TakeLast>::new(count))
    }

    /// Skips the first `count` items in key order.
    #[inline]
    #[must_use]
    pub fn skip(&self, count: usize) -> Stream<I, T> {
        self.subscribe(RangeWindow::<I, T, Skip>::new(count))
    }

    /// Skips the last `count` items in key order.
    #[inline]
    #[must_use]
    pub fn skip_last(&self, count: usize) -> Stream<I, T> {
        self.subscribe(RangeWindow::<I, T, SkipLast>::new(count))
    }
}

// ----------------------------------------------------------------------------

impl<I, T, P> RangeWindow<I, T, P>
where
    I: Id,
    T: StoreValue + Value,
    P: RangePolicy,
{
    fn new(size: usize) -> Self {
        Self {
            size,
            state: BTreeMap::new(),
            split: None,
            left: 0,
            candidates: Vec::with_capacity(3),
            marker: PhantomData,
        }
    }

    fn insert(&mut self, key: Key<I>, value: T) -> bool {
        if let Some(previous) = self.state.get_mut(&key) {
            if previous == &value {
                return false;
            }
            *previous = value;
            return true;
        }

        if self.split.as_ref().is_some_and(|split| &key <= split) {
            self.left += 1;
        }
        self.state.insert(key, value);
        self.rebalance();
        true
    }

    fn remove(&mut self, key: &Key<I>) -> bool {
        let side = self.side(key);
        if self.state.remove(key).is_none() {
            return false;
        }
        if matches!(side, Some(Side::Left)) {
            self.left -= 1;
            if self.split.as_ref() == Some(key) {
                self.split = self
                    .state
                    .range(..key)
                    .next_back()
                    .map(|(key, _)| key.clone());
            }
        }
        self.rebalance();
        true
    }

    fn candidate(&mut self, key: Key<I>, side: Option<Side>) {
        if !self.candidates.iter().any(|(item, _)| item == &key) {
            self.candidates.push((key, side));
        }
    }

    fn rebalance(&mut self) {
        let target = P::left(self.state.len(), self.size);
        while self.left > target {
            let key = self.split.clone().expect("left is non-empty");
            self.candidate(key.clone(), Some(Side::Left));
            self.split = self
                .state
                .range(..&key)
                .next_back()
                .map(|(key, _)| key.clone());
            self.left -= 1;
        }
        while self.left < target {
            let key = match &self.split {
                Some(split) => self
                    .state
                    .range((Excluded(split), Unbounded))
                    .next()
                    .map(|(key, _)| key.clone()),
                None => {
                    self.state.first_key_value().map(|(key, _)| key.clone())
                }
            }
            .expect("right is non-empty");
            self.candidate(key.clone(), Some(Side::Right));
            self.split = Some(key);
            self.left += 1;
        }
    }

    fn side(&self, key: &Key<I>) -> Option<Side> {
        self.state.contains_key(key).then(|| {
            if self.split.as_ref().is_some_and(|split| key <= split) {
                Side::Left
            } else {
                Side::Right
            }
        })
    }

    fn get(&self, key: &Key<I>) -> Option<&T> {
        self.state.get(key)
    }

    fn reconcile(
        &mut self, dirty: Option<&Key<I>>, emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        for (key, previous) in &self.candidates {
            let previous = previous.is_some_and(P::visible);
            let next = self.side(key).is_some_and(P::visible);
            if previous && !next {
                emit.remove(key.clone());
            }
        }
        while let Some((key, previous)) = self.candidates.pop() {
            let previous = previous.is_some_and(P::visible);
            let next = self.side(&key).is_some_and(P::visible);
            if next && (!previous || dirty == Some(&key)) {
                let value = self.get(&key).expect("partitioned key").clone();
                emit.insert(key, value);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl RangePolicy for Take {
    fn left(length: usize, size: usize) -> usize {
        length.min(size)
    }

    fn visible(side: Side) -> bool {
        matches!(side, Side::Left)
    }
}

// ----------------------------------------------------------------------------

impl RangePolicy for TakeLast {
    fn left(length: usize, size: usize) -> usize {
        length.saturating_sub(size)
    }

    fn visible(side: Side) -> bool {
        matches!(side, Side::Right)
    }
}

// ----------------------------------------------------------------------------

impl RangePolicy for Skip {
    fn left(length: usize, size: usize) -> usize {
        length.min(size)
    }

    fn visible(side: Side) -> bool {
        matches!(side, Side::Right)
    }
}

// ----------------------------------------------------------------------------

impl RangePolicy for SkipLast {
    fn left(length: usize, size: usize) -> usize {
        length.saturating_sub(size)
    }

    fn visible(side: Side) -> bool {
        matches!(side, Side::Left)
    }
}

// ----------------------------------------------------------------------------

impl<I, T, P> Action<Key<I>> for RangeWindow<I, T, P>
where
    I: Id,
    T: StoreValue + Value,
    P: RangePolicy + Send + 'static,
{
    type Inputs = (T,);
    type Output = T;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "window", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            let key = match change {
                Change::Insert(key, value) => {
                    self.candidate(key.clone(), self.side(&key));
                    if !self.insert(key.clone(), value.into_owned()) {
                        self.candidates.clear();
                        return Ok(());
                    }
                    key
                }
                Change::Remove(key) => {
                    self.candidate(key.clone(), self.side(&key));
                    if !self.remove(&key) {
                        self.candidates.clear();
                        return Ok(());
                    }
                    key
                }
            };
            self.reconcile(Some(&key), emit);
            Ok(())
        });
    }
}
