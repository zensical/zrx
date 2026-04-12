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

//! Store traits.

use std::borrow::Borrow;
use std::ops::RangeBounds;

pub mod adapter;
pub mod collection;
pub mod comparator;
pub mod decorator;
pub mod item;

use comparator::Comparator;
use item::{Key, Value};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Immutable store.
///
/// This trait defines the required methods for an immutable key-value store,
/// abstracting over implementations like [`HashMap`][] and [`BTreeMap`][]. It
/// forms the foundation for a set of further traits that define complementary
/// capabilities for stores, like [`StoreMut`] and [`StoreIterable`].
///
/// There are several of those traits, all of which can be composed in trait
/// bounds to require specific store capabilities. These are:
///
/// - [`StoreMut`]: Mutable store
/// - [`StoreMutRef`]: Mutable store that can return mutable references
/// - [`StoreIterable`]: Immutable store that is iterable
/// - [`StoreIterableMut`]: Mutable store that is iterable
/// - [`StoreKeys`]: Immutable store that is iterable over its keys
/// - [`StoreValues`]: Immutable store that is iterable over its values
/// - [`StoreRange`]: Immutable store that is iterable over a range
///
/// This trait is implemented for [`HashMap`][] and [`BTreeMap`][], as well as
/// all of the store [`decorators`][] that allow to wrap stores with additional
/// capabilities. Note that stores are not thread-safe, so they can't be shared
/// among worker threads.
///
/// All methods deliberately have [`Infallible`] signatures, as stores must be
/// fast and reliable, and should never fail under normal circumstances. Stores
/// should not need to serialize data, write to the filesystem, or send data
/// over the network. They should only have in-memory representations.
///
/// [`decorators`]: crate::store::decorator
/// [`BTreeMap`]: std::collections::BTreeMap
/// [`HashMap`]: std::collections::HashMap
/// [`Infallible`]: std::convert::Infallible
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::StoreMut;
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Obtain reference to value
/// let value = store.get(&"key");
/// assert_eq!(value, Some(&42));
/// ```
pub trait Store<K, V>
where
    K: Key,
{
    /// Returns a reference to the value identified by the key.
    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Key;

    /// Returns whether the store contains the key.
    fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Key;

    /// Returns the number of items in the store.
    fn len(&self) -> usize;

    /// Returns whether the store is empty.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Mutable store.
///
/// This trait extends [`Store`], requiring further additional mutable methods
/// which can be used to alter the store, like inserting and removing items.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::StoreMut;
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Remove value from store
/// let value = store.remove(&"key");
/// assert_eq!(value, Some(42));
/// ```
pub trait StoreMut<K, V>: Store<K, V>
where
    K: Key,
{
    /// Inserts the value identified by the key.
    fn insert(&mut self, key: K, value: V) -> Option<V>;

    /// Removes the value identified by the key.
    fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Key;

    /// Removes the value identified by the key and returns both.
    fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        K: Borrow<Q>,
        Q: Key;

    /// Clears the store, removing all items.
    fn clear(&mut self);
}

/// Mutable store that can return mutable references.
///
/// This trait extends [`StoreMut`], adding the possibility to obtain mutable
/// references as a requirement, so values can be mutated in-place.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::{StoreMut, StoreMutRef};
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Obtain mutable reference to value
/// let value = store.get_mut(&"key");
/// assert_eq!(value, Some(&mut 42));
/// ```
pub trait StoreMutRef<K, V>: Store<K, V>
where
    K: Key,
{
    /// Returns a mutable reference to the value identified by the key.
    fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Key;

    /// Returns a mutable reference to the value or creates the default.
    fn get_or_insert_default(&mut self, key: &K) -> &mut V
    where
        V: Default;
}

/// Immutable store that is iterable.
///
/// This trait extends [`Store`], adding iteration capabilities as a further
/// requirement, so a store can enumerate its items.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::{StoreIterable, StoreMut};
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Create iterator over the store
/// for (key, value) in store.iter() {
///     println!("{key}: {value}");
/// }
/// ```
pub trait StoreIterable<K, V>: Store<K, V>
where
    K: Key,
    V: Value,
{
    type Iter<'a>: Iterator<Item = (&'a K, &'a V)>
    where
        Self: 'a;

    /// Creates an iterator over the items of the store.
    fn iter(&self) -> Self::Iter<'_>;
}

/// Mutable store that is iterable.
///
/// This trait extends [`StoreMut`], adding mutable iteration capabilities as a
/// requirement, so a store can enumerate its items mutably.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::{StoreIterableMut, StoreMut};
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Create iterator over the store
/// for (key, value) in store.iter_mut() {
///     println!("{key}: {value}");
/// }
/// ```
pub trait StoreIterableMut<K, V>: StoreMut<K, V>
where
    K: Key,
    V: Value,
{
    type IterMut<'a>: Iterator<Item = (&'a K, &'a mut V)>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the store.
    fn iter_mut(&mut self) -> Self::IterMut<'_>;
}

/// Immutable store that is iterable over its keys.
///
/// This trait extends [`Store`], adding key iteration capabilities as a
/// requirement, so a store can enumerate its keys.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::{StoreKeys, StoreMut};
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Create iterator over the store
/// for key in store.keys() {
///     println!("{key}");
/// }
/// ```
pub trait StoreKeys<K, V>: Store<K, V>
where
    K: Key,
{
    type Keys<'a>: Iterator<Item = &'a K>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the store.
    fn keys(&self) -> Self::Keys<'_>;
}

/// Immutable store that is iterable over its values.
///
/// This trait extends [`Store`], adding value iteration capabilities as a
/// requirement, so a store can enumerate its values.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::{StoreMut, StoreValues};
///
/// // Create store and initial state
/// let mut store = HashMap::new();
/// store.insert("key", 42);
///
/// // Create iterator over the store
/// for value in store.values() {
///     println!("{value}");
/// }
/// ```
pub trait StoreValues<K, V>: Store<K, V>
where
    K: Key,
    V: Value,
{
    type Values<'a>: Iterator<Item = &'a V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the store.
    fn values(&self) -> Self::Values<'_>;
}

/// Immutable store that is iterable over a range.
///
/// This trait extends [`Store`], adding iteration capabilities as a further
/// requirement, so a store can enumerate its items over a given range.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use zrx_store::{StoreMut, StoreRange};
///
/// // Create store and initial state
/// let mut store = BTreeMap::new();
/// store.insert("a", 42);
/// store.insert("b", 84);
///
/// // Create iterator over the store
/// for (key, value) in store.range("b"..) {
///     println!("{key}: {value}");
/// }
/// ```
pub trait StoreRange<K, V>: Store<K, V>
where
    K: Key,
    V: Value,
{
    type Range<'a>: Iterator<Item = (&'a K, &'a V)>
    where
        Self: 'a;

    /// Creates an iterator over a range of items of the store.
    fn range<R>(&self, range: R) -> Self::Range<'_>
    where
        R: RangeBounds<K>;
}

// ----------------------------------------------------------------------------

/// Creates a store with a comparator.
///
/// This trait extends [`Store`], adding the capability to create a store with
/// a custom comparator, allowing to customize the ordering of values.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::comparator::Descending;
/// use zrx_store::decorator::Ordered;
/// use zrx_store::{StoreMut, StoreWithComparator};
///
/// // Create store
/// let mut store: Ordered::<_, _, HashMap<_, _>, _> =
///     Ordered::with_comparator(Descending);
///
/// // Insert value
/// store.insert("key", 42);
/// ```
pub trait StoreWithComparator<K, V, C>: Store<K, V>
where
    K: Key,
    C: Comparator<V>,
{
    /// Creates a store with the given comparator.
    fn with_comparator(comparator: C) -> Self;
}

// ----------------------------------------------------------------------------

/// Creates a store from an iterator.
pub trait StoreFromIterator<K, V>: FromIterator<(K, V)> {}

/// Creates an iterator over the items of the store.
pub trait StoreIntoIterator<K, V>: IntoIterator<Item = (K, V)> {}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

#[rustfmt::skip]
impl<K, V, T> StoreFromIterator<K, V> for T
where
    T: FromIterator<(K, V)> {}

#[rustfmt::skip]
impl<K, V, T> StoreIntoIterator<K, V> for T
where
    T: IntoIterator<Item = (K, V)> {}
