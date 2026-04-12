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

//! Indexing decorator, adding index and range access to a store.

use ahash::HashMap;
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::ops::{Index, Range};

use crate::store::comparator::{Ascending, Comparator};
use crate::store::item::{Key, Value};
use crate::store::{Store, StoreIterable, StoreMut, StoreWithComparator};

mod into_iter;
mod iter;

pub use into_iter::IntoIter;
pub use iter::{Iter, Keys, Values};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Indexing decorator, adding index and range access to a store.
///
/// Sometimes, it's useful to have an ordered index over a store, which allows
/// to access values by their offset, as well as to iterate over the store in
/// an ordered fashion. This data type adds ordering to any [`Store`], because
/// the key is required to implement [`Ord`] anyway. This store can't implement
/// [`StoreMutRef`][], as it would allow the user to obtain mutable references
/// to values, possibly invalidating the ordering. Instead, [`StoreMut`][] is
/// implemented, so updating and removing values is supported, while ensuring
/// the ordering stays intact.
///
/// Note that it's a good idea to use [`Indexed::default`][], since it leverages
/// [`ahash`] as a [`BuildHasher`][], which is the fastest known hasher.
///
/// __Warning__: The affected ranges for insertions and deletions only cover the
/// changed indices of those operations, not the range of items that might need
/// to be updated when each item has an explicit position. This makes sure that
/// this data type can be used in both cases, i.e., when the position is part of
/// the value, as well as when it is implicit by the ordering. When the position
/// is part of the value, all subsequent items will need to be updated as well.
///
/// __Warning__: Compared to other decorators, indexes are rather costly, since
/// they make use of a sorted vector for maintaining the ordering and allowing
/// indexed access at the same time, yielding a worst-case complexity of O(n)
/// for all operations. In case indexed access is not required, it's better to
/// use the [`Ordered`][] decorator, which is based on a [`BTreeMap`][] and has
/// a worst-case complexity of O(log n) for all operations. This is particularly
/// important for write-heavy scenarios with frequently changing values.
///
/// [`BTreeMap`]: std::collections::BTreeMap
/// [`BuildHasher`]: std::hash::BuildHasher
/// [`Indexed::default`]: Default::default
/// [`Ordered`]: crate::store::decorator::Ordered
/// [`StoreMutRef`]: crate::store::StoreMutRef
///
/// # Examples
///
/// ```
/// use zrx_store::decorator::Indexed;
/// use zrx_store::StoreMut;
///
/// // Create store and initial state
/// let mut store = Indexed::default();
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
pub struct Indexed<K, V, S = HashMap<K, V>, C = Ascending> {
    /// Underlying store.
    store: S,
    /// Ordering of values.
    ordering: Vec<K>,
    /// Comparator.
    comparator: C,
    /// Capture types.
    marker: PhantomData<V>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Indexed<K, V, S>
where
    K: Key,
    V: Ord,
    S: Store<K, V>,
{
    /// Creates an indexing decorator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store
    /// let mut store = Indexed::<_, _, HashMap<_, _>>::new();
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    #[inline]
    #[must_use]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self::with_comparator(Ascending)
    }
}

impl<K, V, S, C> Indexed<K, V, S, C>
where
    K: Key,
    V: Ord,
    S: Store<K, V>,
    C: Comparator<V>,
{
    /// Returns the position of the key-value pair in the ordering, or the
    /// position where it should be inserted if the key does not exist.
    fn position<Q>(&self, key: &Q, value: &V) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        // Find the existing position of the key-value pair, or the position at
        // which it should be inserted. Since the ordering is guaranteed to be
        // sorted, we can rely on binary search to find the position and keep
        // the index ordered at all times.
        self.ordering.binary_search_by(|check| {
            let check = check.borrow();
            let prior = self.store.get(check).expect("invariant");
            match self.comparator.cmp(prior, value) {
                Ordering::Equal => check.cmp(key),
                ordering => ordering,
            }
        })
    }

    /// Updates the position of the given key-value pair in the ordering, and
    /// returns the affected range with the found or target position.
    #[allow(clippy::range_plus_one)]
    fn update_position(
        &mut self, key: &K, value: &V,
    ) -> Result<Range<usize>, Range<usize>> {
        self.position(key, value).map(|n| n..n + 1).map_err(|n| {
            let prior = self.store.get(key);

            // At this point, we know that the key-value pair either does not
            // exist, or its position needs to be recomputed, as the value has
            // changed. As we remove the old position before inserting the new
            // one, we must adjust the new position when it is greater.
            let o = prior
                .map(|value| self.position(key, value).expect("invariant"));
            let n = o
                .and_then(|o| if o < n { Some(n - 1) } else { None })
                .unwrap_or(n);

            // Remove old and insert new position
            o.map(|o| self.ordering.remove(o));
            self.ordering.insert(n, key.clone());

            // In case the old position is greater than the new one, we must
            // adjust the range, so consumers can correctly recompute state
            let o = o.map_or(n, |o| if o >= n { o + 1 } else { o });
            o.min(n)..o.max(n + 1)
        })
    }
}

impl<K, V, S, C> Indexed<K, V, S, C>
where
    K: Key,
    V: Ord,
    S: StoreMut<K, V>,
    C: Comparator<V>,
{
    /// Inserts the value identified by the key.
    ///
    /// This method returns the affected [`Range`], which is essential for some
    /// operators to determine what state need to be updated. While for inserts,
    /// the range will always have a length of 1, updates can impact the entire
    /// index, e.g. when the last values is changed to sort to the front.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store
    /// let mut store = Indexed::default();
    ///
    /// // Insert value
    /// let range = store.insert("key", 42);
    /// assert_eq!(range, Some(0..1));
    /// ```
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<Range<usize>> {
        self.update_position(&key, &value).err().inspect(|_| {
            self.store.insert(key, value);
        })
    }

    /// Removes the value identified by the key.
    ///
    /// This method only returns the index of the removed value, if any, since
    /// removing a value does not impact the order of the remaining values.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("key", 42);
    ///
    /// // Remove value
    /// let range = store.remove(&"key");
    /// assert_eq!(range, Some(0));
    /// ```
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    pub fn remove<Q>(&mut self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        if let Some(value) = self.store.get(key) {
            // We can safely use expect here, since we're iterating over a
            // store that is synchronized with the ordering
            let n = self.position(key, value).expect("invariant");
            self.store
                .remove(self.ordering.remove(n).borrow())
                .map(|_| n)
        } else {
            None
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> Store<K, V> for Indexed<K, V, S, C>
where
    K: Key,
    S: Store<K, V>,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::Store;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
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
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::Store;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
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

impl<K, V, S, C> StoreMut<K, V> for Indexed<K, V, S, C>
where
    K: Key,
    V: Ord,
    S: StoreMut<K, V>,
    C: Comparator<V>,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = Indexed::default();
    ///
    /// // Insert value
    /// let value = StoreMut::insert(&mut store, "key", 42);
    /// assert_eq!(value, None);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.update_position(&key, &value).is_err() {
            self.store.insert(key, value)
        } else {
            None
        }
    }

    /// Removes the value identified by the key.
    ///
    /// This method needs to be implemented to satisfy the [`StoreMut`] trait,
    /// but usually, [`Indexed::remove`] should be used instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("key", 42);
    ///
    /// // Remove and return value
    /// let value = StoreMut::remove(&mut store, &"key");
    /// assert_eq!(value, Some(42));
    /// ```
    #[inline]
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        if let Some(value) = self.store.get(key) {
            let n = self.position(key, value).expect("invariant");
            self.store.remove(self.ordering.remove(n).borrow())
        } else {
            None
        }
    }

    /// Removes the value identified by the key and returns both.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
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
        if let Some(value) = self.store.get(key) {
            let n = self.position(key, value).expect("invariant");
            self.store.remove_entry(self.ordering.remove(n).borrow())
        } else {
            None
        }
    }

    /// Clears the store, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
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

impl<K, V, S, C> StoreWithComparator<K, V, C> for Indexed<K, V, S, C>
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
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreWithComparator;
    ///
    /// // Create store
    /// let mut store: Indexed::<_, _, HashMap<_, _>, _> =
    ///     Indexed::with_comparator(Descending);
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    fn with_comparator(comparator: C) -> Self {
        Self {
            store: S::default(),
            ordering: Vec::new(),
            comparator,
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S, C> Index<usize> for Indexed<K, V, S, C>
where
    K: Key,
    S: Store<K, V>,
{
    type Output = K;

    /// Returns a reference to the key at the index.
    ///
    /// We return a reference to the key, as it provides the most flexibility
    /// when working with the index. Since an indexable [`Indexed`] requires the
    /// [`Store`] trait, [`Store::get`] can be used to obtain value associated
    /// with the returned key.
    ///
    /// Moreover, the key can be retrieved in constant time, while retrieving
    /// the value might take longer, depending on the underlying store.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("a", 42);
    /// store.insert("b", 84);
    ///
    /// // Obtain reference to key
    /// let key = &store[1];
    /// assert_eq!(key, &"b");
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.ordering[index]
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> FromIterator<(K, V)> for Indexed<K, V, S>
where
    K: Key,
    V: Ord,
    S: StoreMut<K, V> + Default,
{
    /// Creates a store from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::decorator::Indexed;
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
    /// let store: Indexed<_, _, HashMap<_, _>> =
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
impl<'a, K, V, S, C> IntoIterator for &'a Indexed<K, V, S, C>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, V>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V, S>;

    /// Creates an iterator over the items of the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
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

impl<K, V> Default for Indexed<K, V>
where
    K: Key,
    V: Ord,
{
    /// Creates an indexing decorator with [`HashMap::default`][] as a store.
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
    /// use zrx_store::decorator::Indexed;
    ///
    /// // Create store
    /// let mut store = Indexed::default();
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

impl<K, V, S, C> Debug for Indexed<K, V, S, C>
where
    K: Debug,
    S: Debug,
    C: Debug,
{
    /// Formats the indexing decorator for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Indexed")
            .field("store", &self.store)
            .field("ordering", &self.ordering)
            .finish_non_exhaustive()
    }
}
