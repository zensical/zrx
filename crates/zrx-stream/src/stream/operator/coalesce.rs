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

//! Stateful priority selection over independently arriving input lanes.

use std::collections::BTreeSet;
use std::marker::PhantomData;

use ahash::HashMap;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_scheduler::{RevisionId, Value};
use zrx_store::StoreEntry;
use zrx_store::Value as StoreValue;
use zrx_store::entry::Entry;

use crate::stream::Id;
use crate::stream::{Change, Key};

use super::publication::{Publication, Transition};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stateful priority selection over independently arriving input lanes.
pub struct Coalesce<I, T, S>
where
    I: Id,
{
    state: HashMap<Key<I>, S>,
    publication: Publication<Key<I>, u64>,
    dirty_versions: HashMap<Key<I>, u64>,
    published: BTreeSet<Key<I>>,
    marker: PhantomData<fn() -> T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T, S> Coalesce<I, T, S>
where
    I: Id,
{
    /// Creates an empty priority selection.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: HashMap::default(),
            publication: Publication::new(),
            dirty_versions: HashMap::default(),
            published: BTreeSet::new(),
            marker: PhantomData,
        }
    }
}

impl<I, T, S> Coalesce<I, T, S>
where
    I: Id,
    T: Value,
    for<'a> S: zrx_store::row::Coalesce<'a, Output = &'a T>,
{
    fn mark(
        &mut self, revision: RevisionId, key: Key<I>, lane: usize,
        expected: u64, changed: bool,
    ) -> Option<(Key<I>, Transition<u64>)> {
        let lane = 1_u64
            .checked_shl(
                u32::try_from(lane).expect("coalesce lane fits in u32"),
            )
            .expect("coalesce exceeds 64 input lanes");
        let (version, ready) = self.publication.mark_ready(
            revision,
            key.clone(),
            |lanes| *lanes |= lane,
            |lanes| *lanes == expected,
        );
        if changed {
            self.dirty_versions.insert(key, version);
        }
        ready
    }

    fn complete_key(
        &mut self, key: &Key<I>, transition: &Transition<u64>,
        emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        if !self.take_dirty(key, transition.version()) {
            return;
        }
        if let Some(value) = self
            .state
            .get(key)
            .and_then(zrx_store::row::Coalesce::coalesce)
        {
            self.published.insert(key.clone());
            emit.insert(key.clone(), value.clone());
        } else if self.published.remove(key) {
            emit.remove(key.clone());
        }
    }

    fn take_dirty(&mut self, key: &Key<I>, version: u64) -> bool {
        if self
            .dirty_versions
            .get(key)
            .is_none_or(|dirty| *dirty > version)
        {
            return false;
        }
        self.dirty_versions.remove(key);
        true
    }

    fn complete(
        &mut self, revision: RevisionId, emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        for (key, transition) in self.publication.finish(revision) {
            self.complete_key(&key, &transition, emit);
        }
    }

    fn abort(&mut self, revision: RevisionId) {
        for (key, transition) in self.publication.abort(revision) {
            let current_absent = self
                .state
                .get(&key)
                .and_then(zrx_store::row::Coalesce::coalesce)
                .is_none();
            if current_absent
                && !self.published.contains(&key)
                && self.dirty_versions.get(&key) == Some(&transition.version())
            {
                self.dirty_versions.remove(&key);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, S> Default for Coalesce<I, T, S>
where
    I: Id,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

/// Expands one homogeneous input or state slot.
macro_rules! slot {
    ($N:ident, $T:ident) => {
        $T
    };
}

/// Returns the first occupied lane in tuple order.
macro_rules! active {
    ($state:ident; $first:tt $(, $rest:tt)+) => {
        if $state.$first.is_some() {
            Some($first)
        } $(else if $state.$rest.is_some() {
            Some($rest)
        })+ else {
            None
        }
    };
}

/// Returns whether the externally visible priority value changed.
macro_rules! changed {
    ($before:ident, $after:ident, $index:tt) => {
        match ($before, $after) {
            (None, Some(_)) | (Some(_), None) => true,
            (Some(before), Some(after))
                if before != after || after == $index =>
            {
                true
            }
            _ => false,
        }
    };
}

/// Implements priority selection for one tuple arity.
macro_rules! impl_coalesce {
    ($s1:ident => $i1:tt $(, $s:ident => $i:tt)+) => {
        impl<I, T> Action<Key<I>>
            for Coalesce<
                I,
                T,
                (Option<T>, $(Option<slot!($s, T)>,)+),
            >
        where
            I: Id,
            T: StoreValue + Value,
        {
            type Inputs = (T, $(slot!($s, T),)+);
            type Output = T;

            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", name = "coalesce", skip_all
                )
            )]
            fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
                let Context {
                    revision,
                    inputs: ($s1, $($s,)+),
                    output,
                    events,
                } = context;

                impl_coalesce!(
                    @lanes self, output, revision;
                    [$i1 $(, $i)+];
                    $s1 => $i1 $(, $s => $i)+
                );
                events.for_each(output, |event, emit| match event {
                    Event::Progress(ProgressEvent::End) => {
                        self.complete(revision, emit);
                        Ok(())
                    }
                    Event::Progress(ProgressEvent::Abort) => {
                        self.abort(revision);
                        Ok(())
                    }
                    Event::Progress(ProgressEvent::Begin) => Ok(()),
                    Event::Wake { .. } => unreachable!(
                        "progress-only operator received a wake"
                    ),
                });
            }
        }
    };

    (@lanes $self:ident, $output:ident, $revision:ident;
        [$($all:tt),+];
        $segment:ident => $index:tt
        $(, $rest:ident => $rest_index:tt)*
    ) => {
        impl_coalesce!(
            @lane $self, $output, $revision, $segment, $index; [$($all),+]
        );
        impl_coalesce!(
            @lanes $self, $output, $revision; [$($all),+];
            $($rest => $rest_index),*
        );
    };

    (@lanes $self:ident, $output:ident, $revision:ident; [$($all:tt),+];) => {};

    (@lane $self:ident, $output:ident, $revision:ident,
        $segment:ident, $index:tt; [$($all:tt),+]
    ) => {
        $segment.for_each($output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    let state = StoreEntry::entry(
                        &mut $self.state,
                        key.clone(),
                    )
                    .or_default();
                    let before = active!(state; $($all),+);
                    state.$index = Some(value.into_owned());
                    let after = active!(state; $($all),+);
                    if let Some((key, pending)) = $self.mark(
                        $revision,
                        key,
                        $index,
                        $(1_u64 << $all)|+,
                        changed!(before, after, $index),
                    ) {
                        $self.complete_key(&key, &pending, emit);
                    }
                }
                Change::Remove(key) => {
                    let Entry::Occupied(mut entry) =
                        StoreEntry::entry(&mut $self.state, key.clone())
                    else {
                        return Ok(());
                    };
                    let state = entry.get_mut();
                    let before = active!(state; $($all),+);
                    state.$index = None;
                    let after = active!(state; $($all),+);
                    let changed = changed!(before, after, $index);
                    if after.is_none() {
                        entry.remove();
                    }
                    if let Some((key, pending)) = $self.mark(
                        $revision,
                        key,
                        $index,
                        $(1_u64 << $all)|+,
                        changed,
                    ) {
                        $self.complete_key(&key, &pending, emit);
                    }
                }
            }
            Ok(())
        });
    };
}

// ----------------------------------------------------------------------------

impl_coalesce!(s1 => 0, s2 => 1);
impl_coalesce!(s1 => 0, s2 => 1, s3 => 2);
impl_coalesce!(s1 => 0, s2 => 1, s3 => 2, s4 => 3);
impl_coalesce!(s1 => 0, s2 => 1, s3 => 2, s4 => 3, s5 => 4);
impl_coalesce!(s1 => 0, s2 => 1, s3 => 2, s4 => 3, s5 => 4, s6 => 5);
impl_coalesce!(
    s1 => 0,
    s2 => 1,
    s3 => 2,
    s4 => 3,
    s5 => 4,
    s6 => 5,
    s7 => 6
);
impl_coalesce!(
    s1 => 0,
    s2 => 1,
    s3 => 2,
    s4 => 3,
    s5 => 4,
    s6 => 5,
    s7 => 6,
    s8 => 7
);

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::stream::Key;
    use crate::stream::operator::test_revisions;

    use super::Coalesce;

    type Binary = Coalesce<u64, String, (Option<String>, Option<String>)>;

    #[test]
    fn successful_reconciliation_reclaims_dirty_currency() {
        let key = Key::from(7_u64);
        let mut coalesce = Binary::new();
        coalesce.dirty_versions.insert(key.clone(), 2);

        assert!(!coalesce.take_dirty(&key, 1));
        assert!(coalesce.take_dirty(&key, 2));
        assert!(coalesce.dirty_versions.is_empty());
    }

    #[test]
    fn abort_reclaims_a_fresh_absent_dirty_key() {
        let revision = test_revisions(1)[0];
        let key = Key::from(7_u64);
        let mut coalesce = Binary::new();
        let version = coalesce.publication.mark(revision, key.clone(), |_| {});
        coalesce.dirty_versions.insert(key, version);

        coalesce.abort(revision);

        assert!(coalesce.dirty_versions.is_empty());
    }

    #[test]
    fn abort_preserves_a_dirty_key_that_still_requires_repair() {
        let revision = test_revisions(1)[0];
        let key = Key::from(7_u64);
        let mut coalesce = Binary::new();
        coalesce.published.insert(key.clone());
        let version = coalesce.publication.mark(revision, key.clone(), |_| {});
        coalesce.dirty_versions.insert(key.clone(), version);

        coalesce.abort(revision);

        assert_eq!(coalesce.dirty_versions.get(&key), Some(&version));
    }

    #[test]
    fn abort_does_not_reclaim_a_newer_dirty_version() {
        let revisions = test_revisions(2);
        let [older, newer] = revisions.as_slice() else {
            unreachable!()
        };
        let key = Key::from(7_u64);
        let mut coalesce = Binary::new();
        coalesce.publication.mark(*older, key.clone(), |_| {});
        let newer = coalesce.publication.mark(*newer, key.clone(), |_| {});
        coalesce.dirty_versions.insert(key.clone(), newer);

        coalesce.abort(*older);

        assert_eq!(coalesce.dirty_versions.get(&key), Some(&newer));
    }
}
