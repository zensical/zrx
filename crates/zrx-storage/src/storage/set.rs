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

//! Storage set.

use slab::Slab;
use std::any::Any;
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use zrx_store::{Key, Value};

use super::Storage;

pub mod view;

pub use view::{View, ViewMut, Views};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Storage set.
///
/// The storage set contains a set of [`Storage`][] instances, and allows to
/// obtain a set of immutable references, as well as a single mutable reference
/// to those instances. It's a essentially newtype wrapper around a [`Slab`] of
/// boxed trait objects that allows us to attach additional methods to it.
///
/// The fact that a [`Slab`] is used as the underlying data structure allows to
/// obtain stable indices for storages under constant insertions and removals.
///
/// [`Storage`]: crate::storage::Storage
///
/// # Examples
///
/// ```
/// use zrx_storage::{Storage, Storages};
///
/// // Create storage
/// let mut storage = Storage::default();
/// storage.insert("key", 42);
///
/// // Create storage set and add storage
/// let mut storages = Storages::default();
/// let n = storages.insert(storage);
/// ```
#[derive(Debug, Default)]
pub struct Storages {
    /// Inner set of storages.
    inner: Slab<Box<dyn Any>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Storages {
    /// Inserts a storage into the storage set and returns its index.
    ///
    /// This method takes a storage and adds it to the storage set, which will
    /// downcast it to a trait object. The caller can then obtain a reference
    /// to the storage using the index returned by this method, which can be
    /// used to downcast the trait object back to its original type.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::default();
    /// storage.insert("key", 42);
    ///
    /// // Create storage set and add storage
    /// let mut storages = Storages::default();
    /// let n = storages.insert(storage);
    /// ```
    #[inline]
    #[must_use]
    pub fn insert<S, K, V>(&mut self, storage: S) -> usize
    where
        S: Into<Storage<K, V>>,
        K: Key,
        V: Value,
    {
        self.inner.insert(Box::new(storage.into()))
    }

    /// Removes a storage from the storage set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::default();
    /// storage.insert("key", 42);
    ///
    /// // Create storage set and add storage
    /// let mut storages = Storages::default();
    /// let n = storages.insert(storage);
    /// ```
    #[inline]
    #[must_use]
    pub fn remove<K, V>(&mut self, index: usize) -> Option<Storage<K, V>>
    where
        K: Key,
        V: Value,
    {
        self.inner
            .try_remove(index)
            .and_then(|any| any.downcast().ok())
            .map(|value| *value)
    }
}

#[allow(clippy::must_use_candidate)]
impl Storages {
    // Returns the number of storages.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any storages.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Index<usize> for Storages {
    type Output = dyn Any;

    /// Returns a reference to the storage at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::default();
    /// storage.insert("key", 42);
    ///
    /// // Create storage set and add storage
    /// let mut storages = Storages::default();
    /// let n = storages.insert(storage);
    ///
    /// // Obtain reference to storage
    /// let storage = &storages[n];
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.inner[index].as_ref()
    }
}

impl IndexMut<usize> for Storages {
    /// Returns a mutable reference to the storage at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::default();
    /// storage.insert("key", 42);
    ///
    /// // Create storage set and add storage
    /// let mut storages = Storages::default();
    /// let n = storages.insert(storage);
    ///
    /// // Obtain mutable reference to storage
    /// let storage = &mut storages[n];
    /// ```
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.inner[index].as_mut()
    }
}
