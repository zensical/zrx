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

//! Generational slab map.

use std::mem;
use std::num::NonZeroUsize;

use crate::store::entry::Key;

use super::slot::Slot;

mod into_iter;
mod iter;

pub use into_iter::IntoIter;
pub use iter::{Iter, IterMut, Keys, Values};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Generational slab map.
///
/// The map associates side data with a key that embeds a [`Slot`]. Its sparse
/// index gives constant-time access by slab index, while dense entries retain
/// the complete key and therefore reject stale generations. Iteration visits
/// only live entries; removal uses swap-remove and may change iteration order.
#[derive(Clone, Debug)]
pub struct Map<V, K = Slot>
where
    K: Key + AsRef<Slot>,
{
    /// Underlying vector.
    inner: Vec<Entry<K, V>>,
    /// Mapping from index to dense position.
    positions: Vec<Option<NonZeroUsize>>,
}

// ----------------------------------------------------------------------------

/// Generational slab map entry.
#[derive(Clone, Debug)]
struct Entry<K, V> {
    /// Entry key.
    key: K,
    /// Entry value.
    value: V,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<V, K> Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    /// Creates a generational slab map.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::new();
    /// map.insert(slot, 42);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Obtain reference to value
    /// let value = map.get(&slot);
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, key: &K) -> Option<&V> {
        let position = self.position(key)?;
        Some(&self.inner[position].value)
    }

    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Obtain reference to value
    /// let value = map.get_mut(&slot);
    /// assert_eq!(value, Some(&mut 42));
    /// ```
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let position = self.position(key)?;
        Some(&mut self.inner[position].value)
    }

    /// Returns whether the map contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    ///
    /// // Ensure presence of value
    /// let check = map.contains_key(&slot);
    /// assert_eq!(check, true);
    /// ```
    #[inline]
    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool {
        self.position(key).is_some()
    }

    /// Inserts or replaces the value for an exact key.
    ///
    /// # Panics
    ///
    /// Panics if another key occupies the same slot. This can happen if the
    /// key is stale or if the key was constructed manually, and indicates an
    /// invariant violation. The caller should ensure that the key is valid and
    /// unique before calling this method.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::default();
    ///
    /// // Insert value
    /// map.insert(slot, 42);
    /// ```
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let index = key.as_ref().index();
        if let Some(position) = self.resolve(index) {
            let entry = &mut self.inner[position];
            assert!(entry.key == key, "invariant");

            // Insert and return prior value
            return Some(mem::replace(&mut entry.value, value));
        }

        // Grow vector if necessary to accommodate the new index
        if self.positions.len() <= index {
            self.positions.resize(index + 1, None);
        }

        // Insert new entry and associate it with the key's index
        self.inner.push(Entry { key, value });
        self.positions[index] = Some(encode(self.inner.len() - 1));
        None
    }

    /// Removes and returns the value for an exact key.
    ///
    /// # Panics
    ///
    /// Panics if another key occupies the same slot. This can happen if the
    /// key is stale or if the key was constructed manually, and indicates an
    /// invariant violation. The caller should ensure that the key is valid and
    /// unique before calling this method.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let a = slab.insert("a");
    /// let b = slab.insert("b");
    ///
    /// // Create associated map
    /// let mut map = Map::default();
    /// map.insert(a, 42);
    /// map.insert(b, 84);
    ///
    /// // Remove and return value
    /// let value = map.remove(&a);
    /// assert_eq!(value, Some(42));
    ///
    /// // Obtain reference to value
    /// let value = map.get(&b);
    /// assert_eq!(value, Some(&84));
    /// ```
    #[inline]
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = key.as_ref().index();
        self.resolve(index).map(|position| {
            let entry = &self.inner[position];
            assert!(&entry.key == key, "invariant");

            // Remove index association for the removed entry
            self.positions[index] = None;

            // Remove entry and update index association for swapped entry
            let Entry { value, .. } = self.inner.swap_remove(position);
            if let Some(entry) = self.inner.get(position) {
                let index = entry.key.as_ref().index();
                self.positions[index] = Some(encode(position));
            }

            // Return prior value
            value
        })
    }

    /// Clears the map, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::new();
    /// map.insert(slot, 42);
    ///
    /// // Remove all items
    /// map.clear();
    /// assert!(map.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
        self.positions.clear();
    }

    /// Returns the position of the given key.
    fn position(&self, key: &K) -> Option<usize> {
        let position = self.resolve(key.as_ref().index())?;
        (&self.inner[position].key == key).then_some(position)
    }

    /// Returns the position of the given index.
    fn resolve(&self, index: usize) -> Option<usize> {
        self.positions.get(index).copied().flatten().map(decode)
    }
}

#[allow(clippy::must_use_candidate)]
impl<V, K> Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    /// Returns the number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, K, V> IntoIterator for &'a Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    /// Creates an iterator over the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::new();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for (slot, value) in &map {
    ///     println!("[{slot}]: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> IntoIterator for &'a mut Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;

    /// Creates a mutable iterator over the map.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::new();
    /// map.insert(slot, 42);
    ///
    /// // Create iterator over map
    /// for (slot, value) in &mut map {
    ///     println!("[{slot}]: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// ----------------------------------------------------------------------------

impl<V, K> Default for Map<V, K>
where
    K: Key + AsRef<Slot>,
{
    /// Creates a generational slab map.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::{Map, Slab};
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    /// let slot = slab.insert("key");
    ///
    /// // Create slab map
    /// let mut map = Map::default();
    /// map.insert(slot, 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self {
            inner: Vec::default(),
            positions: Vec::default(),
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Encodes a dense to sparse index.
#[inline]
fn encode(index: usize) -> NonZeroUsize {
    let index = index.checked_add(1).expect("invariant");
    NonZeroUsize::new(index).expect("invariant")
}

/// Decodes a sparse to dense index.
#[inline]
fn decode(index: NonZeroUsize) -> usize {
    index.get() - 1
}
