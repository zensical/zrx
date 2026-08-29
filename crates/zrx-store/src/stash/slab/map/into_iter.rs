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

//! Consuming iterator implementations for [`Map`].

use std::vec;

use crate::store::item::Key;

use super::{Entry, Map, Slot};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Consuming iterator over a [`Map`].
#[must_use]
#[derive(Debug)]
pub struct IntoIter<K, V> {
    /// Inner iterator.
    inner: vec::IntoIter<Entry<K, V>>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V> IntoIterator for Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;

    /// Creates a consuming iterator over the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for (slot, value) in map {
    ///     println!("{slot}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { inner: self.inner.into_iter() }
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|Entry { key, value }| (key, value))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
