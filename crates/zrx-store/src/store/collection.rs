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

//! Collection.

use std::any::Any;
use std::fmt::Debug;

use crate::store::item::{Key, Value};
use crate::store::{Store, StoreIterable, StoreKeys, StoreMut, StoreValues};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Collection.
///
/// This trait combines the following traits, which are not dyn-compatible for
/// reasons of flexibility, into a single trait object that allows to erase the
/// type of the underlying store while preserving the types of the key and value
/// type parameters, `K` and `V`:
///
/// - [`Store`]
/// - [`StoreMut`]
/// - [`StoreIterable`]
/// - [`StoreKeys`]
/// - [`StoreValues`]
///
/// Additionally, implementors must implement [`Any`], so a [`Collection`] can
/// be downcast to an immutable or mutable reference of its concrete type, if
/// necessary and known, as well as [`Debug`], in order to conveniently print
/// the contents of the collection.
///
/// We also provide a blanket implementation for implementors which fulfill all
/// of the aforementioned traits, so that they can be used as a [`Collection`].
pub trait Collection<K, V>: Any + Debug {
    /// Returns a reference to the value identified by the key.
    fn get(&self, key: &K) -> Option<&V>;

    /// Returns whether the collection contains the key.
    fn contains_key(&self, id: &K) -> bool;

    /// Returns the number of items in the collection.
    fn len(&self) -> usize;

    /// Returns whether the collection is empty.
    fn is_empty(&self) -> bool;

    /// Inserts the value identified by the key.
    fn insert(&mut self, key: K, value: V) -> Option<V>;

    /// Removes the value identified by the key.
    fn remove(&mut self, key: &K) -> Option<V>;

    /// Removes the value identified by the key and returns both.
    fn remove_entry(&mut self, key: &K) -> Option<(K, V)>;

    /// Clears the collection, removing all items.
    fn clear(&mut self);

    /// Creates an iterator over the items of the collection.
    fn iter(&self) -> Iter<'_, K, V>;

    /// Creates an iterator over the keys of the collection.
    fn keys(&self) -> Keys<'_, K>;

    /// Creates an iterator over the values of the collection.
    fn values(&self) -> Values<'_, V>;
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V> dyn Collection<K, V> {
    /// Attempts to downcast to a reference of `T`.
    #[inline]
    #[must_use]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Collection<K, V> + Any,
    {
        (self as &dyn Any).downcast_ref()
    }

    /// Attempts to downcast to a mutable reference of `T`.
    #[inline]
    #[must_use]
    pub fn downcast_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Collection<K, V> + Any,
    {
        (self as &mut dyn Any).downcast_mut()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, K, V> IntoIterator for &'a dyn Collection<K, V>
where
    K: Key,
    V: Value,
{
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    /// Creates an iterator over the items of the collection.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<K, V, S> Collection<K, V> for S
where
    K: Key,
    V: Value,
    S: Any + Debug,
    S: Store<K, V>
        + StoreMut<K, V>
        + StoreIterable<K, V>
        + StoreKeys<K, V>
        + StoreValues<K, V>,
{
    /// Returns a reference to the value identified by the key.
    #[inline]
    fn get(&self, id: &K) -> Option<&V> {
        Store::get(self, id)
    }

    /// Returns whether the collection contains the key.
    #[inline]
    fn contains_key(&self, id: &K) -> bool {
        Store::contains_key(self, id)
    }

    /// Returns the number of items in the collection.
    #[inline]
    fn len(&self) -> usize {
        Store::len(self)
    }

    /// Returns whether the collection is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        Store::is_empty(self)
    }

    /// Inserts the value identified by the key.
    #[inline]
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        StoreMut::insert(self, key, value)
    }

    /// Removes the value identified by the key.
    #[inline]
    fn remove(&mut self, key: &K) -> Option<V> {
        StoreMut::remove(self, key)
    }

    /// Removes the value identified by the key and returns both.
    #[inline]
    fn remove_entry(&mut self, key: &K) -> Option<(K, V)> {
        StoreMut::remove_entry(self, key)
    }

    /// Clears the collection, removing all items.
    #[inline]
    fn clear(&mut self) {
        StoreMut::clear(self);
    }

    /// Creates an iterator over the items of the collection.
    #[inline]
    fn iter(&self) -> Iter<'_, K, V> {
        Box::new(StoreIterable::iter(self))
    }

    /// Creates an iterator over the keys of the collection.
    #[inline]
    fn keys(&self) -> Keys<'_, K> {
        Box::new(StoreKeys::keys(self))
    }

    /// Creates an iterator over the values of the collection.
    #[inline]
    fn values(&self) -> Values<'_, V> {
        Box::new(StoreValues::values(self))
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Iterator over the items of a [`Collection`].
pub type Iter<'a, K, V> = Box<dyn Iterator<Item = (&'a K, &'a V)> + 'a>;

/// Iterator over the keys of a [`Collection`].
pub type Keys<'a, K> = Box<dyn Iterator<Item = &'a K> + 'a>;

/// Iterator over the values of a [`Collection`].
pub type Values<'a, V> = Box<dyn Iterator<Item = &'a V> + 'a>;
