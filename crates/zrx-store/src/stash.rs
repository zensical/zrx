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

//! Generational stash.

use ahash::HashMap;
use std::borrow::Borrow;
use std::fmt::{self, Debug};
use std::mem;
use std::ops::{Index, IndexMut};

use crate::store::item::{Key, Value};
use crate::store::{
    Store, StoreIterable, StoreIterableMut, StoreMut, StoreMutRef,
};

mod iter;
pub mod slab;
pub mod slots;

pub use iter::{Iter, IterMut, Keys, Values};
pub use slab::{Map, Slab, Slot};
pub use slots::{Slots, SlotsMut};

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

/// Generational stash.
///
/// This data type implements a generational stash, which is a key-value store
/// that is optimized for fast insertion and retrieval of items by slots. It's
/// built on a generational [`Slab`], together with a [`Store`] that provides
/// the underlying item storage.
///
/// Iteration follows underlying slab index order, which is not sorted by key.
/// Removed indices may be reused by later insertions. Iteration is cache
/// efficient because it does not look up items in the underlying [`Store`].
/// Store iterator traits return only references to keys and values; use
/// [`Stash::slots`] or [`Stash::slots_mut`] when slots are required.
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
/// // Create iterator over stash
/// for (key, value) in &stash {
///     println!("{key}: {value}");
/// }
/// ```
#[derive(Clone)]
pub struct Stash<K, V, S = HashMap<K, Slot>> {
    /// Underlying store.
    store: S,
    /// Stash items.
    items: Slab<(K, V)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Stash<K, V, S>
where
    K: Key,
    S: Store<K, Slot>,
{
    /// Creates a stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::Stash;
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
        }
    }

    /// Returns the slot of the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slot;
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Obtain slot of value
    /// let slot = stash.get(&"key");
    /// assert_eq!(slot.map(Slot::index), Some(0));
    /// ```
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<Slot>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store.get(key).copied()
    }

    /// Returns a reference to the key in the slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// let slot = stash.insert("key", 42);
    ///
    /// // Obtain key at slot
    /// let key = stash.key(slot);
    /// assert_eq!(key, Some(&"key"));
    /// ```
    #[inline]
    pub fn key(&self, slot: Slot) -> Option<&K> {
        self.items.get(slot).map(|(key, _)| key)
    }
}

impl<K, V, S> Stash<K, V, S>
where
    K: Key,
    S: StoreMut<K, Slot>,
{
    /// Inserts the value identified by the key and returns its slot.
    ///
    /// This method inserts the value and returns a slot that can be used to
    /// retrieve an immutable or mutable reference to the value. If a value
    /// with the same key already exists, the value is replaced, while the
    /// generation of the slot remains stable.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash
    /// let mut stash = Stash::default();
    ///
    /// // Insert value
    /// let slot = stash.insert("key", 42);
    /// assert_eq!(slot.index(), 0);
    /// ```
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Slot {
        if let Some(&slot) = self.store.get(&key) {
            self.items[slot].1 = value;
            slot
        } else {
            let slot = self.items.insert((key.clone(), value));
            self.store.insert(key, slot);
            slot
        }
    }

    /// Removes the entry in the slot and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// let slot = stash.insert("key", 42);
    ///
    /// // Remove and return entry
    /// let entry = stash.remove(slot);
    /// assert_eq!(entry, Some(("key", 42)));
    /// ```
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    pub fn remove(&mut self, slot: Slot) -> Option<(K, V)> {
        self.items.remove(slot).inspect(|(key, _)| {
            self.store.remove(key).expect("invariant");
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Store<K, V> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, Slot>,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, Store};
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
        self.store.get(key).map(|&slot| {
            let (_, value) = &self.items[slot];
            value
        })
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, Store};
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
    V: Value,
    S: StoreMut<K, Slot>,
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
    /// let value = StoreMut::insert(&mut stash, "key", 42);
    /// assert_eq!(value, None);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&slot) = self.store.get(&key) {
            let prior = &mut self.items[slot].1;
            (prior != &value).then(|| mem::replace(prior, value))
        } else {
            let slot = self.items.insert((key.clone(), value));
            self.store.insert(key, slot);
            None
        }
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
        self.store.remove(key).map(|slot| {
            let (_, value) = self.items.remove(slot).expect("invariant");
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
        let opt = self.store.remove(key);
        opt.map(|slot| self.items.remove(slot).expect("invariant"))
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
    /// // Remove all items
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
    S: StoreMut<K, Slot>,
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
        self.store.get(key).map(|&slot| {
            let (_, value) = &mut self.items[slot];
            value
        })
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
            let slot = self.items.insert((key.clone(), V::default()));
            self.store.insert(key.clone(), slot);
        }

        // We can safely use expect here, as the key is present
        self.get_mut(key).expect("invariant")
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> Index<Slot> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, Slot>,
{
    type Output = V;

    /// Returns a reference to the value in the slot.
    ///
    /// # Panics
    ///
    /// Panics if the slot is stale or invalid for this stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("a", 42);
    ///
    /// // Insert value
    /// let slot = stash.insert("b", 84);
    ///
    /// // Obtain reference to value
    /// let value = &stash[slot];
    /// assert_eq!(value, &84);
    /// ```
    #[inline]
    fn index(&self, slot: Slot) -> &Self::Output {
        &self.items[slot].1
    }
}

impl<K, V, S> IndexMut<Slot> for Stash<K, V, S>
where
    K: Key,
    S: Store<K, Slot>,
{
    /// Returns a mutable reference to the value in the slot.
    ///
    /// # Panics
    ///
    /// Panics if the slot is stale or invalid for this stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("a", 42);
    ///
    /// // Insert value
    /// let slot = stash.insert("b", 84);
    ///
    /// // Obtain mutable reference to value
    /// let value = &mut stash[slot];
    /// assert_eq!(value, &mut 84);
    /// ```
    #[inline]
    fn index_mut(&mut self, slot: Slot) -> &mut Self::Output {
        &mut self.items[slot].1
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> FromIterator<(K, V)> for Stash<K, V, S>
where
    K: Key,
    S: StoreMut<K, Slot> + Default,
{
    /// Creates a stash from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::Stash;
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
    /// // Create iterator over stash
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
    S: Store<K, Slot>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    /// Creates an iterator over the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for (key, value) in &stash {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[allow(clippy::into_iter_without_iter)]
impl<'a, K, V, S> IntoIterator for &'a mut Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreMut<K, Slot>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    /// Creates a mutable iterator over the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::Stash;
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over stash
    /// for (key, value) in &mut stash {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
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
    /// use zrx_store::Stash;
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
    K: Debug,
    V: Debug,
    S: Debug,
{
    /// Formats the stash for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Stash")
            .field("store", &self.store)
            .field("items", &self.items)
            .finish()
    }
}
