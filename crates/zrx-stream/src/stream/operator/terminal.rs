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

//! Keyed revision-terminal publication state.

use std::collections::BTreeMap;
use std::hash::Hash;

use ahash::HashMap;
use zrx_scheduler::RevisionId;

use super::currency::MutationCurrency;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Current keyed work returned for terminal evaluation.
#[must_use = "terminal tickets must be applied or rejected"]
pub(super) struct Ticket<K> {
    key: Option<K>,
    resolved: bool,
}

// ----------------------------------------------------------------------------

/// Ordered current work returned by one revision terminal.
#[must_use = "terminal tickets must be applied or rejected"]
pub(super) struct Tickets<K> {
    inner: Vec<Ticket<K>>,
}

// ----------------------------------------------------------------------------

/// Keyed revision-terminal publication state.
pub(super) struct Terminal<K> {
    dirty: HashMap<RevisionId, BTreeMap<K, u64>>,
    currency: MutationCurrency<K>,
    deferred: BTreeMap<K, u64>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K> Ticket<K> {
    /// Returns the key awaiting terminal resolution.
    pub(super) const fn key(&self) -> &K {
        self.key
            .as_ref()
            .expect("unresolved ticket retains its key")
    }
}

// ----------------------------------------------------------------------------

impl<K> Terminal<K>
where
    K: Clone + Eq + Hash + Ord,
{
    pub(super) fn new() -> Self {
        Self {
            dirty: HashMap::default(),
            currency: MutationCurrency::new(),
            deferred: BTreeMap::new(),
        }
    }

    /// Re-arms state left without terminal authority by a prior abort.
    pub(super) fn begin(&mut self, revision: RevisionId) {
        if self.deferred.is_empty() {
            return;
        }
        let dirty = self.dirty.entry(revision).or_default();
        for (key, version) in std::mem::take(&mut self.deferred) {
            assert!(
                dirty.insert(key, version).is_none(),
                "deferred terminal transition was already scheduled"
            );
        }
    }

    pub(super) fn mark(&mut self, revision: RevisionId, key: K) {
        let dirty = self.dirty.entry(revision).or_default();
        let new_transition = !dirty.contains_key(&key);
        let version = self.currency.mark(&key, new_transition);
        dirty.insert(key, version);
    }

    pub(super) fn finish(&mut self, revision: RevisionId) -> Tickets<K> {
        let dirty = self.dirty.remove(&revision).unwrap_or_default();
        let mut tickets = Vec::with_capacity(dirty.len());
        for (key, version) in dirty {
            if self.currency.is_current(&key, version) {
                tickets.push(Ticket {
                    key: Some(key),
                    resolved: false,
                });
            } else {
                self.currency.release(&key);
            }
        }
        Tickets { inner: tickets }
    }

    /// Applies current terminal work and returns its key.
    pub(super) fn applied(&mut self, ticket: Ticket<K>) -> K {
        self.resolve(ticket)
    }

    /// Rejects current terminal work and returns its key.
    pub(super) fn rejected(&mut self, ticket: Ticket<K>) -> K {
        self.resolve(ticket)
    }

    pub(super) fn abort(&mut self, revision: RevisionId) {
        for (key, version) in self.dirty.remove(&revision).unwrap_or_default() {
            if self.currency.is_current(&key, version) {
                assert!(
                    self.deferred.insert(key, version).is_none(),
                    "current terminal transition was already deferred"
                );
            } else {
                self.currency.release(&key);
            }
        }
    }

    fn resolve(&mut self, mut ticket: Ticket<K>) -> K {
        let key = ticket
            .key
            .take()
            .expect("unresolved terminal ticket retains its key");
        self.currency.release(&key);
        ticket.resolved = true;
        key
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K> Drop for Ticket<K> {
    fn drop(&mut self) {
        assert!(
            self.resolved || std::thread::panicking(),
            "terminal ticket was neither applied nor rejected"
        );
    }
}

// ----------------------------------------------------------------------------

impl<K> IntoIterator for Tickets<K> {
    type Item = Ticket<K>;
    type IntoIter = std::vec::IntoIter<Ticket<K>>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::stream::operator::test_revisions;

    use super::{Terminal, Tickets};

    fn complete(
        terminal: &mut Terminal<u64>, tickets: Tickets<u64>,
    ) -> Vec<u64> {
        tickets
            .into_iter()
            .map(|ticket| terminal.applied(ticket))
            .collect()
    }

    fn finish(
        terminal: &mut Terminal<u64>, revision: zrx_scheduler::RevisionId,
    ) -> Vec<u64> {
        let tickets = terminal.finish(revision);
        complete(terminal, tickets)
    }

    #[test]
    fn newer_revision_suppresses_an_older_dirty_key() {
        let revisions = test_revisions(2);
        let [older, newer] = revisions.as_slice() else {
            unreachable!()
        };
        let (older, newer) = (*older, *newer);
        let mut terminal = Terminal::new();
        terminal.mark(older, 7_u64);
        terminal.mark(newer, 7);

        assert!(finish(&mut terminal, older).is_empty());
        assert_eq!(finish(&mut terminal, newer), [7]);
        assert!(terminal.currency.is_empty());
    }

    #[test]
    fn rejected_work_requires_a_new_relevant_transition() {
        let revisions = test_revisions(2);
        let [failed, unrelated] = revisions.as_slice() else {
            unreachable!()
        };
        let (failed, unrelated) = (*failed, *unrelated);
        let mut terminal = Terminal::new();
        terminal.mark(failed, 7_u64);
        let mut tickets = terminal.finish(failed).into_iter();
        let ticket = tickets.next().unwrap();
        assert_eq!(ticket.key(), &7);
        assert!(tickets.next().is_none());
        assert_eq!(terminal.rejected(ticket), 7);

        assert!(finish(&mut terminal, unrelated).is_empty());
        assert!(terminal.currency.is_empty());
    }

    #[test]
    fn later_mutation_creates_new_work_after_rejection() {
        let revisions = test_revisions(2);
        let [failed, newer] = revisions.as_slice() else {
            unreachable!()
        };
        let (failed, newer) = (*failed, *newer);
        let mut terminal = Terminal::new();
        terminal.mark(failed, 7_u64);
        let ticket = terminal.finish(failed).into_iter().next().unwrap();
        assert_eq!(terminal.rejected(ticket), 7);
        terminal.mark(newer, 7);

        assert_eq!(finish(&mut terminal, newer), [7]);
        assert!(terminal.currency.is_empty());
    }

    #[test]
    fn abort_discards_only_its_dirty_keys() {
        let revisions = test_revisions(2);
        let [aborted, retained] = revisions.as_slice() else {
            unreachable!()
        };
        let (aborted, retained) = (*aborted, *retained);
        let mut terminal = Terminal::new();
        terminal.mark(aborted, 1_u64);
        terminal.mark(retained, 2);

        terminal.abort(aborted);
        assert!(finish(&mut terminal, aborted).is_empty());
        terminal.begin(retained);
        assert_eq!(finish(&mut terminal, retained), [1, 2]);
        assert!(terminal.currency.is_empty());
    }

    #[test]
    fn aborted_current_transition_suppresses_an_older_terminal() {
        let revisions = test_revisions(2);
        let [older, newer] = revisions.as_slice() else {
            unreachable!()
        };
        let mut terminal = Terminal::new();
        terminal.mark(*older, 7_u64);
        terminal.mark(*newer, 7);

        terminal.abort(*newer);
        assert!(finish(&mut terminal, *older).is_empty());
        terminal.begin(*newer);
        assert_eq!(finish(&mut terminal, *newer), [7]);
        assert!(terminal.currency.is_empty());
    }

    #[test]
    fn completed_key_churn_reclaims_all_mutation_currency() {
        let revision = test_revisions(1)[0];
        let mut terminal = Terminal::new();
        for key in 0..10_000 {
            terminal.mark(revision, key);
        }

        let tickets = terminal.finish(revision);
        assert_eq!(complete(&mut terminal, tickets).len(), 10_000);
        assert!(terminal.dirty.is_empty());
        assert!(terminal.deferred.is_empty());
        assert!(terminal.currency.is_empty());
    }
}
