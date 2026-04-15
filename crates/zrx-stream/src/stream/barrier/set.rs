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

//! Barrier set.

use zrx_scheduler::action::options::Event;
use zrx_scheduler::{Id, Scope};
use zrx_store::stash::Items;
use zrx_store::Stash;

use super::advance::Advance;
use super::lifecycle::Lifecycle;
use super::Barrier;

mod drain;

pub use drain::Drain;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Barrier set.
///
/// Maintains a bipartite graph between scopes and barriers, routing lifecycle
/// events to barriers and emitting advances when barriers are fulfilled.
#[derive(Clone, Debug)]
pub struct Barriers<I> {
    /// Inner set of barriers.
    inner: Stash<Scope<I>, Barrier<I>>,
    /// All known scopes.
    scopes: Stash<Scope<I>, Items>,
    /// Global lifecycle.
    lifecycle: Lifecycle,
    /// Barrier indices that are fulfilled and pending drain.
    fulfilled: Items,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Barriers<I>
where
    I: Id,
{
    /// Creates a barrier set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Stash::new(),
            scopes: Stash::new(),
            lifecycle: Lifecycle::default(),
            fulfilled: Items::new(),
        }
    }

    /// Inserts a barrier into the barrier set.
    pub fn insert<B>(&mut self, scope: Scope<I>, barrier: B)
    where
        B: Into<Barrier<I>>,
    {
        let b = self.inner.insert(scope, barrier.into());
        let barrier = &mut self.inner[b];
        for (s, (scope, items)) in self.scopes.slots_mut() {
            if barrier.contains(scope) {
                barrier.items.insert(s);
                items.insert(b);
            }
        }

        // The barrier may already be fulfilled if all matching scopes were
        // completed before it was registered
        if self.inner[b].is_complete(&self.lifecycle) {
            self.fulfilled.insert(b);
        }
    }

    /// Removes a barrier from the barrier set.
    pub fn remove(&mut self, scope: &Scope<I>) -> Option<Barrier<I>> {
        let b = self.inner.get(scope)?;
        let (_, mut barrier) = self.inner.remove(b)?;
        for s in &barrier.items {
            self.scopes[s].remove(b);
        }
        barrier.items.clear();

        // If this barrier was pending, it no longer exists - remove it
        self.fulfilled.remove(b);
        Some(barrier)
    }

    /// Routes a lifecycle event to all barriers.
    pub fn handle(&mut self, event: &Event<I>) {
        match event {
            Event::Insert(scope) => {
                let s = self.scopes.insert(scope.clone(), Items::new());
                if !self.lifecycle.submit(s) {
                    return;
                }

                // Full scan - scope is always new, seed the reverse index
                for (b, (_, barrier)) in self.inner.slots_mut() {
                    if barrier.contains(scope) {
                        self.scopes[s].insert(b);
                        if barrier.insert(s)
                            && barrier.is_complete(&self.lifecycle)
                        {
                            self.fulfilled.insert(b);
                        } else {
                            // Barrier was pending but is no longer fulfilled -
                            // a new submitted scope invalidates it
                            self.fulfilled.remove(b);
                        }
                    }
                }
            }
            Event::Remove(scope) => {
                let Some(s) = self.scopes.get(scope) else {
                    return;
                };
                if !self.lifecycle.withdraw(s) {
                    return;
                }

                for b in &self.scopes[s] {
                    if self.inner[b].remove(s)
                        && self.inner[b].is_complete(&self.lifecycle)
                    {
                        self.fulfilled.insert(b);
                    }
                }

                self.scopes.remove(s);
            }
        }
    }

    /// Marks a scope as complete, queuing advances for any fulfilled barriers.
    pub fn notify(&mut self, scope: &Scope<I>) {
        let Some(s) = self.scopes.get(scope) else {
            return;
        };
        if !self.lifecycle.complete(s) {
            return;
        }

        // Use reverse index - only visit barriers watching this scope
        for b in &self.scopes[s] {
            if self.inner[b].is_complete(&self.lifecycle) {
                self.fulfilled.insert(b);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<S, I> FromIterator<(S, Barrier<I>)> for Barriers<I>
where
    S: Into<Scope<I>>,
    I: Id,
{
    /// Creates a barrier set from an iterator.
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (S, Barrier<I>)>,
    {
        let mut barriers = Barriers::new();
        for (scope, barrier) in iter {
            barriers.insert(scope.into(), barrier);
        }
        barriers
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Barriers<I>
where
    I: Id,
{
    /// Creates a barrier set.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use zrx_scheduler::action::options::Event;
    use zrx_scheduler::Scope;

    use super::{Barrier, Barriers};

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    type TestId = u32;

    fn scope(id: TestId) -> Scope<TestId> {
        Scope::from(id)
    }

    fn barrier_eq(id: TestId) -> Barrier<TestId> {
        Barrier::new(move |s: &Scope<TestId>| *s == Scope::from(id))
    }

    fn barrier_any() -> Barrier<TestId> {
        Barrier::new(|_| true)
    }

    fn barrier_none() -> Barrier<TestId> {
        Barrier::new(|_| false)
    }

    fn insert(b: &mut Barriers<TestId>, id: TestId) {
        b.handle(&Event::Insert(scope(id)));
    }

    fn remove(b: &mut Barriers<TestId>, id: TestId) {
        b.handle(&Event::Remove(scope(id)));
    }

    fn notify(b: &mut Barriers<TestId>, id: TestId) {
        b.notify(&scope(id));
    }

    fn advances(b: &mut Barriers<TestId>) -> usize {
        b.drain().count()
    }

    // -------------------------------------------------------------------------
    // insert - immediate fulfillment
    // -------------------------------------------------------------------------

    /// A barrier registered after its scopes are already completed is queued
    /// as pending immediately and appears in the next advances() call.
    #[test]
    fn insert_fires_immediately_if_already_complete() {
        let mut b = Barriers::default();
        insert(&mut b, 1);
        notify(&mut b, 1);

        b.insert(scope(99), barrier_eq(1));
        assert_eq!(advances(&mut b), 1);
    }

    /// A barrier registered while its scope is still submitted does not
    /// appear in advances.
    #[test]
    fn insert_does_not_fire_if_scope_submitted() {
        let mut b = Barriers::default();
        insert(&mut b, 1);

        b.insert(scope(99), barrier_eq(1));
        assert_eq!(advances(&mut b), 0);
    }

    /// A barrier with no matching scopes never appears in advances.
    #[test]
    fn insert_does_not_fire_if_no_matching_scopes() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_none());
        assert_eq!(advances(&mut b), 0);
    }

    // -------------------------------------------------------------------------
    // handle - insert
    // -------------------------------------------------------------------------

    /// A scope that is inserted and completed fulfills a barrier.
    #[test]
    fn scope_insert_then_notify_fulfills_barrier() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));

        insert(&mut b, 1);
        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 1);
    }

    /// Duplicate Insert events for the same scope are idempotent.
    #[test]
    fn duplicate_insert_is_idempotent() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));

        insert(&mut b, 1);
        println!("{:#?}", b);
        insert(&mut b, 1); // duplicate - no-op
        println!("{:#?}", b);
        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 1);

        // Second drain must be empty.
        assert_eq!(advances(&mut b), 0);
    }

    // -------------------------------------------------------------------------
    // handle - insert invalidation
    // -------------------------------------------------------------------------

    /// A barrier that was pending is invalidated when a new matching scope
    /// is inserted before advances are drained.
    #[test]
    fn pending_barrier_invalidated_by_new_scope() {
        let mut b = Barriers::default();
        b.insert(
            scope(99),
            Barrier::new(|s: &Scope<TestId>| {
                *s == Scope::from(1) || *s == Scope::from(2)
            }),
        );

        insert(&mut b, 1);
        notify(&mut b, 1); // barrier is now pending

        // New matching scope arrives before caller drains - invalidates.
        insert(&mut b, 2);
        assert_eq!(advances(&mut b), 0, "barrier should be invalidated");

        // Complete the new scope - barrier becomes pending again.
        notify(&mut b, 2);
        assert_eq!(advances(&mut b), 1);
    }

    // -------------------------------------------------------------------------
    // handle - remove
    // -------------------------------------------------------------------------

    /// Removing a scope that was never inserted is a no-op.
    #[test]
    fn remove_unknown_scope_is_noop() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));
        remove(&mut b, 42); // never inserted
        assert_eq!(advances(&mut b), 0);
    }

    /// Removing a submitted scope before it completes does not fulfill
    /// the barrier.
    #[test]
    fn remove_submitted_does_not_fulfill() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));

        insert(&mut b, 1);
        remove(&mut b, 1);
        assert_eq!(advances(&mut b), 0);
    }

    /// Removing the last submitted scope unblocks a barrier that has at
    /// least one completed scope.
    #[test]
    fn remove_unblocks_barrier_with_other_completed_scope() {
        let mut b = Barriers::default();
        b.insert(
            scope(99),
            Barrier::new(|s: &Scope<TestId>| {
                *s == Scope::from(1) || *s == Scope::from(2)
            }),
        );

        insert(&mut b, 1);
        insert(&mut b, 2);
        notify(&mut b, 1); // scope 1 completed, scope 2 still submitted

        // Removing scope 2 unblocks the barrier.
        remove(&mut b, 2);
        assert_eq!(advances(&mut b), 1);
    }

    // -------------------------------------------------------------------------
    // notify - idempotency
    // -------------------------------------------------------------------------

    /// Calling notify twice on the same scope does not queue the barrier twice.
    #[test]
    fn notify_is_idempotent() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));

        insert(&mut b, 1);
        notify(&mut b, 1);
        notify(&mut b, 1); // duplicate - no-op
        assert_eq!(advances(&mut b), 1);

        // Second drain must be empty.
        assert_eq!(advances(&mut b), 0);
    }

    /// Notifying a scope that was never inserted is a no-op.
    #[test]
    fn notify_unknown_scope_is_noop() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));
        notify(&mut b, 42);
        assert_eq!(advances(&mut b), 0);
    }

    // -------------------------------------------------------------------------
    // advances - drain semantics
    // -------------------------------------------------------------------------

    /// advances() only drains fulfilled barriers - unfulfilled pending
    /// entries are not consumed.
    #[test]
    fn advances_only_drains_fulfilled() {
        let mut b = Barriers::default();
        b.insert(scope(10), barrier_eq(1));
        b.insert(scope(20), barrier_eq(2));

        insert(&mut b, 1);
        notify(&mut b, 1); // barrier 10 pending

        // barrier 20 still has scope 2 submitted - not fulfilled yet.
        insert(&mut b, 2);

        assert_eq!(advances(&mut b), 1, "only barrier 10 should drain");

        notify(&mut b, 2);
        assert_eq!(advances(&mut b), 1, "barrier 20 drains now");
    }

    /// Calling advances() twice without any intervening events returns
    /// empty on the second call.
    #[test]
    fn advances_is_consumed_on_drain() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));

        insert(&mut b, 1);
        notify(&mut b, 1);

        assert_eq!(advances(&mut b), 1);
        assert_eq!(advances(&mut b), 0); // already drained
    }

    // -------------------------------------------------------------------------
    // Barrier removal
    // -------------------------------------------------------------------------

    /// A removed barrier never appears in advances even if its scope completes.
    #[test]
    fn removed_barrier_does_not_fire() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));
        b.remove(&scope(99));

        insert(&mut b, 1);
        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 0);
    }

    /// A pending barrier that is removed does not appear in advances.
    #[test]
    fn removed_pending_barrier_does_not_drain() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));

        insert(&mut b, 1);
        notify(&mut b, 1); // barrier is now pending

        b.remove(&scope(99)); // remove before drain
        assert_eq!(advances(&mut b), 0);
    }

    /// Removing a barrier cleans up the reverse index.
    #[test]
    fn removed_barrier_cleans_reverse_index() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_eq(1));
        b.remove(&scope(99));

        // Should not panic or produce spurious advances.
        insert(&mut b, 1);
        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 0);
    }

    // -------------------------------------------------------------------------
    // Catch-all / never-match
    // -------------------------------------------------------------------------

    /// A catch-all barrier is fulfilled as soon as any scope completes.
    #[test]
    fn catch_all_barrier_fires_on_first_completion() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_any());

        insert(&mut b, 1);
        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 1);
    }

    /// A never-matching barrier never appears in advances.
    #[test]
    fn never_match_barrier_never_fires() {
        let mut b = Barriers::default();
        b.insert(scope(99), barrier_none());

        insert(&mut b, 1);
        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 0);
    }

    // -------------------------------------------------------------------------
    // Multi-scope barriers
    // -------------------------------------------------------------------------

    /// A barrier matching multiple scopes waits for all to complete.
    #[test]
    fn multi_scope_barrier_waits_for_all() {
        let mut b = Barriers::default();
        b.insert(
            scope(99),
            Barrier::new(|s: &Scope<TestId>| {
                *s == Scope::from(1) || *s == Scope::from(2)
            }),
        );

        insert(&mut b, 1);
        insert(&mut b, 2);

        notify(&mut b, 1);
        assert_eq!(advances(&mut b), 0, "scope 2 still submitted");

        notify(&mut b, 2);
        assert_eq!(advances(&mut b), 1, "both complete - fires");
    }

    /// Two barriers on different scopes advance independently.
    #[test]
    fn two_barriers_advance_independently() {
        let mut b = Barriers::default();
        b.insert(scope(10), barrier_eq(1));
        b.insert(scope(20), barrier_eq(2));

        insert(&mut b, 1);
        insert(&mut b, 2);
        notify(&mut b, 1);
        notify(&mut b, 2);

        assert_eq!(advances(&mut b), 2);
    }
}
