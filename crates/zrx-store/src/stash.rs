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

//! Stash.

use ahash::HashMap;
use slab::Slab;
use std::borrow::Borrow;
use std::fmt::{self, Debug};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Index, IndexMut};

use crate::store::adapter::slab::{Iter, IterMut};
use crate::store::item::{Key, Value};
use crate::store::{
    Store, StoreIterable, StoreIterableMut, StoreMut, StoreMutRef,
};

pub mod items;
mod iter;

pub use items::Items;
pub use iter::{Slots, SlotsMut};

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

/// Stash.
///
/// This data type implements a simple stash, which is a key-value store that
/// is optimized for fast insertion and retrieval of items by index. It's built
/// on a [`Slab`], together with a [`Store`] that provides the underlying item
/// storage. Stashes offer optimal performance for temporary storage.
///
/// Iteration happens on the underlying [`Slab`], which means that the order of
/// items is stable, but not sorted by key. This ensures, that iteration is not
/// affected by insertions and removals, and cache efficient, since no lookups
/// need to be performed on the underlying [`Store`] to obtain the items. Note
/// that the store iterator traits don't allow to return indices for the items,
/// only references to keys and values. In case indices are required, the stash
/// can be iterated with [`Stash::slots`] or [`Stash::slots_mut`], since those
/// return both, indices and references to keys and values.
///
/// # Examples
///
/// ```
/// use zrx_store::{Stash, StoreMut};
///
/// // Create stash and initial state
/// let mut stash = Stash::default();
/// stash.insert("key", 42);
///
/// // Create iterator over the stash
/// for (key, value) in &stash {
///     println!("{key}: {value}");
/// }
/// ```
pub struct Stash<K, V, S = HashMap<K, usize>>
where
    K: Key,
    S: Store<K, usize>,
{
    /// Underlying store.
    store: S,
    /// Stash items.
    items: Slab<(K, V)>,
    /// Capture types.
    marker: PhantomData<K>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Stash<K, V, S>
where
    K: Key,
    S: Store<K, usize>,
{
    /// Creates a stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash
    /// let mut stash = Stash::<_, _, HashMap<_, _>>::new();
    /// stash.insert("key", 42);
    /// ```
    #[must_use]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            store: S::default(),
            items: Slab::new(),
            marker: PhantomData,
        }
    }

    /// Returns the index of the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, Store, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Obtain index of value
    /// let index = stash.get(&"key");
    /// assert_eq!(index, Some(0));
    /// ```
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.get(key).copied()
    }
}

impl<K, V, S> Stash<K, V, S>
where
    K: Key,
    S: StoreMut<K, usize>,
{
    /// Inserts the value identified by the key and returns its index.
    ///
    /// This method inserts the value and returns an index into the store that
    /// can be used to retrieve an immutable or mutable reference to the value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash
    /// let mut stash = Stash::default();
    ///
    /// // Insert value
    /// let index = stash.insert("key", 42);
    /// assert_eq!(index, 0);
    /// ```
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> usize {
        if let Some(&index) = self.store.get(&key) {
            self.items[index].1 = value;
            index
        } else {
            let index = self.items.insert((key.clone(), value));
            self.store.insert(key, index);
            index
        }
    }

    /// Removes the entry at the index and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Remove and return entry
    /// let entry = stash.remove(0);
    /// assert_eq!(entry, Some(("key", 42)));
    /// ```
    #[inline]
    pub fn remove(&mut self, index: usize) -> Option<(K, V)> {
        if let Some((key, _)) = self.items.get(index) {
            self.store.remove(key).map(|index| self.items.remove(index))
        } else {
            None
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Store<K, V> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, usize>,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, Store, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Obtain reference to value
    /// let value = Store::get(&stash, &"key");
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        match self.store.get(key) {
            Some(&index) => self.items.get(index).map(|(_, value)| value),
            None => None,
        }
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, Store, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Ensure presence of key
    /// let check = stash.contains_key(&"key");
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

impl<K, V, S> StoreMut<K, V> for Stash<K, V, S>
where
    K: Key,
    S: StoreMut<K, usize>,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash
    /// let mut stash = Stash::default();
    ///
    /// // Insert value
    /// stash.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&index) = self.store.get(&key) {
            Some(mem::replace(&mut self.items[index].1, value))
        } else {
            let index = self.items.insert((key.clone(), value));
            self.store.insert(key, index);
            None
        }
    }

    /// Inserts the value identified by the key if it changed.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash
    /// let mut stash = Stash::default();
    ///
    /// // Insert value
    /// let check = stash.insert_if_changed(&"key", &42);
    /// assert_eq!(check, true);
    ///
    /// // Ignore unchanged value
    /// let check = stash.insert_if_changed(&"key", &42);
    /// assert_eq!(check, false);
    ///
    /// // Update value
    /// let check = stash.insert_if_changed(&"key", &84);
    /// assert_eq!(check, true);
    /// ```
    #[allow(clippy::map_unwrap_or)]
    #[inline]
    fn insert_if_changed(&mut self, key: &K, value: &V) -> bool
    where
        V: Clone + Eq,
    {
        self.store
            .get(key)
            .map(|&index| update_if_changed(&mut self.items[index].1, value))
            .unwrap_or_else(|| {
                let index = self.items.insert((key.clone(), value.clone()));
                self.store.insert(key.clone(), index);
                true
            })
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Remove and return value
    /// let value = StoreMut::remove(&mut stash, &"key");
    /// assert_eq!(value, Some(42));
    /// ```
    #[inline]
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.remove(key).map(|index| {
            let (_, value) = self.items.remove(index);
            value
        })
    }

    /// Removes the value identified by the key and returns both.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Remove and return entry
    /// let entry = stash.remove_entry(&"key");
    /// assert_eq!(entry, Some(("key", 42)));
    /// ```
    #[inline]
    fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.remove(key).map(|index| self.items.remove(index))
    }

    /// Clears the stash, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, Store, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Clear stash
    /// stash.clear();
    /// assert!(stash.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        self.store.clear();
        self.items.clear();
    }
}

impl<K, V, S> StoreMutRef<K, V> for Stash<K, V, S>
where
    K: Key,
    S: StoreMut<K, usize>,
{
    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut, StoreMutRef};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Obtain mutable reference to value
    /// let mut value = stash.get_mut(&"key");
    /// assert_eq!(value, Some(&mut 42));
    /// ```
    #[inline]
    fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        match self.store.get(key) {
            Some(&index) => self.items.get_mut(index).map(|(_, value)| value),
            None => None,
        }
    }

    /// Returns a mutable reference to the value or creates the default.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMutRef};
    ///
    /// // Create stash
    /// let mut stash = Stash::<_, i32>::default();
    ///
    /// // Obtain mutable reference to value
    /// let value = stash.get_or_insert_default(&"key");
    /// assert_eq!(value, &mut 0);
    /// ```
    #[inline]
    fn get_or_insert_default(&mut self, key: &K) -> &mut V
    where
        V: Default,
    {
        if !self.store.contains_key(key) {
            let n = self.items.insert((key.clone(), V::default()));
            self.store.insert(key.clone(), n);
        }

        // We can safely use expect here, as the key is present
        self.get_mut(key).expect("invariant")
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> Index<usize> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, usize>,
{
    type Output = V;

    /// Returns a reference to the value at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterable, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("a", 42);
    /// stash.insert("b", 84);
    ///
    /// // Obtain reference to value
    /// let value = &stash[1];
    /// assert_eq!(value, &84);
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.items[index].1
    }
}

impl<K, V, S> IndexMut<usize> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, usize>,
{
    /// Returns a mutable reference to the value at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterable, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("a", 42);
    /// stash.insert("b", 84);
    ///
    /// // Obtain mutable reference to value
    /// let value = &mut stash[1];
    /// assert_eq!(value, &mut 84);
    /// ```
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.items[index].1
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> FromIterator<(K, V)> for Stash<K, V, S>
where
    K: Key,
    S: StoreMut<K, usize> + Default,
{
    /// Creates a stash from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create a vector of key-value pairs
    /// let items = vec![
    ///     ("a", 1),
    ///     ("b", 2),
    ///     ("c", 3),
    ///     ("d", 4),
    /// ];
    ///
    /// // Create stash from iterator
    /// let stash: Stash<_, _, HashMap<_, _>> =
    ///     items.into_iter().collect();
    ///
    /// // Create iterator over the stash
    /// for (key, value) in &stash {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        let mut store = Stash::new();
        for (key, value) in iter {
            store.insert(key, value);
        }
        store
    }
}

#[allow(clippy::into_iter_without_iter)]
impl<'a, K, V, S> IntoIterator for &'a Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, usize>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    /// Creates an iterator over the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for (key, value) in &stash {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        StoreIterable::iter(&self.items)
    }
}

#[allow(clippy::into_iter_without_iter)]
impl<'a, K, V, S> IntoIterator for &'a mut Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterableMut<K, usize>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    /// Creates a mutable iterator over the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for (key, value) in &mut stash {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        StoreIterableMut::iter_mut(&mut self.items)
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Default for Stash<K, V>
where
    K: Key,
{
    /// Creates a stash with [`HashMap::default`][] as a store.
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
    /// use zrx_store::{Stash, StoreMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> Debug for Stash<K, V, S>
where
    K: Debug + Key,
    V: Debug,
    S: Debug + Store<K, usize>,
{
    /// Formats the stash for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Stash")
            .field("store", &self.store)
            .field("items", &self.items)
            .finish()
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
