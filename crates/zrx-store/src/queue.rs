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

//! Queue.

use ahash::HashMap;
use slab::Slab;
use std::borrow::Borrow;
use std::fmt::{self, Debug};
use std::mem;
use std::time::Instant;

use crate::store::decorator::Ordered;
use crate::store::item::{Key, Value};
use crate::store::{Store, StoreIterable, StoreMut, StoreMutRef};

mod item;
mod iter;

pub use item::Item;
pub use iter::{Iter, Keys, Values};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Queue.
///
/// This is a specialization of [`Store`], more specifically [`Ordered`], that
/// maintains insertion order and allows to assign specific deadlines to items.
/// Values themselves don't need to implement [`Ord`], since the ordering is
/// completely independent and induced by the queue.
///
/// When an item is inserted, it is annotated with [`Instant::now`], which on
/// the one hand implements insertion order, and on the other hand allows to
/// change the ordering of an item through [`Queue::set_deadline`]. This allows
/// to remove items from visibility until a certain point in time.
///
/// Additionally, [`Queue`] is not a queue in the traditional sense, since it
/// adds queueing to a [`Store`], an immutable collection of key-value pairs.
/// The queue is self-organizing, and iterating over it will always yield the
/// correct order of items at that specific point in time.
///
/// # Examples
///
/// ```
/// use zrx_store::{Queue, StoreIterable, StoreMut};
///
/// // Create queue and initial state
/// let mut queue = Queue::default();
/// queue.insert("a", 4);
/// queue.insert("b", 2);
/// queue.insert("c", 3);
/// queue.insert("d", 1);
///
/// // Create iterator over the queue
/// for (key, value) in &queue {
///     println!("{key}: {value}");
/// }
/// ```
#[derive(Clone)]
pub struct Queue<K, V, S = HashMap<K, Item>>
where
    K: Key,
    S: Store<K, Item>,
{
    /// Underlying store.
    store: Ordered<K, Item, S>,
    /// Queue items.
    items: Slab<V>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Queue<K, V, S>
where
    K: Key,
    S: Store<K, Item>,
{
    /// Creates a queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue
    /// let mut queue = Queue::<_, _, HashMap<_, _>>::new();
    /// queue.insert("key", 42);
    /// ```
    #[must_use]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            store: Ordered::new(),
            items: Slab::new(),
        }
    }

    /// Returns the deadline of the item identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Obtain deadline of item
    /// let deadline = queue.get_deadline(&"key");
    /// assert!(deadline < Some(Instant::now()));
    /// ```
    #[inline]
    pub fn get_deadline(&self, key: &K) -> Option<Instant> {
        self.store.get(key).map(Item::deadline)
    }
}

impl<K, V, S> Queue<K, V, S>
where
    K: Key,
    S: StoreMut<K, Item>,
{
    /// Sets the deadline of the item identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Update deadline of item
    /// queue.set_deadline(&"key", Instant::now());
    /// ```
    #[inline]
    pub fn set_deadline(
        &mut self, key: &K, deadline: Instant,
    ) -> Option<Instant> {
        self.store.remove(key).and_then(|mut item| {
            item.set_deadline(deadline);
            self.store
                .insert(key.clone(), item)
                .map(|prior| prior.deadline())
        })
    }
}

impl<K, V, S> Queue<K, V, S>
where
    K: Key,
    S: StoreMut<K, Item> + StoreIterable<K, Item>,
{
    /// Returns the minimum deadline of all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Obtain minimum deadline of all items
    /// let deadline = queue.deadline();
    /// assert!(deadline < Some(Instant::now()));
    ///
    #[inline]
    pub fn deadline(&self) -> Option<Instant> {
        self.store.iter().next().map(|(_, item)| item.deadline())
    }

    /// Takes ownership of the next item that is due.
    ///
    /// Items are considered to be due if [`Instant::now`] has passed the value
    /// stored in [`Item::deadline`], which allows to defer processing.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("a", 4);
    /// queue.insert("b", 2);
    /// queue.insert("c", 3);
    /// queue.insert("d", 1);
    ///
    /// // Obtain items from queue
    /// while let Some((key, value)) = queue.take() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    pub fn take(&mut self) -> Option<(K, V)> {
        // Obtain the current instant once to select due items during iteration,
        // or tight loops might experience slowdowns of up to a factor of 6
        let deadline = Instant::now();
        let opt = self.store.iter().next().and_then(|(key, item)| {
            (item.deadline() <= deadline).then(|| key.clone())
        });

        // Remove and return the first item we found, which is the next item
        // in current queue order that can be considered to be due
        opt.map(|key| {
            // We can safely use expect here, since we're iterating over a
            // store that is synchronized with the ordering
            self.remove(&key)
                .map(|value| (key, value))
                .expect("invariant")
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Store<K, V> for Queue<K, V, S>
where
    K: Key,
    S: Store<K, Item>,
{
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, Store, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Obtain reference to value
    /// let value = queue.get(&"key");
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        match self.store.get(key) {
            Some(item) => self.items.get(*item.data()),
            None => None,
        }
    }

    /// Returns whether the queue contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, Store, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Ensure presence of key
    /// let check = queue.contains_key(&"key");
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

    /// Returns the number of items in the queue.
    #[inline]
    fn len(&self) -> usize {
        self.store.len()
    }
}

impl<K, V, S> StoreMut<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreMut<K, Item>,
{
    /// Inserts the value identified by the key.
    ///
    /// This method only updates the data of the [`Item`], but does not change
    /// the values of [`Item::deadline`] in case the item already exists. The
    /// caller might use [`Queue::insert_if_changed`][] to check, if any of
    /// those values should be changed deliberately.
    ///
    /// [`Queue::insert_if_changed`]: crate::store::StoreMut::insert_if_changed
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue
    /// let mut queue = Queue::default();
    ///
    /// // Insert value
    /// queue.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(item) = self.store.get(&key) {
            let index = *item.data();
            Some(mem::replace(&mut self.items[index], value))
        } else {
            let index = self.items.insert(value);
            self.store.insert(key, Item::new(index));
            None
        }
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Remove and return value
    /// let value = queue.remove(&"key");
    /// assert_eq!(value, Some(42));
    /// ```
    #[inline]
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store
            .remove(key)
            .map(|item| self.items.remove(*item.data()))
    }

    /// Removes the value identified by the key and returns both.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Remove and return entry
    /// let entry = queue.remove_entry(&"key");
    /// assert_eq!(entry, Some(("key", 42)));
    /// ```
    #[inline]
    fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.store
            .remove_entry(key)
            .map(|(key, item)| (key, self.items.remove(*item.data())))
    }

    /// Clears the queue, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, Store, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Clear queue
    /// queue.clear();
    /// assert!(queue.is_empty());
    /// ```
    #[inline]
    fn clear(&mut self) {
        self.store.clear();
        self.items.clear();
    }
}

impl<K, V, S> StoreMutRef<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreMut<K, Item>,
{
    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut, StoreMutRef};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Obtain mutable reference to value
    /// let mut value = queue.get_mut(&"key");
    /// assert_eq!(value, Some(&mut 42));
    /// ```
    #[inline]
    fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        match self.store.get(key) {
            Some(item) => self.items.get_mut(*item.data()),
            None => None,
        }
    }

    /// Returns a mutable reference to the value or creates the default.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMutRef};
    ///
    /// // Create queue
    /// let mut queue = Queue::<_, i32>::default();
    ///
    /// // Obtain mutable reference to value
    /// let value = queue.get_or_insert_default(&"key");
    /// assert_eq!(value, &mut 0);
    /// ```
    #[inline]
    fn get_or_insert_default(&mut self, key: &K) -> &mut V
    where
        V: Default,
    {
        if !self.store.contains_key(key) {
            let index = self.items.insert(V::default());
            self.store.insert(key.clone(), Item::new(index));
        }

        // We can safely use expect here, as the key is present
        self.get_mut(key).expect("invariant")
    }
}

// ----------------------------------------------------------------------------

#[allow(clippy::into_iter_without_iter)]
impl<'a, K, V, S> IntoIterator for &'a Queue<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, Item>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    /// Creates an iterator over the queue.
    ///
    /// The returned iterator is not ordered
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Create iterator over the queue
    /// for (key, value) in &queue {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Default for Queue<K, V>
where
    K: Key,
{
    /// Creates a queue with [`HashMap::default`][] as a store.
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
    /// use zrx_store::{Queue, StoreMut};
    ///
    /// // Create queue
    /// let mut queue = Queue::default();
    ///
    /// // Insert value
    /// queue.insert("key", 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> Debug for Queue<K, V, S>
where
    K: Debug + Key,
    V: Debug,
    S: Debug + Store<K, Item>,
{
    /// Formats the queue for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Queue")
            .field("store", &self.store)
            .field("items", &self.items)
            .finish()
    }
}
