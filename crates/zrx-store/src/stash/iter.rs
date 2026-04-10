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

use slab::Slab;
use std::ptr;

use crate::store::item::{Key, Value};
use crate::store::{StoreIterable, StoreIterableMut, StoreKeys, StoreValues};

use super::Stash;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a [`Stash`].
pub struct Iter<'a, K, V, S>
where
    K: Key,
    S: StoreIterable<K, usize> + 'a,
{
    /// Inner iterator.
    inner: S::Iter<'a>,
    /// Stash items.
    items: &'a Slab<(K, V)>,
}

/// Mutable iterator over the items of a [`Stash`].
pub struct IterMut<'a, K, V, S>
where
    K: Key,
    S: StoreIterableMut<K, usize> + 'a,
{
    /// Inner iterator.
    inner: S::IterMut<'a>,
    /// Stash items.
    items: &'a mut Slab<(K, V)>,
}

/// Iterator over the keys of a [`Stash`].
pub struct Keys<'a, K, S>
where
    K: Key,
    S: StoreKeys<K, usize> + 'a,
{
    /// Inner iterator.
    inner: S::Keys<'a>,
}

/// Iterator over the values of a [`Stash`].
pub struct Values<'a, K, V, S>
where
    K: Key,
    S: StoreValues<K, usize> + 'a,
{
    /// Inner iterator.
    inner: S::Values<'a>,
    /// Stash items.
    items: &'a Slab<(K, V)>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> StoreIterable<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, usize>,
{
    type Iter<'a> = Iter<'a, K, V, S>
    where
        Self: 'a;

    /// Creates an iterator over the items of a stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterable, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for (key, value) in stash.iter() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        Iter {
            inner: self.store.iter(),
            items: &self.items,
        }
    }
}

impl<K, V, S> StoreIterableMut<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterableMut<K, usize>,
{
    type IterMut<'a> = IterMut<'a, K, V, S>
    where
        Self: 'a;

    /// Creates an iterator over the items of a stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterableMut, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for (key, value) in stash.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        IterMut {
            inner: self.store.iter_mut(),
            items: &mut self.items,
        }
    }
}

impl<K, V, S> StoreKeys<K, V> for Stash<K, V, S>
where
    K: Key,
    S: StoreKeys<K, usize>,
{
    type Keys<'a> = Keys<'a, K, S>
    where
        Self: 'a;

    /// Creates an iterator over the keys of a stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreKeys, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for key in stash.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        Keys { inner: self.store.keys() }
    }
}

impl<K, V, S> StoreValues<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreValues<K, usize>,
{
    type Values<'a> = Values<'a, K, V, S>
    where
        Self: 'a;

    /// Creates an iterator over the values of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut, StoreValues};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for value in stash.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        Values {
            inner: self.store.values(),
            items: &self.items,
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, S> Iterator for Iter<'a, K, V, S>
where
    K: Key,
    S: StoreIterable<K, usize>,
{
    type Item = (&'a K, &'a V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.inner.next();
        opt.map(|(key, &index)| (key, &self.items[index].1))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V, S> ExactSizeIterator for Iter<'a, K, V, S>
where
    K: Key,
    S: StoreIterable<K, usize>,
    S::Iter<'a>: ExactSizeIterator,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, S> Iterator for IterMut<'a, K, V, S>
where
    K: Key,
    S: StoreIterableMut<K, usize>,
{
    type Item = (&'a K, &'a mut V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.inner.next();
        opt.map(|(key, &mut index)| {
            let items = ptr::addr_of_mut!(self.items);
            // SAFETY: Since both data structures are synchronized with each
            // other, and we have a mutable reference to the store, it's safe
            (key, unsafe { &mut (&mut *items)[index].1 })
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V, S> ExactSizeIterator for IterMut<'a, K, V, S>
where
    K: Key,
    S: StoreIterableMut<K, usize>,
    S::IterMut<'a>: ExactSizeIterator,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, S> Iterator for Keys<'a, K, S>
where
    K: Key,
    S: StoreKeys<K, usize>,
{
    type Item = &'a K;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, S> ExactSizeIterator for Keys<'a, K, S>
where
    K: Key,
    S: StoreKeys<K, usize>,
    S::Keys<'a>: ExactSizeIterator,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, S> Iterator for Values<'a, K, V, S>
where
    K: Key,
    S: StoreValues<K, usize>,
{
    type Item = &'a V;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.inner.next();
        opt.map(|&index| &self.items[index].1)
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V, S> ExactSizeIterator for Values<'a, K, V, S>
where
    K: Key,
    S: StoreValues<K, usize>,
    S::Values<'a>: ExactSizeIterator,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.inner.len()
    }
}
