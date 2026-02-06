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

//! Iterator implementations for collections.

use std::collections::btree_map::{self, BTreeMap};
use std::collections::hash_map::{self, HashMap};
use std::hash::BuildHasher;
use std::ops::RangeBounds;

use crate::store::key::Key;
use crate::store::{
    StoreIterable, StoreIterableMut, StoreKeys, StoreRange, StoreValues,
};

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> StoreIterable<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    type Iter<'a> = hash_map::Iter<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        HashMap::iter(self)
    }
}

impl<K, V, S> StoreIterableMut<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    type IterMut<'a> = hash_map::IterMut<'a, K, V>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{StoreIterableMut, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        HashMap::iter_mut(self)
    }
}

impl<K, V, S> StoreKeys<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    type Keys<'a> = hash_map::Keys<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{StoreKeys, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for key in store.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        HashMap::keys(self)
    }
}

impl<K, V, S> StoreValues<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    type Values<'a> = hash_map::Values<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{StoreMut, StoreValues};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for value in store.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        HashMap::values(self)
    }
}

// ----------------------------------------------------------------------------

impl<K, V> StoreIterable<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    type Iter<'a> = btree_map::Iter<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{StoreIterable, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.iter() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        BTreeMap::iter(self)
    }
}

impl<K, V> StoreIterableMut<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    type IterMut<'a> = btree_map::IterMut<'a, K, V>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{StoreIterableMut, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        BTreeMap::iter_mut(self)
    }
}

impl<K, V> StoreKeys<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    type Keys<'a> = btree_map::Keys<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{StoreKeys, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for key in store.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        BTreeMap::keys(self)
    }
}

impl<K, V> StoreValues<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    type Values<'a> = btree_map::Values<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{StoreMut, StoreValues};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for value in store.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        BTreeMap::values(self)
    }
}

impl<K, V> StoreRange<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    type Range<'a> = btree_map::Range<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over a range of items in a store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{StoreMut, StoreRange};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
    /// store.insert("a", 42);
    /// store.insert("b", 84);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.range("b"..) {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<K>,
    {
        BTreeMap::range(self, range)
    }
}
