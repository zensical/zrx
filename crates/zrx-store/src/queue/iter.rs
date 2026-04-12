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

//! Iterator implementations for [`Queue`].

use slab::Slab;
use std::ptr;
use std::time::Instant;

use crate::store::decorator::ordered;
use crate::store::item::{Key, Value};
use crate::store::{
    StoreIterable, StoreIterableMut, StoreKeys, StoreMut, StoreValues,
};

use super::item::Item;
use super::Queue;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a [`Queue`].
#[derive(Debug)]
pub struct Iter<'a, K, V>
where
    K: Key,
{
    /// Inner iterator.
    inner: ordered::Iter<'a, K, Item>,
    /// Queue items.
    items: &'a Slab<V>,
    /// Cutoff deadline.
    deadline: Instant,
}

/// Mutable iterator over the items of a [`Queue`].
pub struct IterMut<'a, K, V>
where
    K: Key,
{
    /// Inner iterator.
    inner: ordered::Iter<'a, K, Item>,
    /// Queue items.
    items: &'a mut Slab<V>,
    /// Cutoff deadline.
    deadline: Instant,
}

/// Iterator over the keys of a [`Queue`].
pub struct Keys<'a, K>
where
    K: Key,
{
    /// Inner iterator.
    inner: ordered::Iter<'a, K, Item>,
    /// Cutoff deadline.
    deadline: Instant,
}

/// Iterator over the values of a [`Queue`].
pub struct Values<'a, K, V>
where
    K: Key,
{
    /// Inner iterator.
    inner: ordered::Values<'a, K, Item>,
    /// Queue items.
    items: &'a Slab<V>,
    /// Cutoff deadline.
    deadline: Instant,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> StoreIterable<K, V> for Queue<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, Item>,
{
    type Iter<'a> = Iter<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the items of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreIterable, StoreMut};
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
    fn iter(&self) -> Self::Iter<'_> {
        Iter {
            inner: self.store.iter(),
            items: &self.items,
            deadline: Instant::now(),
        }
    }
}

impl<K, V, S> StoreIterableMut<K, V> for Queue<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreMut<K, Item> + StoreIterable<K, Item>,
{
    type IterMut<'a> = IterMut<'a, K, V>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreIterableMut, StoreMut};
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
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        IterMut {
            inner: self.store.iter(),
            items: &mut self.items,
            deadline: Instant::now(),
        }
    }
}

impl<K, V, S> StoreKeys<K, V> for Queue<K, V, S>
where
    K: Key,
    S: StoreIterable<K, Item>,
{
    type Keys<'a> = Keys<'a, K>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreKeys, StoreMut};
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
    fn keys(&self) -> Self::Keys<'_> {
        Keys {
            inner: self.store.iter(),
            deadline: Instant::now(),
        }
    }
}

impl<K, V, S> StoreValues<K, V> for Queue<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreValues<K, Item>,
{
    type Values<'a> = Values<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the queue.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Queue, StoreMut, StoreValues};
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
    fn values(&self) -> Self::Values<'_> {
        Values {
            inner: self.store.values(),
            items: &self.items,
            deadline: Instant::now(),
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Key,
{
    type Item = (&'a K, &'a V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find_map(|(key, item)| {
            (item.deadline() <= self.deadline)
                .then(|| (key, &self.items[*item.data()]))
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for IterMut<'a, K, V>
where
    K: Key,
{
    type Item = (&'a K, &'a mut V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Obtain a mutable pointer to the queue items, as we need to reference
        // it in the closure passed to the iterator's map method
        let items = ptr::addr_of_mut!(*self.items);
        self.inner.find_map(|(key, item)| {
            (item.deadline() <= self.deadline)
                // SAFETY: The borrow checker won't let us return a mutable
                // reference to an item in the slab, but we know this is safe,
                // as the store and the slab are two distinct data structures
                // that are synchronized with each other
                .then(|| (key, unsafe { &mut (&mut *items)[*item.data()] }))
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K> Iterator for Keys<'a, K>
where
    K: Key,
{
    type Item = &'a K;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find_map(|(key, item)| {
            (item.deadline() <= self.deadline).then_some(key)
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

// ----------------------------------------------------------------------------

impl<'a, K, V> Iterator for Values<'a, K, V>
where
    K: Key,
{
    type Item = &'a V;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.find_map(|item| {
            (item.deadline() <= self.deadline)
                .then(|| &self.items[*item.data()])
        })
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
