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

//! Storage.

use ahash::HashMap;
use std::ops::{Deref, DerefMut};

use zrx_store::{Collection, Key, Value};

pub mod accessor;
pub mod borrow;
pub mod convert;
mod error;
pub mod set;

pub use error::{Error, Result};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Storage.
///
/// Fundamentally, storages are solely thin wrappers around implementors of the
/// [`Collection`] trait, which allows to dereference or downcast to a concrete
/// type, if known, both, immutably and mutably. [`Collection`] is a convenience
/// trait, is dyn-compatible, and combines several of the [`Store`][] traits.
///
/// While [`zrx-store`][] implements generic key-value stores, [`zrx-storage`][]
/// focuses on providing abstractions and utilities for orchestrating immutable
/// and mutable access across multiple key-value stores with varying semantics,
/// encapsulating boilerplate, and making it easy to write reusable operators.
///
/// [`Store`]: zrx_store::Store
/// [`zrx-store`]: zrx_store
/// [`zrx-storage`]: crate
///
/// # Examples
///
/// ```
/// use zrx_storage::Storage;
///
/// // Create storage and initial state
/// let mut storage = Storage::default();
/// storage.insert("a", 4);
/// storage.insert("b", 2);
/// storage.insert("c", 3);
/// storage.insert("d", 1);
///
/// // Create iterator over the storage
/// for (key, value) in &*storage {
///     println!("{key}: {value}");
/// }
/// ```
#[derive(Debug)]
pub struct Storage<K, V> {
    /// Inner collection.
    inner: Box<dyn Collection<K, V>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K, V> Storage<K, V> {
    /// Creates a new storage from the given collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_storage::Storage;
    ///
    /// // Create storage
    /// let mut storage = Storage::new(HashMap::new());
    ///
    /// // Insert value
    /// storage.insert("key", 42);
    /// ```
    #[must_use]
    pub fn new<T>(collection: T) -> Self
    where
        T: Collection<K, V>,
    {
        Self { inner: Box::new(collection) }
    }

    /// Attempts to downcast to a mutable reference of `T`.
    ///
    /// This method is a wrapper around the method with the same name that is
    /// implemented for [`Collection`] trait objects, and is primarily provided
    /// for convenience, so failed downcasts return a [`Result`] instead of an
    /// [`Option`], turning them into errors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Downcast`] if conversion fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::collections::HashMap;
    /// use zrx_storage::Storage;
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::new(HashMap::new());
    /// storage.insert("key", 42);
    ///
    /// // Downcast storage to inner collection
    /// let store = storage.downcast_ref::<HashMap<_, _>>()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn downcast_ref<T>(&self) -> Result<&T>
    where
        T: Collection<K, V>,
    {
        self.inner.downcast_ref().ok_or(Error::Downcast)
    }

    /// Attempts to downcast to a mutable reference of `T`.
    ///
    /// This method is a wrapper around the method with the same name that is
    /// implemented for [`Collection`] trait objects, and is primarily provided
    /// for convenience, so failed downcasts return a [`Result`] instead of an
    /// [`Option`], turning them into errors.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Downcast`] if conversion fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::collections::HashMap;
    /// use zrx_storage::Storage;
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::new(HashMap::new());
    /// storage.insert("key", 42);
    ///
    /// // Downcast storage to inner collection
    /// let store = storage.downcast_mut::<HashMap<_, _>>()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn downcast_mut<T>(&mut self) -> Result<&mut T>
    where
        T: Collection<K, V>,
    {
        self.inner.downcast_mut().ok_or(Error::Downcast)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T, K, V> From<T> for Storage<K, V>
where
    T: IntoIterator<Item = (K, V)>,
    K: Key,
    V: Value,
{
    /// Creates a storage from an iterator.
    ///
    /// This implementation allows to create storages directly from iterators
    /// of key-value pairs, which is particularly useful for testing, as well
    /// as documentation examples. Note that the inner collection will always
    /// be a [`HashMap`] when using this method.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::Storage;
    ///
    /// // Create storage from iterator
    /// let mut storage = Storage::from([("key", 42)]);
    /// ```
    #[inline]
    fn from(iter: T) -> Self {
        Self::from_iter(iter)
    }
}

// ----------------------------------------------------------------------------

impl<K, V> FromIterator<(K, V)> for Storage<K, V>
where
    K: Key,
    V: Value,
{
    /// Creates a storage from an iterator.
    ///
    /// When creating storages directly from an iterator, which is primarily
    /// intended for testing, the inner collection is a [`HashMap`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::Storage;
    ///
    /// // Create storage from iterator
    /// let mut storage = Storage::from_iter([("key", 42)]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = (K, V)>,
    {
        Self {
            inner: Box::new(HashMap::from_iter(iter)),
        }
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Deref for Storage<K, V> {
    type Target = dyn Collection<K, V>;

    /// Dereferences to the inner collection.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.inner
    }
}

impl<K, V> DerefMut for Storage<K, V> {
    /// Dereferences to the inner collection mutably.
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.inner
    }
}

// ----------------------------------------------------------------------------

impl<K, V> Default for Storage<K, V>
where
    K: Key,
    V: Value,
{
    /// Creates a storage with [`HashMap::default`][] as a collection.
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
    /// use zrx_storage::Storage;
    ///
    /// // Create storage
    /// let mut storage = Storage::default();
    ///
    /// // Insert value
    /// storage.insert("key", 42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new(HashMap::default())
    }
}
