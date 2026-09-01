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

//! Stateful same-key joins over independently arriving input lanes.

use std::collections::BTreeSet;
use std::marker::PhantomData;

use ahash::HashMap;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_scheduler::{RevisionId, Value};
use zrx_store::StoreEntry;
use zrx_store::Value as StoreValue;
use zrx_store::entry::Entry;
use zrx_store::row::{
    AntiJoin as _, FullJoin as _, Join as _, LeftJoin as _, SemiJoin as _,
};

use crate::stream::Id;
use crate::stream::{Change, Key};

use super::publication::{Publication, Transition};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Projection semantics for a stored join row.
pub(in crate::stream) trait Projection<S, T>:
    Send + 'static
{
    /// Whether a replacement in every lane changes a present projection.
    const PROJECTS_ALL: bool;

    /// Projects an output from a contributing row.
    fn project(state: &S) -> Option<T>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stateful same-key join over independently arriving input lanes.
pub struct Join<I, T, S, P>
where
    I: Id,
{
    state: HashMap<Key<I>, S>,
    publication: Publication<Key<I>, u64>,
    published: BTreeSet<Key<I>>,
    marker: PhantomData<fn() -> (T, P)>,
}

// ----------------------------------------------------------------------------

/// Inner-join projection.
pub struct Inner;

// ----------------------------------------------------------------------------

/// Left-join projection.
pub struct Left;

// ----------------------------------------------------------------------------

/// Full-join projection.
pub struct Full;

// ----------------------------------------------------------------------------

/// Semi-join projection.
pub struct Semi;

// ----------------------------------------------------------------------------

/// Anti-join projection.
pub struct Anti;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T, S, P> Join<I, T, S, P>
where
    I: Id,
{
    /// Creates an empty same-key join.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: HashMap::default(),
            publication: Publication::new(),
            published: BTreeSet::new(),
            marker: PhantomData,
        }
    }
}

impl<I, T, S, P> Join<I, T, S, P>
where
    I: Id,
    T: Value,
    P: Projection<S, T>,
{
    fn mark(
        &mut self, revision: RevisionId, key: Key<I>, lane: usize,
        expected: u64,
    ) -> Option<(Key<I>, Transition<u64>)> {
        let lane = 1_u64
            .checked_shl(u32::try_from(lane).expect("join lane fits in u32"))
            .expect("join exceeds 64 input lanes");
        self.publication
            .mark_ready(
                revision,
                key,
                |lanes| *lanes |= lane,
                |lanes| *lanes == expected,
            )
            .1
    }

    fn complete_key(
        &mut self, key: &Key<I>, transition: &Transition<u64>,
        emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        let lanes = *transition.state();
        let value = self.state.get(key).and_then(P::project);
        if let Some(value) = value {
            let inserted = self.published.insert(key.clone());
            if inserted || P::PROJECTS_ALL || lanes & 1 != 0 {
                emit.insert(key.clone(), value);
            }
        } else if self.published.remove(key) {
            emit.remove(key.clone());
        }
    }

    fn complete(
        &mut self, revision: RevisionId, emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        for (key, transition) in self.publication.finish(revision) {
            self.complete_key(&key, &transition, emit);
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, S, P> Default for Join<I, T, S, P>
where
    I: Id,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

/// Implements join projections and execution for one row arity.
macro_rules! impl_joins {
    ($T1:ident => $s1:ident => $i1:tt $(, $T:ident => $s:ident => $i:tt)+) => {
        impl<$T1, $($T,)+> Projection<
            (Option<$T1>, $(Option<$T>,)+),
            ($T1, $($T,)+),
        > for Inner
        where
            $T1: Clone + StoreValue,
            $($T: Clone + StoreValue,)+
        {
            const PROJECTS_ALL: bool = true;

            #[inline]
            fn project(
                state: &(Option<$T1>, $(Option<$T>,)+),
            ) -> Option<($T1, $($T,)+)> {
                #[allow(non_snake_case)]
                let ($T1, $($T,)+) = state.join()?;
                Some(($T1.clone(), $($T.clone(),)+))
            }
        }

        impl<$T1, $($T,)+> Projection<
            (Option<$T1>, $(Option<$T>,)+),
            ($T1, $(Option<$T>,)+),
        > for Left
        where
            $T1: Clone + StoreValue,
            $($T: Clone + StoreValue,)+
        {
            const PROJECTS_ALL: bool = true;

            #[inline]
            fn project(
                state: &(Option<$T1>, $(Option<$T>,)+),
            ) -> Option<($T1, $(Option<$T>,)+)> {
                #[allow(non_snake_case)]
                let ($T1, $($T,)+) = state.left_join()?;
                Some(($T1.clone(), $($T.cloned(),)+))
            }
        }

        impl<$T1, $($T,)+> Projection<
            (Option<$T1>, $(Option<$T>,)+),
            (Option<$T1>, $(Option<$T>,)+),
        > for Full
        where
            $T1: Clone + StoreValue,
            $($T: Clone + StoreValue,)+
        {
            const PROJECTS_ALL: bool = true;

            #[inline]
            fn project(
                state: &(Option<$T1>, $(Option<$T>,)+),
            ) -> Option<(Option<$T1>, $(Option<$T>,)+)> {
                #[allow(non_snake_case)]
                let ($T1, $($T,)+) = state.full_join()?;
                Some(($T1.cloned(), $($T.cloned(),)+))
            }
        }

        impl<$T1, $($T,)+> Projection<
            (Option<$T1>, $(Option<$T>,)+),
            $T1,
        > for Semi
        where
            $T1: Clone + StoreValue,
            $($T: Clone + StoreValue,)+
        {
            const PROJECTS_ALL: bool = false;

            #[inline]
            fn project(
                state: &(Option<$T1>, $(Option<$T>,)+),
            ) -> Option<$T1> {
                state.semi_join().cloned()
            }
        }

        impl<$T1, $($T,)+> Projection<
            (Option<$T1>, $(Option<$T>,)+),
            $T1,
        > for Anti
        where
            $T1: Clone + StoreValue,
            $($T: Clone + StoreValue,)+
        {
            const PROJECTS_ALL: bool = false;

            #[inline]
            fn project(
                state: &(Option<$T1>, $(Option<$T>,)+),
            ) -> Option<$T1> {
                state.anti_join().cloned()
            }
        }

        impl<I, O, P, $T1, $($T,)+> Action<Key<I>>
            for Join<
                I,
                O,
                (Option<$T1>, $(Option<$T>,)+),
                P,
            >
        where
            I: Id,
            O: Value,
            P: Projection<(Option<$T1>, $(Option<$T>,)+), O>,
            $T1: StoreValue + Value,
            $($T: StoreValue + Value,)+
        {
            type Inputs = ($T1, $($T,)+);
            type Output = O;

            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(level = "debug", name = "join", skip_all)
            )]
            fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
                let Context {
                    revision,
                    inputs: ($s1, $($s,)+),
                    output,
                    events,
                } = context;

                impl_joins!(
                    @lanes self, output, P, revision;
                    [$i1 $(, $i)+];
                    $s1 => $i1 $(, $s => $i)+
                );
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
                        unreachable!(
                            "progress-only operator received a wake"
                        )
                    }
                });
            }
        }
    };

    (@lanes $self:ident, $output:ident, $P:ident, $revision:ident;
        [$($all:tt),+];
        $segment:ident => $index:tt
        $(, $rest:ident => $rest_index:tt)*
    ) => {
        impl_joins!(
            @lane $self, $output, $P, $revision, $segment, $index; [$($all),+]
        );
        impl_joins!(
            @lanes $self, $output, $P, $revision; [$($all),+];
            $($rest => $rest_index),*
        );
    };

    (@lanes $self:ident, $output:ident, $P:ident, $revision:ident;
        [$($all:tt),+];
    ) => {};

    (@lane $self:ident, $output:ident, $P:ident, $revision:ident,
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
                    state.$index = Some(value.into_owned());
                    if let Some((key, pending)) = $self.mark(
                        $revision,
                        key,
                        $index,
                        $(1_u64 << $all)|+
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
                    state.$index = None;
                    let empty = !($(state.$all.is_some())||+);
                    if empty {
                        entry.remove();
                    }
                    if let Some((key, pending)) = $self.mark(
                        $revision,
                        key,
                        $index,
                        $(1_u64 << $all)|+
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

impl_joins!(T1 => s1 => 0, T2 => s2 => 1);
impl_joins!(T1 => s1 => 0, T2 => s2 => 1, T3 => s3 => 2);
impl_joins!(T1 => s1 => 0, T2 => s2 => 1, T3 => s3 => 2, T4 => s4 => 3);
impl_joins!(
    T1 => s1 => 0,
    T2 => s2 => 1,
    T3 => s3 => 2,
    T4 => s4 => 3,
    T5 => s5 => 4
);
impl_joins!(
    T1 => s1 => 0,
    T2 => s2 => 1,
    T3 => s3 => 2,
    T4 => s4 => 3,
    T5 => s5 => 4,
    T6 => s6 => 5
);
impl_joins!(
    T1 => s1 => 0,
    T2 => s2 => 1,
    T3 => s3 => 2,
    T4 => s4 => 3,
    T5 => s5 => 4,
    T6 => s6 => 5,
    T7 => s7 => 6
);
impl_joins!(
    T1 => s1 => 0,
    T2 => s2 => 1,
    T3 => s3 => 2,
    T4 => s4 => 3,
    T5 => s5 => 4,
    T6 => s6 => 5,
    T7 => s7 => 6,
    T8 => s8 => 7
);
