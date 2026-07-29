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
use std::collections::BTreeSet;
use std::fmt::{self, Debug};

use crate::store::comparator::{Ascending, Comparable, Comparator};
use crate::store::item::{Key, Value};
use crate::store::{Store, StoreIterable, StoreMut, StoreWithComparator};

mod into_iter;
mod iter;

pub use into_iter::IntoIter;
pub use iter::{Iter, Keys, Values};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Ordering decorator, adding ordering to a store.
///
/// This is a thin wrapper around [`Store`] which is optimized for maintaining
/// a changing ordering of values, while also being able to identify and update
/// them. This is ideal for cases like scheduling, where deadlines must be able
/// to change, while items must be addressable by identifier.
///
/// This implementation uses a [`BTreeSet`] instead of a [`BinaryHeap`][], since
/// the latter doesn't expose an API to be able to maintain the heap invariant.
/// Note that it's a good idea to use [`Ordered::default`][], since it leverages
/// [`ahash`] as a [`BuildHasher`][], which is the fastest known hasher.
///
/// [`BinaryHeap`]: std::collections::BinaryHeap
/// [`BuildHasher`]: std::hash::BuildHasher
/// [`Ordered::default`]: Default::default
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
/// for (key, value) in &store {
///     println!("{key}: {value}");
/// }
/// ```
#[derive(Clone, PartialEq, Eq)]
pub struct Ordered<K, V, S = HashMap<K, V>, C = Ascending> {
    /// Underlying store.
    store: S,
    /// Ordering of values.
    ordering: BTreeSet<(Comparable<V, C>, K)>,
    /// Comparator.
    comparator: C,
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
    /// Creates an ordering decorator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = Ordered::<_, _, HashMap<_, _>>::new();
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    #[must_use]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self::with_comparator(Ascending)
    }
}

impl<K, V, S, C> Ordered<K, V, S, C>
where
    K: Key,
    V: Ord,
    S: Store<K, V>,
    C: Comparator<V> + Clone,
{
    /// Inserts the given key-value pair into the ordering.
    fn insert_ordering(&mut self, key: K, value: V) {
        let value = Comparable::new(value, self.comparator.clone());
        self.ordering.insert((value, key));
    }

    /// Removes the given key-value pair from the ordering.
    fn remove_ordering(&mut self, key: K, value: V) -> Option<(K, V)> {
        let value = Comparable::new(value, self.comparator.clone());

        // Remove the entry from the ordering, and return the key and value, as
        // we need to return it to the caller. Note that we can be sure that the
        // value and key exist, because the ordering is synchronized with the
        // store, so we just pass it through as an option for ergonomics.
        let opt = self.ordering.take(&(value, key));
        opt.map(|(value, key)| (key, value.into_inner()))
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> Store<K, V> for Ordered<K, V, S, C>
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

impl<K, V, S, C> StoreMut<K, V> for Ordered<K, V, S, C>
where
    K: Key,
    V: Clone + Ord,
    S: StoreMut<K, V>,
    C: Comparator<V> + Clone,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = Ordered::default();
    ///
    /// // Insert value
    /// let value = store.insert("key", 42);
    /// assert_eq!(value, None);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        let exists = self.store.contains_key(&key);
        if let Some(prior) = self.store.insert(key.clone(), value.clone()) {
            return self.remove_ordering(key, prior).map(|(key, prior)| {
                self.insert_ordering(key, value);
                prior
            });
        }
        if !exists {
            self.insert_ordering(key, value);
        }
        None
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
        self.store.remove_entry(key).and_then(|(key, value)| {
            self.remove_ordering(key, value).map(|(_, value)| value)
        })
    }

    /// Removes the value identified by the key and returns both.
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
    /// // Remove and return entry
    /// let entry = store.remove_entry(&"key");
    /// assert_eq!(entry, Some(("key", 42)));
    /// ```
    #[inline]
    fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.remove_entry(key).and_then(|(key, value)| {
            self.remove_ordering(key, value) // fmt
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

// ----------------------------------------------------------------------------

impl<K, V, S, C> StoreWithComparator<K, V, C> for Ordered<K, V, S, C>
where
    K: Key,
    S: Store<K, V> + Default,
    C: Comparator<V>,
{
    /// Creates a store with the given comparator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::comparator::Descending;
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::{StoreMut, StoreWithComparator};
    ///
    /// // Create store
    /// let mut store: Ordered::<_, _, HashMap<_, _>, _> =
    ///     Ordered::with_comparator(Descending);
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    fn with_comparator(comparator: C) -> Self {
        Self {
            store: S::default(),
            ordering: BTreeSet::new(),
            comparator,
        }
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

#[allow(clippy::into_iter_without_iter)]
impl<'a, K, V, S, C> IntoIterator for &'a Ordered<K, V, S, C>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, V>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V, C>;

    /// Creates an iterator over the items of the store.
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
    /// for (key, value) in &store {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Default for Ordered<K, V>
where
    K: Key,
    V: Ord,
{
    /// Creates a tracking decorator with [`HashMap::default`][] as a store.
    ///
    /// Note that this method does not allow to customize the [`BuildHasher`][],
    /// but uses [`ahash`] by default, which is the fastest known hasher.
    ///
    /// [`BuildHasher`]: std::hash::BuildHasher
    /// [`HashMap::default`]: Default::default
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = Ordered::default();
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S, C> Debug for Ordered<K, V, S, C>
where
    K: Debug,
    V: Debug,
    S: Debug,
{
    /// Formats the ordering decorator for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Order")
            .field("store", &self.store)
            .field("ordering", &self.ordering)
            .finish_non_exhaustive()
    }
}
