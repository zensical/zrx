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

//! Iterator implementation for [`Advance`].

use zrx_scheduler::{Id, Key};
use zrx_store::stash::slab::map;
use zrx_store::Stash;

use crate::stream::barrier::Slots;

use super::Advance;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator for [`Advance`].
pub struct Iter<'a, I> {
    /// Inner iterator.
    inner: map::Iter<'a, zrx_store::stash::Slot, ()>,
    /// All known scopes.
    stash: &'a Stash<Key<I>, Slots>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, I> Advance<'a, I>
where
    I: Id,
{
    /// Creates an iterator over the barrier advancement.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'a, I> {
        Iter {
            inner: self.slots.iter(),
            stash: self.scopes,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I> Iterator for Iter<'a, I>
where
    I: Id,
{
    type Item = &'a Key<I>;

    /// Returns the next scope.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .and_then(|(slot, ())| self.stash.key(*slot))
    }
}
