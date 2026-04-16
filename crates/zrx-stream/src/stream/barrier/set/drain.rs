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

//! Drain iterator implementation for [`Barriers`].

use zrx_scheduler::{Id, Scope};
use zrx_store::stash::{items, Items};
use zrx_store::Stash;

use crate::stream::barrier::Barrier;

use super::{Advance, Barriers};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Drain iterator for [`Barriers`].
pub struct Drain<'a, I> {
    /// Inner set of barriers.
    inner: &'a Stash<Scope<I>, Barrier<I>>,
    /// All known scopes.
    scopes: &'a Stash<Scope<I>, Items>,
    /// Drain iterator over fulfilled barriers.
    items: items::Drain<'a>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Barriers<I>
where
    I: Id,
{
    /// Creates a drain iterator over the barrier set.
    ///
    /// Returns all pending advances, consuming only those that are still
    /// fulfilled at the time of collection. Barriers that were fulfilled but
    /// subsequently invalidated by a new scope insertion are excluded and
    /// removed from the pending set.
    #[must_use]
    pub fn drain(&mut self) -> Drain<'_, I> {
        Drain {
            inner: &self.inner,
            scopes: &self.scopes,
            items: self.fulfilled.drain(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I> Iterator for Drain<'a, I>
where
    I: Id,
{
    type Item = Advance<'a, I>;

    /// Returns the next barrier advancement.
    fn next(&mut self) -> Option<Self::Item> {
        let index = self.items.next()?;
        let barrier = &self.inner[index];
        Some(Advance::new(
            self.inner.key(index).expect("invariant"),
            barrier.items(),
            self.scopes,
        ))
    }
}
