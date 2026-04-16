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
use zrx_store::{Stash, Store};

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

        // Always complete - if it was already completed, we need to notify
        // again, which is exactly the case for rebuilds
        self.lifecycle.complete(s);

        // Use reverse index - only visit barriers watching this scope
        for b in &self.scopes[s] {
            if self.inner[b].is_complete(&self.lifecycle) {
                self.fulfilled.insert(b);
            }
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Barriers<I>
where
    I: Id,
{
    /// Returns the number of barriers.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any barriers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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
