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

//! Store implementations for collections.

use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasher;

use crate::store::item::Key;
use crate::store::{Store, StoreMut, StoreMutRef};

mod iter;

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Store<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
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
        HashMap::get(self, key)
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
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
        HashMap::contains_key(self, key)
    }

    /// Returns the number of items in the store.
    #[inline]
    fn len(&self) -> usize {
        HashMap::len(self)
    }
}

impl<K, V, S> StoreMut<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = HashMap::new();
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        HashMap::insert(self, key, value)
    }

    /// Inserts the value identified by the key if it changed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = HashMap::new();
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
        HashMap::get_mut(self, key)
            .map(|prior| update_if_changed(prior, value))
            .unwrap_or_else(|| {
                HashMap::insert(self, key.clone(), value.clone());
                true
            })
    }

    /// Removes the value identified by the key.
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
        HashMap::remove(self, key)
    }

    /// Removes the value identified by the key and returns both.
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
        HashMap::remove_entry(self, key)
    }

    /// Clears the store, removing all items.
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
    /// // Clear store
    /// store.clear();
    /// assert!(store.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        HashMap::clear(self);
    }
}

impl<K, V, S> StoreMutRef<K, V> for HashMap<K, V, S>
where
    K: Key,
    S: BuildHasher,
{
    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{StoreMut, StoreMutRef};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
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
        HashMap::get_mut(self, key)
    }

    /// Returns a mutable reference to the value or creates the default.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreMutRef;
    ///
    /// // Create store
    /// let mut store = HashMap::new();
    /// # let _: HashMap<_, i32> = store;
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
        HashMap::entry(self, key.clone()).or_default()
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Store<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
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
        BTreeMap::get(self, key)
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
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
        BTreeMap::contains_key(self, key)
    }

    /// Returns the number of items in the store.
    #[inline]
    fn len(&self) -> usize {
        BTreeMap::len(self)
    }
}

impl<K, V> StoreMut<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = BTreeMap::new();
    ///
    /// // Insert value
    /// store.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        BTreeMap::insert(self, key, value)
    }

    /// Inserts the value identified by the key if it changed.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = BTreeMap::new();
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
        BTreeMap::get_mut(self, key)
            .map(|prior| update_if_changed(prior, value))
            .unwrap_or_else(|| {
                BTreeMap::insert(self, key.clone(), value.clone());
                true
            })
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
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
        BTreeMap::remove(self, key)
    }

    /// Removes the value identified by the key and returns both.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
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
        BTreeMap::remove_entry(self, key)
    }

    /// Clears the store, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
    /// store.insert("key", 42);
    ///
    /// // Clear store
    /// store.clear();
    /// assert!(store.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        BTreeMap::clear(self);
    }
}

impl<K, V> StoreMutRef<K, V> for BTreeMap<K, V>
where
    K: Key,
{
    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::{StoreMut, StoreMutRef};
    ///
    /// // Create store and initial state
    /// let mut store = BTreeMap::new();
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
        BTreeMap::get_mut(self, key)
    }

    /// Returns a mutable reference to the value or creates the default.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use zrx_store::StoreMutRef;
    ///
    /// // Create store
    /// let mut store = BTreeMap::new();
    /// # let _: BTreeMap<_, i32> = store;
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
        BTreeMap::entry(self, key.clone()).or_default()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Updates the prior value if it has changed.
#[inline]
fn update_if_changed<V>(prior: &mut V, value: &V) -> bool
where
    V: Clone + Eq,
{
    if prior == value {
        false
    } else {
        *prior = value.clone();
        true
    }
}
