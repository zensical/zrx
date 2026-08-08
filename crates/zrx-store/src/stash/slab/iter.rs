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
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Iterator implementations for a generational [`Slab`].

use slab;

use super::slot::Slot;
use super::{Entry, Slab};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a generational [`Slab`].
#[must_use]
#[derive(Debug)]
pub struct Iter<'a, T> {
    /// Inner iterator.
    inner: slab::Iter<'a, Entry<T>>,
}

/// Mutable iterator over the items of a generational [`Slab`].
#[must_use]
#[derive(Debug)]
pub struct IterMut<'a, T> {
    /// Inner iterator.
    inner: slab::IterMut<'a, Entry<T>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Slab<T> {
    /// Creates an iterator over the items of the slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// slab.insert(42);
    ///
    /// // Create iterator over slab
    /// for (slot, value) in slab.iter() {
    ///     println!("[{slot}]: {value}");
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, T> {
        Iter { inner: self.inner.iter() }
    }

    /// Creates a mutable iterator over the items of the slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// slab.insert(42);
    ///
    /// // Create iterator over slab
    /// for (slot, value) in slab.iter_mut() {
    ///     println!("[{slot}]: {value}");
    /// }
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut { inner: self.inner.iter_mut() }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> Clone for Iter<'_, T> {
    /// Clones the iterator.
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

// ----------------------------------------------------------------------------

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Slot, &'a T);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, entry)| {
            (Slot::from_parts(index, entry.generation), &entry.value)
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Slot, &'a mut T);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(index, entry)| {
            (Slot::from_parts(index, entry.generation), &mut entry.value)
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
