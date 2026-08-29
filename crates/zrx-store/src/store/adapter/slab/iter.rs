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

//! Iterator implementations for [`Slab`].

use slab::Slab;

use crate::store::entry::{Key, Value};
use crate::store::{StoreIterable, StoreIterableMut, StoreKeys, StoreValues};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a [`Slab`].
#[must_use]
#[derive(Debug)]
pub struct Iter<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

/// Mutable iterator over the items of a [`Slab`].
#[must_use]
#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    /// Inner iterator.
    inner: slab::IterMut<'a, (K, V)>,
}

/// Iterator over the keys of a [`Slab`].
#[must_use]
#[derive(Debug)]
pub struct Keys<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

/// Iterator over the values of a [`Slab`].
#[must_use]
#[derive(Debug)]
pub struct Values<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V> StoreIterable<K, V> for Slab<(K, V)>
where
    K: Key,
    V: Value,
{
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{StoreIterable, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Create iterator over store
    /// for (key, value) in StoreIterable::iter(&store) {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        Iter { inner: Slab::iter(self) }
    }
}

impl<K, V> StoreIterableMut<K, V> for Slab<(K, V)>
where
    K: Key,
    V: Value,
{
    type IterMut<'a>
        = IterMut<'a, K, V>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{StoreIterableMut, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Create iterator over store
    /// for (key, value) in StoreIterableMut::iter_mut(&mut store) {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        IterMut { inner: Slab::iter_mut(self) }
    }
}

impl<K, V> StoreKeys<K, V> for Slab<(K, V)>
where
    K: Key,
{
    type Keys<'a>
        = Keys<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{StoreKeys, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Create iterator over store
    /// for key in StoreKeys::keys(&store) {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        Keys { inner: Slab::iter(self) }
    }
}

impl<K, V> StoreValues<K, V> for Slab<(K, V)>
where
    K: Key,
    V: Value,
{
    type Values<'a>
        = Values<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{StoreMut, StoreValues};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Create iterator over store
    /// for key in StoreValues::values(&store) {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        Values { inner: Slab::iter(self) }
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Key,
{
    type Item = (&'a K, &'a V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, (key, value))| (key, value))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V>
where
    K: Key,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: Key,
{
    type Item = (&'a K, &'a mut V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, (key, value))| (&*key, value))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V>
where
    K: Key,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Keys<'a, K, V>
where
    K: Key,
{
    type Item = &'a K;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, (key, _))| key)
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V>
where
    K: Key,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: Key,
{
    type Item = &'a V;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, (_, value))| value)
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V>
where
    K: Key,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
