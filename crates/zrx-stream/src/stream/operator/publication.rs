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

//! Keyed revision publication state.

use std::collections::BTreeMap;
use std::collections::btree_map::Entry as BTreeEntry;
use std::hash::Hash;

use ahash::HashMap;
use zrx_scheduler::RevisionId;

use super::currency::MutationCurrency;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Current transition accepted for operator-specific publication.
pub(super) struct Transition<S> {
    /// Operator-specific state accumulated within one revision.
    state: S,
    /// Latest mutation version incorporated into this transition.
    version: u64,
}

// ----------------------------------------------------------------------------

/// Revision-local transitions with bounded stale-publication currency.
pub(super) struct Publication<K, S> {
    /// Revision-local transition state, ordered by output key.
    pending: HashMap<RevisionId, BTreeMap<K, Transition<S>>>,
    /// Per-key currency retained only while a transition remains unresolved.
    currency: MutationCurrency<K>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<S> Transition<S> {
    pub(super) fn state(&self) -> &S {
        &self.state
    }

    pub(super) const fn version(&self) -> u64 {
        self.version
    }
}

// ----------------------------------------------------------------------------

impl<K, S> Publication<K, S>
where
    K: Clone + Eq + Hash + Ord,
    S: Default,
{
    pub(super) fn new() -> Self {
        Self {
            pending: HashMap::default(),
            currency: MutationCurrency::new(),
        }
    }

    pub(super) fn mark<F>(
        &mut self, revision: RevisionId, key: K, update: F,
    ) -> u64
    where
        F: FnOnce(&mut S),
    {
        self.mark_ready(revision, key, update, |_| false).0
    }

    pub(super) fn mark_ready<F, R>(
        &mut self, revision: RevisionId, key: K, update: F, ready: R,
    ) -> (u64, Option<(K, Transition<S>)>)
    where
        F: FnOnce(&mut S),
        R: FnOnce(&S) -> bool,
    {
        let keys = self.pending.entry(revision).or_default();
        // Repeated mutations in one revision update the same transition and
        // therefore do not acquire another unit of pending currency.
        let (transition, new_transition) = match keys.entry(key.clone()) {
            BTreeEntry::Occupied(entry) => (entry.into_mut(), false),
            BTreeEntry::Vacant(entry) => (
                entry.insert(Transition {
                    state: S::default(),
                    version: 0,
                }),
                true,
            ),
        };
        let version = self.currency.mark(&key, new_transition);
        update(&mut transition.state);
        transition.version = version;
        if !ready(&transition.state) {
            return (version, None);
        }
        let transition = keys.remove(&key).expect("ready key is pending");
        if keys.is_empty() {
            self.pending.remove(&revision);
        }
        (version, self.resolve(key, transition))
    }

    pub(super) fn take_ready<F>(
        &mut self, revision: RevisionId, ready: F,
    ) -> Vec<(K, Transition<S>)>
    where
        F: Fn(&S) -> bool,
    {
        let ready = self
            .pending
            .get(&revision)
            .into_iter()
            .flat_map(|keys| keys.iter())
            .filter_map(|(key, transition)| {
                ready(&transition.state).then_some(key.clone())
            })
            .collect::<Vec<_>>();
        let mut removed = Vec::with_capacity(ready.len());
        let mut empty = false;
        if let Some(keys) = self.pending.get_mut(&revision) {
            for key in ready {
                let transition =
                    keys.remove(&key).expect("ready key remains pending");
                removed.push((key, transition));
            }
            empty = keys.is_empty();
        }
        if empty {
            self.pending.remove(&revision);
        }
        let mut transitions = Vec::with_capacity(removed.len());
        for (key, transition) in removed {
            if let Some(transition) = self.resolve(key, transition) {
                transitions.push(transition);
            }
        }
        transitions
    }

    pub(super) fn finish(
        &mut self, revision: RevisionId,
    ) -> BTreeMap<K, Transition<S>> {
        let pending = self.pending.remove(&revision).unwrap_or_default();
        let mut current = BTreeMap::new();
        for (key, transition) in pending {
            if let Some((key, transition)) = self.resolve(key, transition) {
                current.insert(key, transition);
            }
        }
        current
    }

    fn resolve(
        &mut self, key: K, transition: Transition<S>,
    ) -> Option<(K, Transition<S>)> {
        // Check currency before releasing it: the final transition removes the
        // key, while an older transition must still observe a newer version.
        let current = self.currency.is_current(&key, transition.version);
        self.currency.release(&key);
        current.then_some((key, transition))
    }

    pub(super) fn abort(
        &mut self, revision: RevisionId,
    ) -> BTreeMap<K, Transition<S>> {
        let pending = self.pending.remove(&revision).unwrap_or_default();
        let mut current = BTreeMap::new();
        for (key, transition) in pending {
            if let Some((key, transition)) = self.resolve(key, transition) {
                current.insert(key, transition);
            }
        }
        current
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::stream::operator::test_revisions;

    use super::Publication;

    #[test]
    fn newer_revision_prevents_an_older_transition_from_publishing() {
        let revisions = test_revisions(2);
        let [older, newer] = revisions.as_slice() else {
            unreachable!()
        };
        let (older, newer) = (*older, *newer);
        let mut publication = Publication::<u64, Vec<u64>>::new();
        publication.mark(older, 7, |state| state.push(10));
        publication.mark(newer, 7, |state| state.push(20));

        let newer = publication.finish(newer).remove(&7).unwrap();
        assert_eq!(newer.state(), &[20]);

        assert!(publication.finish(older).is_empty());
        assert!(publication.currency.is_empty());
    }

    #[test]
    fn one_revision_coalesces_repeated_mutations_into_one_transition() {
        let revision = test_revisions(1)[0];
        let mut publication = Publication::<u64, Vec<u64>>::new();
        publication.mark(revision, 7, |state| state.push(10));
        publication.mark(revision, 7, |state| state.push(20));

        let transitions = publication.finish(revision);
        assert_eq!(transitions.len(), 1);
        let transition = transitions.get(&7).unwrap();
        assert_eq!(transition.state(), &[10, 20]);
        assert!(publication.currency.is_empty());
    }

    #[test]
    fn ready_transition_is_removed_from_terminal_pending_state() {
        let revision = test_revisions(1)[0];
        let mut publication = Publication::<u64, Vec<u64>>::new();
        let (_, ready) = publication.mark_ready(
            revision,
            7,
            |state| state.push(10),
            |state| state.len() == 1,
        );
        let (key, transition) = ready.unwrap();
        assert_eq!(key, 7);
        assert_eq!(transition.state(), &[10]);
        assert!(publication.finish(revision).is_empty());
        assert!(publication.currency.is_empty());
    }

    #[test]
    fn abort_discards_only_the_aborted_revision() {
        let revisions = test_revisions(2);
        let [aborted, retained] = revisions.as_slice() else {
            unreachable!()
        };
        let (aborted, retained) = (*aborted, *retained);
        let mut publication = Publication::<u64, Vec<u64>>::new();
        publication.mark(aborted, 1, |state| state.push(10));
        publication.mark(retained, 2, |state| state.push(20));

        let discarded = publication.abort(aborted);
        assert_eq!(discarded.len(), 1);
        assert!(publication.finish(aborted).is_empty());
        assert_eq!(
            publication.finish(retained).get(&2).unwrap().state(),
            &[20],
        );
        assert!(publication.currency.is_empty());
    }

    #[test]
    fn aborted_current_transition_suppresses_an_older_terminal() {
        let revisions = test_revisions(2);
        let [older, newer] = revisions.as_slice() else {
            unreachable!()
        };
        let mut publication = Publication::<u64, Vec<u64>>::new();
        publication.mark(*older, 7, |state| state.push(10));
        publication.mark(*newer, 7, |state| state.push(20));

        publication.abort(*newer);
        assert!(publication.finish(*older).is_empty());
        assert!(publication.currency.is_empty());
    }

    #[test]
    fn completed_key_churn_reclaims_all_mutation_currency() {
        let revision = test_revisions(1)[0];
        let mut publication = Publication::<u64, ()>::new();
        for key in 0..10_000 {
            publication.mark(revision, key, |()| {});
        }

        assert_eq!(publication.finish(revision).len(), 10_000);
        assert!(publication.pending.is_empty());
        assert!(publication.currency.is_empty());
    }
}
