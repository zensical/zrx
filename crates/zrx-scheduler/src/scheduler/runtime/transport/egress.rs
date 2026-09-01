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

//! Bounded external output storage, selection, and acceptance.

use crate::scheduler::RevisionId;
use crate::scheduler::action::{InputChange, Port, Segment};
use crate::scheduler::event::Change;
use crate::scheduler::plan::{OutputBinding, OutputId};
use crate::scheduler::runtime::progress::{Obligation, Obligations};
use crate::scheduler::{Id, Value};

use super::{Data, Entry, Lane, Reservation};

// Bootstrap output buffering owned by egress transport. Later occupancy
// measurements may tune it without changing output semantics.
const BOOTSTRAP_ENTRY_CAPACITY: usize = 64;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One typed external output batch accepted from the scheduler boundary.
#[must_use]
pub struct Egress<I>
where
    I: Id,
{
    output: OutputId,
    revision: RevisionId,
    segment: Segment<I>,
}

// ----------------------------------------------------------------------------

/// Owning iterator over one typed external output batch.
pub struct EgressIter<I, V>
where
    I: Id,
{
    segment: Segment<I>,
    marker: std::marker::PhantomData<fn() -> V>,
}

// ----------------------------------------------------------------------------

struct State<I>
where
    I: Id,
{
    id: OutputId,
    source: usize,
    port: Port,
    lane: Lane<I>,
}

// ----------------------------------------------------------------------------

pub struct Egresses<I>
where
    I: Id,
{
    states: Vec<State<I>>,
    next: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Egress<I>
where
    I: Id,
{
    /// Returns the installed external output identity.
    #[must_use]
    pub const fn output(&self) -> OutputId {
        self.output
    }

    /// Returns the scheduler revision that owns this output batch.
    #[must_use]
    pub const fn revision(&self) -> RevisionId {
        self.revision
    }

    /// Returns the exact keyed value port carried by this batch.
    #[must_use]
    pub fn port(&self) -> Port {
        self.segment.port()
    }

    /// Consumes every output item through its statically known value type.
    ///
    /// # Panics
    ///
    /// Panics when `V` differs from the installed output binding.
    pub fn for_each<V>(self, callback: impl FnMut(InputChange<'_, I, V>))
    where
        V: Value,
    {
        self.segment.drain(callback);
    }

    /// Consumes this batch as owned typed changes.
    ///
    /// Unique transport moves values. Shared fan-out transport clones values
    /// only where ownership cannot be recovered.
    ///
    /// # Panics
    ///
    /// Panics when `V` differs from the installed output binding.
    #[must_use]
    pub fn into_changes<V>(self) -> EgressIter<I, V>
    where
        V: Value,
    {
        assert_eq!(self.port(), Port::of::<I, V>(), "egress port mismatch");
        EgressIter {
            segment: self.segment,
            marker: std::marker::PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I> Egresses<I>
where
    I: Id,
{
    pub fn new(bindings: Vec<OutputBinding>) -> Self {
        let states = bindings
            .into_iter()
            .map(|OutputBinding { id, source, port }| State {
                id,
                source,
                port,
                lane: Lane::new(BOOTSTRAP_ENTRY_CAPACITY),
            })
            .collect();
        Self { states, next: 0 }
    }

    pub fn take(&mut self) -> Option<(usize, Egress<I>, Obligation)> {
        let len = self.states.len();
        let index = (0..len)
            .map(|offset| (self.next + offset) % len)
            .find(|&index| self.states[index].lane.front_data().is_some())?;
        let state = &mut self.states[index];
        let Data { segment, obligation, .. } = state
            .lane
            .take_data()
            .expect("selected egress remains visible");
        debug_assert_eq!(segment.port(), state.port);
        let source = state.source;
        let output = state.id;
        self.next = if index + 1 == self.states.len() {
            0
        } else {
            index + 1
        };
        let revision = obligation.revision();
        Some((source, Egress { output, revision, segment }, obligation))
    }

    pub fn has_capacity(&self, output: usize) -> bool {
        self.states[output].lane.has_capacity(1)
    }

    pub fn reserve_prechecked(&mut self, output: usize) -> Reservation {
        self.states[output].lane.reserve_prechecked()
    }

    pub fn commit(
        &mut self, output: usize, position: Reservation,
        entry: Option<Entry<I>>,
    ) -> (usize, usize) {
        let state = &mut self.states[output];
        let released = position.commit(&mut state.lane, entry);
        (state.source, released)
    }

    pub fn prune(
        &mut self, revision: RevisionId, obligations: &mut Obligations,
    ) -> Vec<usize> {
        let mut sources = Vec::new();
        for state in &mut self.states {
            if state.lane.prune_revision(revision, obligations) != 0 {
                sources.push(state.source);
            }
        }
        sources
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, V> Iterator for EgressIter<I, V>
where
    I: Id,
    V: Value,
{
    type Item = Change<I, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.segment.pop_front_owned::<V>()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.segment.len();
        (len, Some(len))
    }
}

impl<I, V> ExactSizeIterator for EgressIter<I, V>
where
    I: Id,
    V: Value,
{
}

impl<I, V> std::iter::FusedIterator for EgressIter<I, V>
where
    I: Id,
    V: Value,
{
}
