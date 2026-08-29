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

//! Iterator implementations for [`Stash`].

use crate::store::entry::{Key, Value};
use crate::store::{
    Store, StoreIterable, StoreIterableMut, StoreKeys, StoreMut, StoreValues,
};

use super::Stash;
use super::slab::{self, Slot};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a [`Stash`].
#[must_use]
#[derive(Debug)]
pub struct Iter<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

/// Mutable iterator over the items of a [`Stash`].
#[must_use]
#[derive(Debug)]
pub struct IterMut<'a, K, V> {
    /// Inner iterator.
    inner: slab::IterMut<'a, (K, V)>,
}

/// Iterator over the keys of a [`Stash`].
#[must_use]
#[derive(Debug)]
pub struct Keys<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

/// Iterator over the values of a [`Stash`].
#[must_use]
#[derive(Debug)]
pub struct Values<'a, K, V> {
    /// Inner iterator.
    inner: slab::Iter<'a, (K, V)>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> StoreIterable<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: Store<K, Slot>,
{
    type Iter<'a>
        = Iter<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the items of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterable};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for (key, value) in stash.iter() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        Iter { inner: self.items.iter() }
    }
}

impl<K, V, S> StoreIterableMut<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreMut<K, Slot>,
{
    type IterMut<'a>
        = IterMut<'a, K, V>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterableMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for (key, value) in stash.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        IterMut { inner: self.items.iter_mut() }
    }
}

impl<K, V, S> StoreKeys<K, V> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, Slot>,
{
    type Keys<'a>
        = Keys<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreKeys};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for key in stash.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        Keys { inner: self.items.iter() }
    }
}

impl<K, V, S> StoreValues<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: Store<K, Slot>,
{
    type Values<'a>
        = Values<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreValues};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for value in stash.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        Values { inner: self.items.iter() }
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Iter<'a, K, V> {
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
        self.inner.next().map(|(_, (key, value))| (&*key, value))
    }

    /// Returns the bounds on the remaining length of the iterator.
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
        self.inner.next().map(|(_, (key, _))| key)
    }

    /// Returns the bounds on the remaining length of the iterator.
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
        self.inner.next().map(|(_, (_, value))| value)
    }

    /// Returns the bounds on the remaining length of the iterator.
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
