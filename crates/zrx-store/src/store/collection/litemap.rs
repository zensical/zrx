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

//! Store implementations for `litemap`.

use litemap::{store, LiteMap};
use std::borrow::Borrow;

use crate::store::util::update_if_changed;
use crate::store::{
    Key, Store, StoreIterable, StoreIterableMut, StoreKeys, StoreMut,
    StoreMutRef, StoreValues,
};

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Store<K, V> for LiteMap<K, V, S>
where
    K: Key,
    S: store::Store<K, V>,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
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
        LiteMap::get(self, key)
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
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
        LiteMap::contains_key(self, key)
    }

    /// Returns the number of items in the store.
    #[inline]
    fn len(&self) -> usize {
        LiteMap::len(self)
    }
}

impl<K, V, S> StoreMut<K, V> for LiteMap<K, V, S>
where
    K: Key,
    S: store::StoreMut<K, V>,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and insert value
    /// let mut store = LiteMap::new_vec();
    /// store.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        LiteMap::insert(self, key, value)
    }

    /// Inserts the value identified by the key if it changed.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = LiteMap::new_vec();
    ///
    /// // Insert value
    /// let check = store.insert_if_changed(&"key", &42);
    /// assert_eq!(check, true);
    ///
    /// // Ignore unchanged value
    /// let check = store.insert_if_changed(&"key", &42);
    /// assert_eq!(check, false);
    ///
    /// // Update value
    /// let check = store.insert_if_changed(&"key", &84);
    /// assert_eq!(check, true);
    /// ```
    #[inline]
    fn insert_if_changed(&mut self, key: &K, value: &V) -> bool
    where
        V: Clone + Eq,
    {
        LiteMap::get_mut(self, key)
            .map(|check| update_if_changed(check, value))
            .unwrap_or_else(|| {
                LiteMap::insert(self, key.clone(), value.clone());
                true
            })
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
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
        LiteMap::remove(self, key)
    }

    /// Clears the store, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
    /// store.insert("key", 42);
    ///
    /// // Clear store
    /// store.clear();
    /// assert!(store.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        LiteMap::clear(self);
    }
}

impl<K, V, S> StoreMutRef<K, V> for LiteMap<K, V, S>
where
    K: Key,
    S: store::StoreMut<K, V>,
{
    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{StoreMut, StoreMutRef};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
    /// store.insert("key", 42);
    ///
    /// // Obtain mutable reference to value
    /// let mut value = store.get_mut(&"key");
    /// assert_eq!(value, Some(&mut 42));
    /// ```
    #[inline]
    fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        LiteMap::get_mut(self, key)
    }

    /// Returns a mutable reference to the value or creates the default.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{StoreMut, StoreMutRef};
    ///
    /// // Create store
    /// let mut store = LiteMap::<_, i32>::new_vec();
    ///
    /// // Obtain mutable reference to value
    /// let value = store.get_or_insert_default(&"key");
    /// assert_eq!(value, &mut 0);
    /// ```
    #[inline]
    fn get_or_insert_default(&mut self, key: &K) -> &mut V
    where
        V: Default,
    {
        // Unfortunately, there's no way to insert a value and obtain a mutable
        // reference to it in one step, so we must use two operations. However,
        // we can safely use expect here, as we've just inserted the key and
        // know that it is present, unless the map panicked
        LiteMap::try_insert(self, key.clone(), V::default());
        LiteMap::get_mut(self, key).expect("invariant")
    }
}

impl<K, V, S> StoreIterable<K, V> for LiteMap<K, V, S>
where
    K: Key,
    S: store::Store<K, V>,
    for<'a> S: store::StoreIterable<'a, K, V>,
{
    /// Creates an iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{StoreIterable, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
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
        LiteMap::iter(self)
    }
}

impl<K, V, S> StoreIterableMut<K, V> for LiteMap<K, V, S>
where
    K: Key,
    S: store::Store<K, V>,
    for<'a> S: store::StoreIterableMut<'a, K, V>,
{
    /// Creates a mutable iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{StoreIterableMut, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a K, &'a mut V)>
    where
        K: 'a,
        V: 'a,
    {
        LiteMap::iter_mut(self)
    }
}

impl<K, V, S> StoreKeys<K, V> for LiteMap<K, V, S>
where
    K: Key,
    for<'a> S: store::StoreIterable<'a, K, V>,
{
    /// Creates a key iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{StoreKeys, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
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
        V: 'a,
    {
        LiteMap::iter(self).map(|(key, _)| key)
    }
}

impl<K, V, S> StoreValues<K, V> for LiteMap<K, V, S>
where
    K: Key,
    for<'a> S: store::StoreIterable<'a, K, V>,
{
    /// Creates a value iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use litemap::LiteMap;
    /// use zrx_store::{StoreMut, StoreValues};
    ///
    /// // Create store and initial state
    /// let mut store = LiteMap::new_vec();
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
        LiteMap::iter(self).map(|(_, value)| value)
    }
}
