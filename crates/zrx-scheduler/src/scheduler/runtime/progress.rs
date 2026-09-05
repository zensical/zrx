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

//! Revision progress over affine physical-work obligations.

use std::iter;
use thiserror::Error as ThisError;

use zrx_store::stash::Slab;

use crate::scheduler::plan::InputIndex;
use crate::scheduler::{RevisionId, Settlement};

mod overlay;

pub use overlay::{ProgressBranches, ProgressIdentity, Progresses};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid external revision transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ThisError)]
pub enum Error {
    /// The identity is stale or no longer resident.
    #[error("revision {0:?} is not active")]
    Inactive(RevisionId),
    /// Root ingress has already been sealed or aborted.
    #[error("revision {0:?} is closed")]
    Closed(RevisionId),
}

// ----------------------------------------------------------------------------

/// Active revision lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    /// Root work may still be admitted.
    Open,
    /// Ingress is closed and retained work is draining.
    Sealed,
    /// Ingress is fenced and retained work is being pruned or drained.
    Aborted,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Affine authority proving that physical work may still affect a revision.
#[must_use = "dropping an obligation without retiring it leaks revision progress"]
pub struct Obligation {
    revision: RevisionId,
}

// ----------------------------------------------------------------------------

/// Homogeneous affine authority for several pieces of physical work.
///
/// The batch is represented only by its revision and remaining count. Keep it
/// counted rather than collecting identical obligation tokens into a `Vec`.
#[must_use = "dropping obligations without retiring them leaks revision progress"]
pub struct Obligations {
    revision: RevisionId,
    remaining: usize,
}

// ----------------------------------------------------------------------------

/// Progress ledger for one active scheduler revision.
struct Revision {
    input: InputIndex,
    phase: Phase,
    outstanding: usize,
}

// ----------------------------------------------------------------------------

/// Generational owner of every active scheduler revision.
pub struct Revisions {
    states: Slab<Revision>,
    open: Vec<Option<RevisionId>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Obligation {
    pub const fn revision(&self) -> RevisionId {
        self.revision
    }

    fn into_revision(self) -> RevisionId {
        self.revision
    }
}

// ----------------------------------------------------------------------------

impl Obligations {
    const fn new(revision: RevisionId, remaining: usize) -> Self {
        Self { revision, remaining }
    }

    pub const fn for_revision(revision: RevisionId) -> Self {
        Self::new(revision, 0)
    }

    pub const fn revision(&self) -> RevisionId {
        self.revision
    }

    pub fn push(&mut self, obligation: Obligation) {
        assert_eq!(
            obligation.into_revision(),
            self.revision,
            "obligation belongs to another revision"
        );
        self.remaining = self
            .remaining
            .checked_add(1)
            .expect("revision obligation batch overflowed");
    }
}

// ----------------------------------------------------------------------------

impl Revision {
    const fn new(input: InputIndex) -> Self {
        Self {
            input,
            phase: Phase::Open,
            outstanding: 0,
        }
    }

    fn seal(&mut self, id: RevisionId) -> Result<Option<Settlement>, Error> {
        if self.phase != Phase::Open {
            return Err(Error::Closed(id));
        }
        self.phase = Phase::Sealed;
        Ok(self.settle_if_idle(id))
    }

    fn abort(&mut self, id: RevisionId) -> Result<Option<Settlement>, Error> {
        if self.phase == Phase::Aborted {
            return Err(Error::Closed(id));
        }
        self.phase = Phase::Aborted;
        Ok(self.settle_if_idle(id))
    }

    #[allow(
        clippy::needless_pass_by_value,
        reason = "replacement consumes affine authority, not a borrowed count"
    )]
    fn replace_many(
        &mut self, id: RevisionId, obligations: Obligations, successors: usize,
    ) -> (Obligations, Option<Settlement>) {
        // Destructuring consumes the affine batch in O(1); do not iterate its
        // synthetic tokens merely to recover the homogeneous count.
        let Obligations {
            revision,
            remaining: predecessors,
        } = obligations;
        assert!(predecessors != 0, "no obligations to replace");
        assert_eq!(revision, id, "obligations belong to another revision");
        self.outstanding = self
            .outstanding
            .checked_sub(predecessors)
            .and_then(|count| count.checked_add(successors))
            .expect("invalid revision obligation replacement");
        let obligations = Obligations::new(id, successors);
        (obligations, self.settle_if_idle(id))
    }

    fn retire(&mut self, obligation: Obligation) -> Option<Settlement> {
        let id = obligation.into_revision();
        self.outstanding = self
            .outstanding
            .checked_sub(1)
            .expect("revision obligation retired more than once");
        self.settle_if_idle(id)
    }

    fn settle_if_idle(&self, id: RevisionId) -> Option<Settlement> {
        if self.outstanding != 0 {
            return None;
        }
        match self.phase {
            Phase::Open => None,
            Phase::Sealed => Some(Settlement::Complete(id)),
            Phase::Aborted => Some(Settlement::Aborted(id)),
        }
    }
}

// ----------------------------------------------------------------------------

impl Revisions {
    pub fn new(inputs: usize) -> Self {
        Self {
            states: Slab::default(),
            open: vec![None; inputs],
        }
    }

    pub fn open(&self, input: InputIndex) -> Option<RevisionId> {
        self.open[input.get()]
    }

    /// Opens and returns one fresh scheduler revision.
    #[must_use]
    pub fn begin(&mut self, input: InputIndex) -> RevisionId {
        assert!(self.open(input).is_none(), "source already open");
        let id = RevisionId(self.states.insert(Revision::new(input)));
        self.open[input.get()] = Some(id);
        id
    }

    /// Returns the source input attributed to an active revision.
    pub fn input(&self, id: RevisionId) -> Option<InputIndex> {
        self.states.get(id.0).map(|revision| revision.input)
    }

    /// Iterates every active revision identity.
    pub fn ids(&self) -> impl Iterator<Item = RevisionId> + '_ {
        self.states.iter().map(|(slot, _)| RevisionId(slot))
    }

    /// Returns whether no active revision remains.
    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    /// Admits several root batches or control-plane events atomically.
    ///
    /// # Errors
    ///
    /// Returns an error when the identity is inactive or ingress is closed.
    ///
    /// # Panics
    ///
    /// Panics if the revision obligation count overflows.
    pub fn admit_many(
        &mut self, id: RevisionId, count: usize,
    ) -> Result<Obligations, Error> {
        let revision = self.states.get_mut(id.0).ok_or(Error::Inactive(id))?;
        if revision.phase != Phase::Open {
            return Err(Error::Closed(id));
        }
        revision.outstanding = revision
            .outstanding
            .checked_add(count)
            .expect("revision obligation count overflowed");
        Ok(Obligations::new(id, count))
    }

    /// Closes root ingress and removes an immediately settled revision.
    ///
    /// # Errors
    ///
    /// Returns an error when the identity is inactive or already closed.
    pub fn seal(
        &mut self, id: RevisionId,
    ) -> Result<Option<Settlement>, Error> {
        let revision = self.states.get_mut(id.0).ok_or(Error::Inactive(id))?;
        let settlement = revision.seal(id)?;
        let open = &mut self.open[revision.input.get()];
        if *open == Some(id) {
            *open = None;
        }
        Ok(self.finish(id, settlement))
    }

    /// Fences root ingress and removes an immediately settled revision.
    ///
    /// # Errors
    ///
    /// Returns an error when the identity is inactive or already closed.
    pub fn abort(
        &mut self, id: RevisionId,
    ) -> Result<Option<Settlement>, Error> {
        let revision = self.states.get_mut(id.0).ok_or(Error::Inactive(id))?;
        let settlement = revision.abort(id)?;
        let open = &mut self.open[revision.input.get()];
        if *open == Some(id) {
            *open = None;
        }
        Ok(self.finish(id, settlement))
    }

    /// Replaces one obligation with zero or more successors atomically.
    ///
    /// # Panics
    ///
    /// Panics if the obligation is stale or revision accounting is invalid.
    pub fn replace(
        &mut self, obligation: Obligation, successors: usize,
    ) -> (Obligations, Option<Settlement>) {
        self.replace_many(obligation.into(), successors)
    }

    pub fn replace_many(
        &mut self, obligations: Obligations, successors: usize,
    ) -> (Obligations, Option<Settlement>) {
        let id = obligations.revision();
        let (obligations, settlement) = self
            .states
            .get_mut(id.0)
            .expect("work names an active revision")
            .replace_many(id, obligations, successors);
        let settlement = self.finish(id, settlement);
        (obligations, settlement)
    }

    /// Returns whether committed work belongs to an aborted revision.
    pub fn is_aborted(&self, id: RevisionId) -> bool {
        self.states
            .get(id.0)
            .is_some_and(|revision| revision.phase == Phase::Aborted)
    }

    /// Retires one obligation and removes a newly settled revision.
    ///
    /// # Panics
    ///
    /// Panics if the obligation is stale or was already retired.
    pub fn retire(&mut self, obligation: Obligation) -> Option<Settlement> {
        let id = obligation.revision();
        let settlement = self
            .states
            .get_mut(id.0)
            .expect("work names an active revision")
            .retire(obligation);
        self.finish(id, settlement)
    }

    fn finish(
        &mut self, id: RevisionId, settlement: Option<Settlement>,
    ) -> Option<Settlement> {
        if settlement.is_some() {
            let revision = self
                .states
                .remove(id.0)
                .expect("settled revision remains active");
            debug_assert_eq!(revision.outstanding, 0);
            debug_assert_ne!(revision.phase, Phase::Open);
        }
        settlement
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<Obligation> for Obligations {
    fn from(obligation: Obligation) -> Self {
        Self::new(obligation.into_revision(), 1)
    }
}

impl Iterator for Obligations {
    type Item = Obligation;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(Obligation { revision: self.revision })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Obligations {}

impl iter::FusedIterator for Obligations {}

#[cfg(test)]
mod tests {
    use super::{Error, Obligation, Obligations, Phase, Revisions, Settlement};
    use crate::scheduler::RevisionId;
    use crate::scheduler::plan::InputIndex;
    use crate::scheduler::runtime::wake::Authority;

    fn begin(revisions: &mut Revisions) -> RevisionId {
        revisions.begin(InputIndex::new(1))
    }

    fn phase(revisions: &Revisions, id: RevisionId) -> Option<Phase> {
        revisions.states.get(id.0).map(|revision| revision.phase)
    }

    fn outstanding(revisions: &Revisions, id: RevisionId) -> Option<usize> {
        revisions
            .states
            .get(id.0)
            .map(|revision| revision.outstanding)
    }

    fn admit(revisions: &mut Revisions, id: RevisionId) -> Obligation {
        revisions.admit_many(id, 1).unwrap().next().unwrap()
    }

    #[test]
    fn closing_an_older_revision_preserves_the_new_open_revision() {
        let mut revisions = Revisions::new(1);
        let input = InputIndex::new(0);
        let old = revisions.begin(input);
        let work = admit(&mut revisions, old);
        assert_eq!(revisions.open(input), Some(old));
        assert_eq!(revisions.seal(old), Ok(None));
        assert_eq!(revisions.open(input), None);
        let current = revisions.begin(input);
        assert_eq!(revisions.seal(old), Err(Error::Closed(old)));
        assert_eq!(revisions.open(input), Some(current));
        assert_eq!(revisions.abort(old), Ok(None));
        assert_eq!(revisions.open(input), Some(current));
        assert_eq!(revisions.retire(work), Some(Settlement::Aborted(old)));
        assert_eq!(revisions.input(old), None);
        assert_eq!(revisions.abort(old), Err(Error::Inactive(old)));
        assert_eq!(revisions.open(input), Some(current));
        assert_eq!(
            revisions.abort(current),
            Ok(Some(Settlement::Aborted(current)))
        );
        assert_eq!(revisions.open(input), None);
        assert!(revisions.is_empty());
    }

    #[test]
    #[should_panic(expected = "source already open")]
    fn revision_owner_rejects_a_second_open_revision_for_one_input() {
        let mut revisions = Revisions::new(1);
        let input = InputIndex::new(0);
        let _ = revisions.begin(input);
        let _ = revisions.begin(input);
    }

    #[test]
    fn counted_authority_converges_and_replaces_without_token_storage() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let mut inputs = Obligations::for_revision(id);
        inputs.push(admit(&mut revisions, id));
        inputs.push(admit(&mut revisions, id));
        assert_eq!(inputs.len(), 2);
        assert_eq!(revisions.seal(id), Ok(None));

        let (mut output, settlement) = revisions.replace_many(inputs, 1);
        assert_eq!(settlement, None);
        assert_eq!(output.len(), 1);
        assert_eq!(
            revisions.retire(output.next().unwrap()),
            Some(Settlement::Complete(id))
        );
    }

    #[test]
    #[should_panic(expected = "obligation belongs to another revision")]
    fn counted_authority_rejects_mixed_revisions() {
        let mut revisions = Revisions::new(8);
        let first = begin(&mut revisions);
        let second = revisions.begin(InputIndex::new(2));
        let mut obligations = Obligations::for_revision(first);
        obligations.push(admit(&mut revisions, second));
    }

    #[test]
    fn empty_revision_settles_when_sealed() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);

        assert_eq!(revisions.seal(id), Ok(Some(Settlement::Complete(id))));
        assert_eq!(phase(&revisions, id), None);
    }

    #[test]
    fn revision_owns_source_attribution_until_settlement() {
        let mut revisions = Revisions::new(8);
        let input = InputIndex::new(7);
        let id = revisions.begin(input);
        let work = admit(&mut revisions, id);

        assert_eq!(revisions.input(id), Some(input));
        assert_eq!(revisions.seal(id), Ok(None));
        assert_eq!(revisions.input(id), Some(input));
        assert_eq!(revisions.retire(work), Some(Settlement::Complete(id)));
        assert_eq!(revisions.input(id), None);
    }

    #[test]
    fn diamond_with_zero_output_settles_after_the_join_output() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let root = admit(&mut revisions, id);
        assert_eq!(revisions.seal(id), Ok(None));

        let (mut branches, settled) = revisions.replace(root, 2);
        assert_eq!(settled, None);
        let right = branches.next().unwrap();
        let left = branches.next().unwrap();

        assert_eq!(revisions.retire(left), None);
        let (mut joined, settled) = revisions.replace(right, 1);
        assert_eq!(settled, None);
        assert_eq!(outstanding(&revisions, id), Some(1));

        assert_eq!(
            revisions.retire(joined.next().unwrap()),
            Some(Settlement::Complete(id))
        );
        assert_eq!(phase(&revisions, id), None);
    }

    #[test]
    fn replication_settles_only_after_every_shard_in_any_order() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let root = admit(&mut revisions, id);
        let (mut shards, settled) = revisions.replace(root, 4);
        assert_eq!(settled, None);
        assert_eq!(revisions.seal(id), Ok(None));

        let shard_3 = shards.next().unwrap();
        let shard_2 = shards.next().unwrap();
        let shard_1 = shards.next().unwrap();
        let shard_0 = shards.next().unwrap();
        assert_eq!(revisions.retire(shard_2), None);
        assert_eq!(revisions.retire(shard_0), None);
        assert_eq!(revisions.retire(shard_3), None);
        assert_eq!(revisions.retire(shard_1), Some(Settlement::Complete(id)));
    }

    #[test]
    fn wake_transfers_authority_until_it_fires() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let root = admit(&mut revisions, id);
        let (mut work, settled) = revisions.replace(root, 2);
        assert_eq!(settled, None);
        let wake = Authority::new(work.next().unwrap());
        let immediate = work.next().unwrap();
        assert_eq!(revisions.seal(id), Ok(None));
        assert_eq!(revisions.retire(immediate), None);

        assert_eq!(
            revisions.retire(wake.fire()),
            Some(Settlement::Complete(id))
        );
    }

    #[test]
    fn clearing_a_wake_releases_its_authority() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let wake = Authority::new(admit(&mut revisions, id));
        assert_eq!(revisions.seal(id), Ok(None));

        assert_eq!(wake.clear(&mut revisions), Some(Settlement::Complete(id)));
    }

    #[test]
    fn abort_fences_ingress_but_committed_work_can_finish() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let root = admit(&mut revisions, id);
        let (mut work, settled) = revisions.replace(root, 2);
        assert_eq!(settled, None);
        let running = work.next().unwrap();
        let queued = work.next().unwrap();

        assert_eq!(revisions.abort(id), Ok(None));
        assert_eq!(revisions.retire(queued), None);
        let (mut output, settled) = revisions.replace(running, 1);
        assert_eq!(settled, None);
        assert_eq!(
            revisions.retire(output.next().unwrap()),
            Some(Settlement::Aborted(id))
        );
    }

    #[test]
    fn closed_revision_rejects_new_root_work() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let _root = admit(&mut revisions, id);
        assert_eq!(revisions.seal(id), Ok(None));
        assert!(
            matches!(revisions.admit_many(id, 1), Err(Error::Closed(actual)) if actual == id)
        );
    }

    #[test]
    fn sealed_revision_can_abort_for_generation_retirement() {
        let mut revisions = Revisions::new(8);
        let id = begin(&mut revisions);
        let work = admit(&mut revisions, id);
        assert_eq!(revisions.seal(id), Ok(None));
        assert_eq!(revisions.abort(id), Ok(None));
        assert_eq!(revisions.retire(work), Some(Settlement::Aborted(id)));
    }

    #[test]
    fn simultaneous_revisions_settle_independently() {
        let mut revisions = Revisions::new(8);
        let first_id = begin(&mut revisions);
        let second_id = revisions.begin(InputIndex::new(2));
        let first_work = admit(&mut revisions, first_id);
        let second_work = admit(&mut revisions, second_id);
        assert_eq!(revisions.seal(first_id), Ok(None));
        assert_eq!(revisions.seal(second_id), Ok(None));

        assert_eq!(
            revisions.retire(second_work),
            Some(Settlement::Complete(second_id))
        );
        assert_eq!(phase(&revisions, first_id), Some(Phase::Sealed));
        assert_eq!(
            revisions.retire(first_work),
            Some(Settlement::Complete(first_id))
        );
    }

    #[test]
    fn settled_slot_rejects_stale_identity_after_reuse() {
        let mut revisions = Revisions::new(8);
        let stale = begin(&mut revisions);
        assert_eq!(
            revisions.seal(stale),
            Ok(Some(Settlement::Complete(stale)))
        );

        let current = begin(&mut revisions);
        assert_ne!(current, stale);
        assert_eq!(revisions.seal(stale), Err(Error::Inactive(stale)));
        assert_eq!(
            revisions.seal(current),
            Ok(Some(Settlement::Complete(current)))
        );
    }
}
