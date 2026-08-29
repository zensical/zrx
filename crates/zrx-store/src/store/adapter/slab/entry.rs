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

//! Entry implementations for [`Slab`].

use slab::Slab;
use std::mem;

use crate::store::StoreEntry;
use crate::store::entry;
use crate::store::item::{Key, Value};

// -----------------------------------------------------------------------------
// Structs
// -----------------------------------------------------------------------------

/// Occupied slab entry.
pub struct OccupiedEntry<'a, K, V> {
    /// Slab containing the entry.
    slab: &'a mut Slab<(K, V)>,
    /// Slab index.
    index: usize,
}

/// Vacant slab entry.
pub struct VacantEntry<'a, K, V> {
    /// Vacant slab entry.
    entry: slab::VacantEntry<'a, (K, V)>,
    /// Key of the entry.
    key: K,
}

// -----------------------------------------------------------------------------
// Trait implementations
// -----------------------------------------------------------------------------

impl<K, V> StoreEntry<K, V> for Slab<(K, V)>
where
    K: Key,
    V: Value,
{
    type Occupied<'a>
        = OccupiedEntry<'a, K, V>
    where
        Self: 'a;
    type Vacant<'a>
        = VacantEntry<'a, K, V>
    where
        Self: 'a;

    /// Returns the entry for the given key.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::entry::Entry;
    /// use zrx_store::{StoreEntry, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Obtain entry for value
    /// let entry = StoreEntry::entry(&mut store, "key");
    /// assert!(matches!(entry, Entry::Occupied(_)));
    /// ```
    #[inline]
    fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        let index = Slab::iter(self)
            .find_map(|(index, (check, _))| (check == &key).then_some(index));
        if let Some(index) = index {
            Entry::Occupied(OccupiedEntry { slab: self, index })
        } else {
            let entry = Slab::vacant_entry(self);
            Entry::Vacant(VacantEntry { entry, key })
        }
    }
}

// -----------------------------------------------------------------------------

impl<'a, K, V> entry::OccupiedEntry<'a, K, V> for OccupiedEntry<'a, K, V>
where
    K: Key,
    V: Value,
{
    /// Returns a reference to the key.
    #[inline]
    fn key(&self) -> &K {
        &self.slab[self.index].0
    }

    /// Returns a reference to the value.
    #[inline]
    fn get(&self) -> &V {
        &self.slab[self.index].1
    }

    /// Returns a mutable reference to the value.
    #[inline]
    fn get_mut(&mut self) -> &mut V {
        &mut self.slab[self.index].1
    }

    /// Returns a mutable reference to the value, consuming the entry.
    #[inline]
    fn into_mut(self) -> &'a mut V {
        &mut self.slab[self.index].1
    }

    /// Inserts the value if different and returns the previous value.
    #[inline]
    fn insert(&mut self, value: V) -> Option<V> {
        (self.slab[self.index].1 != value)
            .then(|| mem::replace(&mut self.slab[self.index].1, value))
    }

    /// Removes and returns the value.
    #[inline]
    fn remove(self) -> V {
        self.slab.remove(self.index).1
    }

    /// Removes and returns the key and value.
    #[inline]
    fn remove_entry(self) -> (K, V) {
        self.slab.remove(self.index)
    }
}

impl<'a, K, V> entry::VacantEntry<'a, K, V> for VacantEntry<'a, K, V>
where
    K: Key,
    V: Value,
{
    /// Returns the key that would be used when inserting a value.
    #[inline]
    fn key(&self) -> &K {
        &self.key
    }

    /// Returns the key that would be used when inserting a value.
    #[inline]
    fn into_key(self) -> K {
        self.key
    }

    /// Inserts the value and returns a mutable reference to it.
    #[inline]
    fn insert(self, value: V) -> &'a mut V {
        &mut self.entry.insert((self.key, value)).1
    }
}

// -----------------------------------------------------------------------------
// Type aliases
// -----------------------------------------------------------------------------

/// Entry type for [`Slab`].
pub type Entry<'a, K, V> = entry::Entry<
    OccupiedEntry<'a, K, V>, // fmt
    VacantEntry<'a, K, V>,
>;
