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

//! Store entry.

use super::item::{Key, Value};

// -----------------------------------------------------------------------------
// Traits
// -----------------------------------------------------------------------------

/// Occupied store entry.
pub trait OccupiedEntry<'a, K, V>
where
    K: Key,
    V: Value,
{
    /// Returns a reference to the key.
    fn key(&self) -> &K;

    /// Returns a reference to the value.
    fn get(&self) -> &V;

    /// Returns a mutable reference to the value.
    fn get_mut(&mut self) -> &mut V;

    /// Returns a mutable reference to the value, consuming the entry.
    fn into_mut(self) -> &'a mut V;

    /// Inserts and returns the value.
    fn insert(&mut self, value: V) -> V;

    /// Removes and returns the value.
    fn remove(self) -> V;

    /// Removes and returns the key and value.
    fn remove_entry(self) -> (K, V);
}

/// Vacant store entry.
pub trait VacantEntry<'a, K, V>
where
    K: Key,
    V: Value,
{
    /// Returns the key that would be used when inserting a value.
    fn key(&self) -> &K;

    /// Returns the key that would be used when inserting a value.
    fn into_key(self) -> K;

    /// Inserts the value and returns a mutable reference to it.
    fn insert(self, value: V) -> &'a mut V;
}

// -----------------------------------------------------------------------------
// Enums
// -----------------------------------------------------------------------------

/// Store entry.
///
/// This enum is used to represent an entry in a [`Store`][], which can either
/// be occupied or vacant. It provides methods to access and modify the entry,
/// as well as to insert new values when the entry is vacant.
///
/// [`Store`]: crate::store::Store
#[must_use]
pub enum Entry<O, E> {
    /// Occupied entry.
    Occupied(O),
    /// Vacant entry.
    Vacant(E),
}

// -----------------------------------------------------------------------------
// Implementations
// -----------------------------------------------------------------------------

impl<O, E> Entry<O, E> {
    /// Returns the key for the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{StoreEntry, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Obtain key for entry
    /// let entry = StoreEntry::entry(&mut store, "key");
    /// assert_eq!(entry.key(), &"key");
    /// ```
    #[inline]
    pub fn key<'a, K, V>(&self) -> &K
    where
        O: OccupiedEntry<'a, K, V>,
        E: VacantEntry<'a, K, V>,
        K: Key,
        V: Value,
    {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }

    /// Modifies the value if occupied and returns the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::{StoreEntry, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Modify value
    /// let value = StoreEntry::entry(&mut store, "key")
    ///     .and_modify(|value| *value *= 2)
    ///     .or_insert(0);
    /// assert_eq!(value, &84);
    /// ```
    #[inline]
    pub fn and_modify<'a, F, K, V>(mut self, f: F) -> Self
    where
        O: OccupiedEntry<'a, K, V>,
        E: VacantEntry<'a, K, V>,
        F: FnOnce(&mut V),
        K: Key,
        V: Value,
    {
        if let Self::Occupied(entry) = &mut self {
            f(entry.get_mut());
        }
        self
    }

    /// Inserts the given value if vacant.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreEntry;
    ///
    /// // Create store
    /// let mut store = HashMap::new();
    ///
    /// // Insert value when vacant
    /// let value = StoreEntry::entry(&mut store, "key")
    ///     .or_insert(42);
    /// assert_eq!(value, &42);
    /// ```
    #[inline]
    pub fn or_insert<'a, K, V>(self, value: V) -> &'a mut V
    where
        O: OccupiedEntry<'a, K, V>,
        E: VacantEntry<'a, K, V>,
        K: Key,
        V: Value,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(value),
        }
    }

    /// Inserts the value returned by the function if vacant.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreEntry;
    ///
    /// // Create store
    /// let mut store = HashMap::new();
    ///
    /// // Insert value when vacant
    /// let value = StoreEntry::entry(&mut store, "key")
    ///     .or_insert_with(|| 42);
    /// assert_eq!(value, &42);
    /// ```
    #[inline]
    pub fn or_insert_with<'a, F, K, V>(self, f: F) -> &'a mut V
    where
        O: OccupiedEntry<'a, K, V>,
        E: VacantEntry<'a, K, V>,
        F: FnOnce() -> V,
        K: Key,
        V: Value,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(f()),
        }
    }

    /// Inserts the value returned by the function receiving the key if vacant.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreEntry;
    ///
    /// // Create store
    /// let mut store = HashMap::new();
    ///
    /// // Create value from key when vacant
    /// let value = StoreEntry::entry(&mut store, "key")
    ///     .or_insert_with_key(|key| key.len());
    /// assert_eq!(value, &3);
    /// ```
    #[inline]
    pub fn or_insert_with_key<'a, F, K, V>(self, f: F) -> &'a mut V
    where
        O: OccupiedEntry<'a, K, V>,
        E: VacantEntry<'a, K, V>,
        F: FnOnce(&K) -> V,
        K: Key,
        V: Value,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => {
                let value = f(entry.key());
                entry.insert(value)
            }
        }
    }

    /// Inserts the default value if vacant.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::StoreEntry;
    ///
    /// // Create store
    /// let mut store = HashMap::<_, u64>::new();
    ///
    /// // Create default value when vacant
    /// let value = StoreEntry::entry(&mut store, "key").or_default();
    /// assert_eq!(value, &0);
    /// ```
    #[inline]
    pub fn or_default<'a, K, V>(self) -> &'a mut V
    where
        O: OccupiedEntry<'a, K, V>,
        E: VacantEntry<'a, K, V>,
        K: Key,
        V: Value + Default,
    {
        self.or_insert_with(V::default)
    }
}
