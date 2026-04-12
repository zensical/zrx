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

//! Iterator implementations for [`Indexed`].

use ahash::HashMap;
use std::marker::PhantomData;
use std::ops::{Bound, RangeBounds};
use std::slice;

use crate::store::item::{Key, Value};
use crate::store::{Store, StoreIterable, StoreKeys, StoreValues};

use super::Indexed;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of an [`Indexed`] store.
#[derive(Debug)]
pub struct Iter<'a, K, V, S = HashMap<K, V>> {
    /// Underlying store.
    store: &'a S,
    /// Ordering of values.
    ordering: slice::Iter<'a, K>,
    /// Capture types.
    marker: PhantomData<V>,
}

/// Iterator over the values of an [`Indexed`] store.
#[derive(Debug)]
pub struct Values<'a, K, V, S = HashMap<K, V>> {
    /// Underlying store.
    store: &'a S,
    /// Ordering of values.
    ordering: slice::Iter<'a, K>,
    /// Capture types.
    marker: PhantomData<V>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> Indexed<K, V, S, C> {
    /// Creates an iterator over a range of items of the store.
    ///
    /// This method is not implemented as part of [`StoreRange`][], because it
    /// deviates from the trait, as it uses numeric indices instead of keys.
    ///
    /// [`StoreRange`]: crate::store::StoreRange
    ///
    /// # Panics
    ///
    /// Panics if the range is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("a", 42);
    /// store.insert("b", 22);
    /// store.insert("c", 32);
    /// store.insert("d", 12);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.range(2..4) {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    pub fn range<R>(&self, range: R) -> Iter<'_, K, V, S>
    where
        R: RangeBounds<usize>,
    {
        // Compute length
        let len = self.ordering.len();

        // Compute range start
        let start = match range.start_bound() {
            Bound::Included(&start) => start,
            Bound::Excluded(&start) => start + 1,
            Bound::Unbounded => 0,
        };

        // Compute range end
        let end = match range.end_bound() {
            Bound::Included(&end) => end + 1,
            Bound::Excluded(&end) => end,
            Bound::Unbounded => len,
        };

        // Create range iterator
        Iter {
            ordering: self.ordering[start..end].iter(),
            store: &self.store,
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> StoreIterable<K, V> for Indexed<K, V, S, C>
where
    K: Key,
    V: Value,
    S: Store<K, V>,
{
    type Iter<'a> = Iter<'a, K, V, S>
    where
        Self: 'a;

    /// Creates an iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreIterable;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
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
            store: &self.store,
            ordering: self.ordering.iter(),
            marker: PhantomData,
        }
    }
}

impl<K, V, S, C> StoreKeys<K, V> for Indexed<K, V, S, C>
where
    K: Key,
    S: Store<K, V>,
{
    type Keys<'a> = Keys<'a, K>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreKeys;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for key in store.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        self.ordering.iter()
    }
}

impl<K, V, S, C> StoreValues<K, V> for Indexed<K, V, S, C>
where
    K: Key,
    V: Value,
    S: Store<K, V>,
{
    type Values<'a> = Values<'a, K, V, S>
    where
        Self: 'a;

    /// Creates an iterator over the values of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreValues;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for value in store.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        Values {
            store: &self.store,
            ordering: self.ordering.iter(),
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, S> Iterator for Iter<'a, K, V, S>
where
    K: Key,
    V: Value,
    S: Store<K, V>,
{
    type Item = (&'a K, &'a V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.ordering.next();
        opt.and_then(|key| self.store.get(key).map(|value| (key, value)))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ordering.size_hint()
    }
}

impl<K, V, S> ExactSizeIterator for Iter<'_, K, V, S>
where
    K: Key,
    V: Value,
    S: Store<K, V>,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.ordering.len()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V, S> Iterator for Values<'a, K, V, S>
where
    K: Key,
    V: Value,
    S: Store<K, V>,
{
    type Item = &'a V;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let opt = self.ordering.next();
        opt.and_then(|key| self.store.get(key))
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ordering.size_hint()
    }
}

impl<K, V, S> ExactSizeIterator for Values<'_, K, V, S>
where
    K: Key,
    V: Value,
    S: Store<K, V>,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.ordering.len()
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Iterator over the keys of an [`Indexed`] store.
pub type Keys<'a, K> = slice::Iter<'a, K>;
