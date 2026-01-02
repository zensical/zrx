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
use std::time::Instant;
use std::{fmt, mem, ptr};

use crate::store::decorator::Ordered;
use crate::store::{
    Key, Store, StoreIterable, StoreIterableMut, StoreKeys, StoreMut,
    StoreMutRef, StoreValues,
};

mod item;

pub use item::Item;

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
/// use zrx_store::queue::Queue;
/// use zrx_store::{StoreIterable, StoreMut};
///
/// // Create queue and initial state
/// let mut queue = Queue::default();
/// queue.insert("a", 4);
/// queue.insert("b", 2);
/// queue.insert("c", 3);
/// queue.insert("d", 1);
///
/// // Create iterator over the queue
/// for (key, value) in queue.iter() {
///     println!("{key}: {value}");
/// }
/// ```
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
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
        self.store.get(key).cloned().and_then(|mut item| {
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{Store, StoreMut};
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{Store, StoreMut};
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
    /// caller might use [`Queue::insert_if_changed`] to check, if any of those
    /// values should be changed deliberately.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
    ///
    /// // Create queue and insert value
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(item) = self.store.get(&key) {
            let n = *item.data();
            Some(mem::replace(&mut self.items[n], value))
        } else {
            let n = self.items.insert(value);
            self.store.insert(key, Item::new(n));
            None
        }
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
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

    /// Clears the queue, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{Store, StoreMut};
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{StoreMut, StoreMutRef};
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
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMutRef;
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
            let n = self.items.insert(V::default());
            self.store.insert(key.clone(), Item::new(n));
        }

        // We can safely use expect here, as the key is present
        self.get_mut(key).expect("invariant")
    }
}

impl<K, V, S> StoreIterable<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreIterable<K, Item>,
{
    /// Creates an iterator over the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{StoreIterable, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Create iterator over the queue
    /// for (key, value) in queue.iter() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a V)>
    where
        K: 'a,
        V: 'a,
    {
        // Obtain the current instant once to select due items during iteration,
        // or tight loops might experience slowdowns of up to a factor of 6
        let deadline = Instant::now();
        let iter = self.store.iter();
        iter.take_while(move |(_, item)| item.deadline() <= deadline)
            .map(|(key, item)| (key, &self.items[*item.data()]))
    }
}

impl<K, V, S> StoreIterableMut<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreMut<K, Item> + StoreIterable<K, Item>,
{
    /// Creates a mutable iterator over the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{StoreIterableMut, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Create iterator over the queue
    /// for (key, value) in queue.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = (&'a K, &'a mut V)>
    where
        K: 'a,
        V: 'a,
    {
        // Obtain a mutable pointer to the queue items, as we need to reference
        // it in the closure passed to the iterator's map method
        let items = ptr::addr_of_mut!(self.items);

        // Obtain the current instant once to select due items during iteration,
        // or tight loops might experience slowdowns of up to a factor of 6
        let deadline = Instant::now();
        let iter = self.store.iter();
        iter.take_while(move |(_, item)| item.deadline() <= deadline)
            .map(move |(key, item)| {
                // SAFETY: The borrow checker won't let us return a mutable
                // reference to an item in the slab, but we know this is safe,
                // as the store and the slab are two distinct data structures
                // that are synchronized with each other
                (key, unsafe { &mut (&mut *items)[*item.data()] })
            })
    }
}

impl<K, V, S> StoreKeys<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreIterable<K, Item>,
{
    /// Creates a key iterator over the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{StoreKeys, StoreMut};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Create iterator over the queue
    /// for key in queue.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys<'a>(&'a self) -> impl Iterator<Item = &'a K>
    where
        K: 'a,
    {
        // Obtain the current instant once to select due items during iteration,
        // or tight loops might experience slowdowns of up to a factor of 6
        let deadline = Instant::now();
        let iter = self.store.iter();
        iter.take_while(move |(_, item)| item.deadline() <= deadline)
            .map(|(key, _)| key)
    }
}

impl<K, V, S> StoreValues<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreIterable<K, Item>,
{
    /// Creates a value iterator over the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::{StoreMut, StoreValues};
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    ///
    /// // Create iterator over the queue
    /// for value in queue.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values<'a>(&'a self) -> impl Iterator<Item = &'a V>
    where
        V: 'a,
    {
        // Obtain the current instant once to select due items during iteration,
        // or tight loops might experience slowdowns of up to a factor of 6
        let deadline = Instant::now();
        let iter = self.store.iter();
        iter.take_while(move |(_, item)| item.deadline() <= deadline)
            .map(|(_, item)| &self.items[*item.data()])
    }
}

// ----------------------------------------------------------------------------

#[allow(clippy::implicit_hasher)]
impl<K, V> Default for Queue<K, V, HashMap<K, Item>>
where
    K: Key,
{
    /// Creates a queue with [`HashMap::default`] as a store.
    ///
    /// Note that this method does not allow to customize the [`BuildHasher`][],
    /// but uses [`ahash`] by default, which is the fastest known hasher.
    ///
    /// [`BuildHasher`]: std::hash::BuildHasher
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Queue;
    /// use zrx_store::StoreMut;
    ///
    /// // Create queue and initial state
    /// let mut queue = Queue::default();
    /// queue.insert("key", 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> fmt::Debug for Queue<K, V, S>
where
    K: Key + fmt::Debug,
    V: fmt::Debug,
    S: Store<K, Item> + fmt::Debug,
{
    /// Formats the queue for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Queue")
            .field("store", &self.store)
            .field("items", &self.items)
            .finish()
    }
}
