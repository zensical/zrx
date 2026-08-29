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

//! Store implementations for [`Slab`].

use slab::Slab;
use std::borrow::Borrow;
use std::mem;

use crate::store::item::{Key, Value};
use crate::store::{Store, StoreMut, StoreMutRef};

mod iter;

pub use iter::{Iter, IterMut, Keys, Values};

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V> Store<K, V> for Slab<(K, V)> {
    /// Returns a reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Obtain reference to value
    /// let value = Store::get(&store, &"key");
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        Slab::iter(self).find_map(|(_, (check, value))| {
            (check.borrow() == key).then_some(value)
        })
    }

    /// Returns whether the store contains the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Ensure presence of key
    /// let check = Store::contains_key(&store, &"key");
    /// assert_eq!(check, true);
    /// ```
    #[inline]
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Key,
    {
        Slab::iter(self).any(|(_, (check, _))| check.borrow() == key)
    }

    /// Returns the number of items in the store.
    #[inline]
    fn len(&self) -> usize {
        Slab::len(self)
    }
}

impl<K, V> StoreMut<K, V> for Slab<(K, V)>
where
    K: Key,
    V: Value,
{
    /// Inserts the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store
    /// let mut store = Slab::default();
    ///
    /// // Insert value
    /// let value = StoreMut::insert(&mut store, "key", 42);
    /// assert_eq!(value, None);
    /// ```
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        let opt = Slab::iter_mut(self).find(|(_, (check, _))| check == &key);
        if let Some((_, (_, prior))) = opt {
            (prior != &value).then(|| mem::replace(prior, value))
        } else {
            self.insert((key, value));
            None
        }
    }

    /// Removes the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Remove and return value
    /// let value = StoreMut::remove(&mut store, &"key");
    /// assert_eq!(value, Some(42));
    /// ```
    #[inline]
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        self.remove_entry(key).map(|(_, value)| value)
    }

    /// Removes the value identified by the key and returns both.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Remove and return entry
    /// let entry = StoreMut::remove_entry(&mut store, &"key");
    /// assert_eq!(entry, Some(("key", 42)));
    /// ```
    #[inline]
    fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        Slab::iter(self)
            .find_map(|(index, (check, _))| {
                (check.borrow() == key).then_some(index)
            })
            .map(|index| self.remove(index))
    }

    /// Clears the store, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{Store, StoreMut};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Remove all items
    /// StoreMut::clear(&mut store);
    /// assert!(Store::is_empty(&store));
    /// ```
    #[inline]
    fn clear(&mut self) {
        Slab::clear(self);
    }
}

impl<K, V> StoreMutRef<K, V> for Slab<(K, V)>
where
    K: Key,
{
    /// Returns a mutable reference to the value identified by the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use slab::Slab;
    /// use zrx_store::{StoreMut, StoreMutRef};
    ///
    /// // Create store and initial state
    /// let mut store = Slab::default();
    /// StoreMut::insert(&mut store, "key", 42);
    ///
    /// // Obtain mutable reference to value
    /// let mut value = StoreMutRef::get_mut(&mut store, &"key");
    /// assert_eq!(value, Some(&mut 42));
    /// ```
    #[inline]
    fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Key,
    {
        Slab::iter_mut(self).find_map(|(_, (check, value))| {
            ((*check).borrow() == key).then_some(value)
        })
    }
}
