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

//! Iterator implementations for [`Stash`] slots.

use super::slab::{self, Slot};
use super::Stash;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the slots of a [`Stash`].
#[must_use]
#[derive(Debug)]
pub struct Slots<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

/// Mutable iterator over the slots of a [`Stash`].
#[must_use]
#[derive(Debug)]
pub struct SlotsMut<'a, K, V> {
    /// Inner iterator.
    inner: slab::IterMut<'a, (K, V)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Stash<K, V, S> {
    /// Creates an iterator over the slots of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for (slot, (key, value)) in stash.slots() {
    ///     println!("[{slot}] {key}: {value}");
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn slots(&self) -> Slots<'_, K, V> {
        Slots { inner: self.items.iter() }
    }

    /// Creates a mutable iterator over the slots of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for (slot, (key, value)) in stash.slots_mut() {
    ///     println!("[{slot}] {key}: {value}");
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn slots_mut(&mut self) -> SlotsMut<'_, K, V> {
        SlotsMut { inner: self.items.iter_mut() }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Slots<'a, K, V> {
    type Item = (Slot, (&'a K, &'a V));

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.inner.next();
        opt.map(|(slot, (key, value))| (slot, (key, value)))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Slots<'_, K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for SlotsMut<'a, K, V> {
    type Item = (Slot, (&'a K, &'a mut V));

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.inner.next();
        opt.map(|(slot, (key, value))| (slot, (&*key, value)))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for SlotsMut<'_, K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
