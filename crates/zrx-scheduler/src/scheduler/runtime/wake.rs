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

//! Keyed generational wake registry and deadline projection.

use ahash::HashMap;
use std::collections::VecDeque;
use std::time::Instant;

use zrx_store::stash::{Slab, Slot};
use zrx_store::{Queue, StoreMut};

use crate::scheduler::action::{WakeKey, WakeRequest};
use crate::scheduler::{RevisionId, Settlement};

use super::progress::{Obligation, Revisions};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

pub enum Due {
    Current(usize),
    Stale,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One revision obligation retained while a wake is scheduled.
pub struct Authority {
    obligation: Obligation,
}

// ----------------------------------------------------------------------------

/// Generational identity of one scheduled runtime wake.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WakeId(Slot);

// ----------------------------------------------------------------------------

pub struct Scheduled {
    pub owner: usize,
    pub key: WakeKey,
    pub deadline: Instant,
    pub authority: Authority,
}

// ----------------------------------------------------------------------------

/// Affine release authority for one dispatched wake's terminal hold.
#[must_use = "a dispatched wake must reconcile its terminal hold"]
pub(super) struct Flight {
    owner: usize,
    revision: RevisionId,
}

// ----------------------------------------------------------------------------

/// Authoritative keyed wake records and their deadline projection.
pub struct Wakes {
    states: Slab<Scheduled>,
    current: HashMap<(usize, WakeKey), WakeId>,
    deadlines: Queue<WakeId, ()>,
    due: Vec<VecDeque<WakeId>>,
    in_flight: Vec<Option<RevisionId>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Authority {
    pub const fn new(obligation: Obligation) -> Self {
        Self { obligation }
    }

    pub(super) const fn revision(&self) -> RevisionId {
        self.obligation.revision()
    }

    pub fn fire(self) -> Obligation {
        self.obligation
    }

    pub fn clear(self, revisions: &mut Revisions) -> Option<Settlement> {
        revisions.retire(self.obligation)
    }
}

// ----------------------------------------------------------------------------

impl Wakes {
    pub fn new(nodes: usize) -> Self {
        Self {
            states: Slab::default(),
            current: HashMap::default(),
            deadlines: Queue::new(),
            due: (0..nodes).map(|_| VecDeque::new()).collect(),
            in_flight: vec![None; nodes],
        }
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.deadlines.deadline()
    }

    /// Installs a wake and returns the exact record it replaced, if any.
    pub fn install(
        &mut self, owner: usize, key: WakeKey, deadline: Instant,
        obligation: Obligation,
    ) -> (WakeId, Option<Scheduled>) {
        let id = WakeId(self.states.insert(Scheduled {
            owner,
            key,
            deadline,
            authority: Authority::new(obligation),
        }));
        self.deadlines.insert(id, ());
        self.deadlines.set_deadline(&id, deadline);
        let replaced = self.current.insert((owner, key), id).map(|prior| {
            self.deadlines.remove(&prior);
            self.remove_due(owner, prior);
            self.states
                .remove(prior.0)
                .expect("current wake remains resident")
        });
        assert!(owner < self.due.len(), "wake owner must be installed");
        (id, replaced)
    }

    /// Removes one current semantic wake.
    pub fn clear(&mut self, owner: usize, key: WakeKey) -> Option<Scheduled> {
        let id = self.current.remove(&(owner, key))?;
        self.deadlines.remove(&id);
        self.remove_due(owner, id);
        Some(
            self.states
                .remove(id.0)
                .expect("current wake remains resident"),
        )
    }

    /// Takes one due projection, retaining stale-token progress as `Some`.
    pub fn mark_due(&mut self) -> Option<Due> {
        let (id, ()) = self.deadlines.take()?;
        let owner = self.states.get(id.0).and_then(|scheduled| {
            (self.current.get(&(scheduled.owner, scheduled.key)) == Some(&id))
                .then_some(scheduled.owner)
        });
        if let Some(owner) = owner {
            self.due[owner].push_back(id);
            Some(Due::Current(owner))
        } else {
            Some(Due::Stale)
        }
    }

    pub fn has_due(&self, owner: usize) -> bool {
        !self.due[owner].is_empty()
    }

    /// Authenticates and transfers one due wake to its action invocation.
    pub fn take_due(&mut self, owner: usize) -> Option<(Scheduled, Flight)> {
        loop {
            let id = self.due[owner].pop_front()?;
            if let Some(scheduled) = self.take_current(id) {
                let revision = scheduled.authority.revision();
                assert!(
                    self.in_flight[owner].replace(revision).is_none(),
                    "wake callbacks must have exclusive node ownership"
                );
                return Some((scheduled, Flight { owner, revision }));
            }
        }
    }

    /// Releases the hold after output and replacement wakes are reconciled.
    #[allow(
        clippy::needless_pass_by_value,
        reason = "reconciliation consumes affine terminal-hold authority"
    )]
    pub fn reconcile(&mut self, flight: Flight) {
        assert_eq!(
            self.in_flight[flight.owner].take(),
            Some(flight.revision),
            "wake completion lost its terminal hold"
        );
    }

    /// Authenticates and transfers one queued wake out of the registry.
    fn take_current(&mut self, id: WakeId) -> Option<Scheduled> {
        let scheduled = self.states.get(id.0)?;
        let identity = (scheduled.owner, scheduled.key);
        if self.current.get(&identity) != Some(&id) {
            return None;
        }
        self.current.remove(&identity);
        self.states.remove(id.0)
    }

    /// Returns whether current wake authority can still re-enter this node.
    ///
    /// A branch `End` for the same revision must remain behind this authority;
    /// Dispatch transfers the hold to the callback until reconciliation has
    /// installed its output and any replacement wakes. Pruning scheduled wakes
    /// does not release a committed callback's hold.
    pub fn holds_end(&self, owner: usize, revision: RevisionId) -> bool {
        self.in_flight[owner] == Some(revision)
            || self.states.iter().any(|(slot, scheduled)| {
                scheduled.owner == owner
                    && scheduled.authority.revision() == revision
                    && self.current.get(&(owner, scheduled.key))
                        == Some(&WakeId(slot))
            })
    }

    /// Removes every resident wake owned by one revision.
    pub fn prune<F>(&mut self, revision: RevisionId, mut remove: F)
    where
        F: FnMut(Scheduled),
    {
        let ids: Vec<_> = self
            .states
            .iter()
            .filter_map(|(slot, scheduled)| {
                (scheduled.authority.revision() == revision)
                    .then_some(WakeId(slot))
            })
            .collect();
        for id in ids {
            let scheduled =
                self.states.remove(id.0).expect("wake remains resident");
            self.deadlines.remove(&id);
            self.remove_due(scheduled.owner, id);
            if self.current.get(&(scheduled.owner, scheduled.key)) == Some(&id)
            {
                self.current.remove(&(scheduled.owner, scheduled.key));
            }
            remove(scheduled);
        }
    }

    fn remove_due(&mut self, owner: usize, id: WakeId) {
        self.due[owner].retain(|current| *current != id);
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Default for Wakes {
    fn default() -> Self {
        Self::new(1)
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Retains only the last request for each action-local wake key.
pub fn deduplicate(requests: &mut Vec<WakeRequest>) {
    const LINEAR: usize = 8;

    if requests.len() <= LINEAR {
        let mut index = 0;
        while index < requests.len() {
            let key = requests[index].key();
            if requests[index + 1..]
                .iter()
                .any(|request| request.key() == key)
            {
                requests.remove(index);
            } else {
                index += 1;
            }
        }
        return;
    }

    let mut last = HashMap::default();
    last.reserve(requests.len());
    for (index, request) in requests.iter().enumerate() {
        last.insert(request.key(), index);
    }
    let mut index = 0;
    requests.retain(|request| {
        let keep = last[&request.key()] == index;
        index += 1;
        keep
    });
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Due, Wakes, deduplicate};
    use crate::scheduler::Settlement;
    use crate::scheduler::action::{Wake, WakeKey, WakeRequest};
    use crate::scheduler::plan::InputIndex;
    use crate::scheduler::runtime::progress::Revisions;

    #[derive(Clone, Copy)]
    enum Projection {
        Scheduled,
        Due,
    }

    #[test]
    fn dispatched_hold_survives_cancellation_until_reconciliation() {
        let mut revisions = Revisions::default();
        let revision = revisions.begin(InputIndex::new(0));
        let other = revisions.begin(InputIndex::new(1));
        let mut wakes = Wakes::new(2);
        wakes.install(
            0,
            WakeKey::new(1),
            Instant::now(),
            revisions.admit_many(revision, 1).unwrap().next().unwrap(),
        );
        assert!(matches!(wakes.mark_due(), Some(Due::Current(0))));
        let (scheduled, flight) = wakes.take_due(0).unwrap();
        assert!(wakes.holds_end(0, revision));
        assert!(!wakes.holds_end(0, other));
        assert!(!wakes.holds_end(1, revision));
        assert!(wakes.clear(0, WakeKey::new(1)).is_none());
        assert!(revisions.abort(revision).unwrap().is_none());
        wakes.prune(revision, |_| panic!("dispatched wake was pruned"));
        assert!(wakes.holds_end(0, revision));
        assert_eq!(
            revisions.retire(scheduled.authority.fire()),
            Some(Settlement::Aborted(revision))
        );
        wakes.reconcile(flight);
        assert!(!wakes.holds_end(0, revision));
        assert_eq!(wakes.next_deadline(), None);
    }

    #[test]
    fn request_deduplication_retains_last_values_in_source_order() {
        let now = Instant::now();
        let mut requests = vec![
            WakeRequest::new(Wake::at(WakeKey::new(2), now)),
            WakeRequest::new(Wake::clear(WakeKey::new(1))),
            WakeRequest::new(Wake::at(WakeKey::new(2), now)),
            WakeRequest::new(Wake::at(WakeKey::new(3), now)),
            WakeRequest::new(Wake::clear(WakeKey::new(1))),
        ];

        deduplicate(&mut requests);

        assert_eq!(
            requests,
            [
                WakeRequest::new(Wake::at(WakeKey::new(2), now)),
                WakeRequest::new(Wake::at(WakeKey::new(3), now)),
                WakeRequest::new(Wake::clear(WakeKey::new(1))),
            ]
        );
    }

    fn project(wakes: &mut Wakes, projection: Projection) {
        if matches!(projection, Projection::Scheduled) {
            return;
        }
        assert!(matches!(wakes.mark_due(), Some(Due::Current(0))));
        assert!(wakes.has_due(0));
    }

    #[test]
    fn replacement_invalidates_every_old_projection() {
        for projection in [Projection::Scheduled, Projection::Due] {
            let mut revisions = Revisions::default();
            let revision = revisions.begin(InputIndex::new(1));
            let mut wakes = Wakes::default();
            let deadline = Instant::now();
            let (old, replaced) = wakes.install(
                0,
                WakeKey::new(1),
                deadline,
                revisions.admit_many(revision, 1).unwrap().next().unwrap(),
            );
            assert!(replaced.is_none());
            project(&mut wakes, projection);

            let (_new, replaced) = wakes.install(
                0,
                WakeKey::new(1),
                deadline + Duration::from_secs(60),
                revisions.admit_many(revision, 1).unwrap().next().unwrap(),
            );
            let replaced = replaced.expect("current wake was replaced");
            assert!(wakes.take_current(old).is_none());
            assert!(wakes.take_due(0).is_none());
            assert!(wakes.holds_end(0, revision));

            let _ = replaced.authority.clear(&mut revisions);
            let current = wakes.clear(0, WakeKey::new(1)).unwrap();
            let _ = current.authority.clear(&mut revisions);
        }
    }

    #[test]
    fn clearing_invalidates_every_current_projection() {
        for projection in [Projection::Scheduled, Projection::Due] {
            let mut revisions = Revisions::default();
            let revision = revisions.begin(InputIndex::new(1));
            let mut wakes = Wakes::default();
            let (id, _) = wakes.install(
                0,
                WakeKey::new(1),
                Instant::now(),
                revisions.admit_many(revision, 1).unwrap().next().unwrap(),
            );
            project(&mut wakes, projection);

            let cleared = wakes.clear(0, WakeKey::new(1)).unwrap();
            assert!(wakes.take_current(id).is_none());
            assert!(wakes.take_due(0).is_none());
            let _ = cleared.authority.clear(&mut revisions);
        }
    }

    #[test]
    fn pruning_removes_scheduled_and_due_wakes_once() {
        let mut revisions = Revisions::default();
        let revision = revisions.begin(InputIndex::new(1));
        let mut wakes = Wakes::default();
        let now = Instant::now();
        let (due, _) = wakes.install(
            0,
            WakeKey::new(2),
            now,
            revisions.admit_many(revision, 1).unwrap().next().unwrap(),
        );
        let (scheduled, _) = wakes.install(
            0,
            WakeKey::new(3),
            now + Duration::from_secs(60),
            revisions.admit_many(revision, 1).unwrap().next().unwrap(),
        );
        assert!(matches!(wakes.mark_due(), Some(Due::Current(0))));
        assert_eq!(revisions.seal(revision).unwrap(), None);

        let mut removed = 0;
        let mut settlement = None;
        wakes.prune(revision, |wake| {
            removed += 1;
            settlement = wake.authority.clear(&mut revisions).or(settlement);
        });
        assert_eq!(removed, 2);
        assert_eq!(settlement, Some(Settlement::Complete(revision)));
        for id in [due, scheduled] {
            assert!(wakes.take_current(id).is_none());
        }
        assert!(wakes.take_due(0).is_none());
        assert_eq!(wakes.next_deadline(), None);
    }
}
