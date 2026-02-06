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

//! Iterator implementations for [`Ordered`].

use std::collections::btree_map;
use std::slice;

use crate::store::comparator::{Ascending, Comparable};
use crate::store::key::Key;
use crate::store::{Store, StoreIterable, StoreKeys, StoreValues};

use super::Ordered;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of an [`Ordered`] store.
pub struct Iter<'a, K, V, C = Ascending> {
    /// Ordering of values.
    ordering: btree_map::Iter<'a, Comparable<V, C>, Vec<K>>,
    /// Current value.
    value: Option<&'a V>,
    /// Current keys.
    keys: slice::Iter<'a, K>,
}

/// Iterator over the keys of an [`Ordered`] store.
pub struct Keys<'a, K, V, C = Ascending> {
    /// Ordering of values.
    ordering: btree_map::Values<'a, Comparable<V, C>, Vec<K>>,
    /// Current keys.
    keys: slice::Iter<'a, K>,
}

/// Iterator over the values of an [`Ordered`] store.
pub struct Values<'a, K, V, C = Ascending> {
    /// Ordering of values.
    ordering: btree_map::Keys<'a, Comparable<V, C>, Vec<K>>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> StoreIterable<K, V> for Ordered<K, V, S, C>
where
    K: Key,
    S: Store<K, V>,
{
    type Iter<'a> = Iter<'a, K, V, C>
    where
        Self: 'a;

    /// Creates an iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{StoreIterable, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.iter() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        Iter {
            ordering: self.ordering.iter(),
            value: None,
            keys: slice::Iter::default(),
        }
    }
}

impl<K, V, S, C> StoreKeys<K, V> for Ordered<K, V, S, C>
where
    K: Key,
    S: Store<K, V>,
{
    type Keys<'a> = Keys<'a, K, V, C>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{StoreKeys, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for key in store.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        Keys {
            ordering: self.ordering.values(),
            keys: slice::Iter::default(),
        }
    }
}

impl<K, V, S, C> StoreValues<K, V> for Ordered<K, V, S, C>
where
    K: Key,
    S: Store<K, V>,
{
    type Values<'a> = Values<'a, K, V, C>
    where
        Self: 'a;

    /// Creates an iterator over the values of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{StoreMut, StoreValues};
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for value in store.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        Values { ordering: self.ordering.keys() }
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, C> Iterator for Iter<'a, K, V, C>
where
    K: Key,
    V: 'a,
{
    type Item = (&'a K, &'a V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Check if we have keys left with the current value
            if let Some(key) = self.keys.next() {
                return self.value.map(|value| (key, value));
            }

            // Fetch the next value and associated keys
            if let Some((value, keys)) = self.ordering.next() {
                self.value = Some(value);
                self.keys = keys.iter();
            } else {
                break;
            }
        }

        // No more items to return
        None
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ordering.size_hint()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, C> Iterator for Keys<'a, K, V, C>
where
    K: Key,
{
    type Item = &'a K;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Check if we have keys left with the current value
            if let Some(key) = self.keys.next() {
                return Some(key);
            }

            // Fetch the next value and associated keys
            if let Some(keys) = self.ordering.next() {
                self.keys = keys.iter();
            } else {
                break;
            }
        }

        // No more items to return
        None
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ordering.size_hint()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, C> Iterator for Values<'a, K, V, C>
where
    K: Key,
    V: 'a,
{
    type Item = &'a V;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.ordering.next().map(|value| &**value)
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ordering.size_hint()
    }
}
