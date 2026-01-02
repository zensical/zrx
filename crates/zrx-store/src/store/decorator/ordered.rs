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

//! Ordering decorator, adding ordering to a store.

use ahash::HashMap;
use std::borrow::Borrow;
use std::collections::BTreeMap;
use std::fmt;
use std::vec::IntoIter;

use crate::store::{
    Key, Store, StoreIterable, StoreKeys, StoreMut, StoreValues,
};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Ordering decorator, adding ordering.
///
/// This is a thin wrapper around [`Store`] which is optimized for maintaining
/// a changing ordering of values, while also being able to identify and update
/// them. This is ideal for cases like scheduling, where deadlines must be able
/// to change, while items must be addressable by identifier.
///
/// This implementation uses a [`BTreeMap`] over a [`BinaryHeap`][], because the
/// latter does not expose an efficient API for maintaining the heap invariant.
/// Note that it's a good idea to use [`Ordered::default`], since it leverages
/// [`ahash`] as a [`BuildHasher`][], which is the fastest known hasher.
///
/// [`BinaryHeap`]: std::collections::BinaryHeap
/// [`BuildHasher`]: std::hash::BuildHasher
///
/// # Examples
///
/// ```
/// use zrx_store::decorator::Ordered;
/// use zrx_store::StoreMut;
///
/// // Create store and initial state
/// let mut store = Ordered::default();
/// store.insert("a", 4);
/// store.insert("b", 2);
/// store.insert("c", 3);
/// store.insert("d", 1);
///
/// // Create iterator over the store
/// for (key, value) in store {
///     println!("{key}: {value}");
/// }
/// ```
pub struct Ordered<K, V, S = HashMap<K, V>>
where
    K: Key,
    S: Store<K, V>,
{
    /// Underlying store.
    store: S,
    /// Ordering of values.
    ordering: BTreeMap<V, Vec<K>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Ordered<K, V, S>
where
    K: Key,
    V: Ord,
    S: Store<K, V>,
{
    /// Creates an ordering decorator over a store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::<_, _, HashMap<_, _>>::new();
    /// store.insert("key", 42);
    /// ```
    #[must_use]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            store: S::default(),
            ordering: BTreeMap::new(),
        }
    }

    /// Updates the given key-value pair in the ordering.
    fn update_ordering(&mut self, value: V, key: K) {
        self.ordering
            .entry(value)
            .or_insert_with(|| Vec::with_capacity(1))
            .push(key);
    }

    /// Removes the given key-value pair from the ordering.
    fn remove_ordering<Q>(&mut self, value: &V, key: &Q)
    where
        K: Borrow<Q>,
        Q: Key,
    {
        if let Some(keys) = self.ordering.get_mut(value) {
            keys.retain(|check| check.borrow() != key);
            if keys.is_empty() {
                self.ordering.remove(value);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Store<K, V> for Ordered<K, V, S>
where
    K: Key,
    S: Store<K, V>,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Obtain reference to value
    /// let value = store.get(&"key");
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.get(key)
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Ensure presence of key
    /// let check = store.contains_key(&"key");
    /// assert_eq!(check, true);
    /// ```
    #[inline]
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.contains_key(key)
    }

    /// Returns the number of items in the store.
    #[inline]
    fn len(&self) -> usize {
        self.store.len()
    }
}

impl<K, V, S> StoreMut<K, V> for Ordered<K, V, S>
where
    K: Key,
    V: Clone + Ord,
    S: StoreMut<K, V>,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and insert value
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(prior) = self.store.insert(key.clone(), value.clone()) {
            self.remove_ordering(&prior, &key);
            self.update_ordering(value, key);
            Some(prior)
        } else {
            self.update_ordering(value, key);
            None
        }
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Remove and return value
    /// let value = store.remove(&"key");
    /// assert_eq!(value, Some(42));
    /// ```
    #[inline]
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.remove(key).inspect(|value| {
            self.remove_ordering(value, key);
        })
    }

    /// Clears the store, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Clear store
    /// store.clear();
    /// assert!(store.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        self.store.clear();
        self.ordering.clear();
    }
}

impl<K, V, S> StoreIterable<K, V> for Ordered<K, V, S>
where
    K: Key,
    S: StoreIterable<K, V>,
{
    /// Creates an iterator over the store.
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
    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)>
    where
        K: 'a,
        V: 'a,
    {
        self.ordering
            .iter()
            .flat_map(|(value, keys)| keys.iter().map(move |key| (key, value)))
    }
}

impl<K, V, S> StoreKeys<K, V> for Ordered<K, V, S>
where
    K: Key,
    S: StoreKeys<K, V>,
{
    /// Creates a key iterator over the store.
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
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where
        K: 'a,
    {
        self.ordering.iter().flat_map(|(_, keys)| keys.iter())
    }
}

impl<K, V, S> StoreValues<K, V> for Ordered<K, V, S>
where
    K: Key,
    S: StoreValues<K, V>,
{
    /// Creates a value iterator over the store.
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
    fn values<'a>(&'a self) -> impl Iterator<Item = &'a V>
    where
        V: 'a,
    {
        self.ordering.keys()
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> FromIterator<(K, V)> for Ordered<K, V, S>
where
    K: Key,
    V: Clone + Ord,
    S: StoreMut<K, V> + Default,
{
    /// Creates a store from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create a vector of key-value pairs
    /// let items = vec![
    ///     ("a", 4),
    ///     ("b", 2),
    ///     ("c", 3),
    ///     ("d", 1),
    /// ];
    ///
    /// // Create store from iterator
    /// let store: Ordered<_, _, HashMap<_, _>> =
    ///     items.into_iter().collect();
    ///
    /// // Create iterator over the store
    /// for (key, value) in store {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        let mut store = Self::new();
        for (key, value) in iter {
            store.insert(key, value);
        }
        store
    }
}

impl<K, V, S> IntoIterator for Ordered<K, V, S>
where
    K: Key,
    V: Clone,
    S: Store<K, V>,
{
    type Item = (K, V);
    type IntoIter = IntoIter<Self::Item>;

    /// Creates an iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    fn into_iter(self) -> Self::IntoIter {
        self.ordering
            .into_iter()
            .flat_map(|(value, keys)| {
                keys.into_iter().map(move |key| (key, value.clone()))
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
}

// ----------------------------------------------------------------------------

#[allow(clippy::implicit_hasher)]
impl<K, V> Default for Ordered<K, V, HashMap<K, V>>
where
    K: Key,
    V: Ord,
{
    /// Creates a tracking decorator with [`HashMap::default`] as a store.
    ///
    /// Note that this method does not allow to customize the [`BuildHasher`][],
    /// but uses [`ahash`] by default, which is the fastest known hasher.
    ///
    /// [`BuildHasher`]: std::hash::BuildHasher
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
    /// store.insert("key", 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> fmt::Debug for Ordered<K, V, S>
where
    K: Key + fmt::Debug,
    V: fmt::Debug,
    S: Store<K, V> + fmt::Debug,
{
    /// Formats the ordering decorator for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Order")
            .field("store", &self.store)
            .field("ordering", &self.ordering)
            .finish()
    }
}
