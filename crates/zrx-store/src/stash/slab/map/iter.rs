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
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Iterator implementations for generational [`Map`].

use std::slice;

use crate::store::item::Key;

use super::{Entry, Map, Slot};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a generational [`Map`].
#[must_use]
#[derive(Debug)]
pub struct Iter<'a, K, V> {
    /// Inner iterator.
    inner: slice::Iter<'a, Entry<K, V>>,
}

/// Mutable iterator over the items of a generational [`Map`].
#[must_use]
#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    /// Inner iterator.
    inner: slice::IterMut<'a, Entry<K, V>>,
}

/// Iterator over the keys of a generational [`Map`].
#[must_use]
#[derive(Debug)]
pub struct Keys<'a, K, V> {
    /// Inner iterator.
    inner: slice::Iter<'a, Entry<K, V>>,
}

/// Iterator over the values of a generational [`Map`].
#[must_use]
#[derive(Debug)]
pub struct Values<'a, K, V> {
    /// Inner iterator.
    inner: slice::Iter<'a, Entry<K, V>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<V, K> Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    /// Creates an iterator over the items of the map.
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
    /// // Create associated map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for (slot, value) in map.iter() {
    ///     println!("{slot}: {value}");
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { inner: self.inner.iter() }
    }

    /// Creates a mutable iterator over the items of the map.
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
    /// // Create associated map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for (slot, value) in map.iter_mut() {
    ///     println!("{slot}: {value}");
    /// }
    /// ```
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut { inner: self.inner.iter_mut() }
    }

    /// Creates an iterator over the keys of the map.
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
    /// // Create associated map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for slot in map.keys() {
    ///     println!("{slot}");
    /// }
    /// ```
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys { inner: self.inner.iter() }
    }

    /// Creates an iterator over the values of the map.
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
    /// // Create associated map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for value in map.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values { inner: self.inner.iter() }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|Entry { key, value }| (key, value))
    }

    /// Returns the remaining length.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, &'a mut V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|Entry { key, value }| (&*key, value))
    }

    /// Returns the remaining length.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|Entry { key, .. }| key)
    }

    /// Returns the remaining length.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|Entry { value, .. }| value)
    }

    /// Returns the remaining length.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
