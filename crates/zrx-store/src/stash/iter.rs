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

//! Iterator implementations for [`Stash`].

use crate::store::adapter::slab::{Iter, IterMut, Keys, Values};
use crate::store::item::{Key, Value};
use crate::store::{StoreIterable, StoreIterableMut, StoreKeys, StoreValues};

use super::Stash;

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> StoreIterable<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterable<K, usize>,
{
    type Iter<'a> = Iter<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the items of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterable};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for (key, value) in stash.iter() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter(&self) -> Self::Iter<'_> {
        StoreIterable::iter(&self.items)
    }
}

impl<K, V, S> StoreIterableMut<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreIterableMut<K, usize>,
{
    type IterMut<'a> = IterMut<'a, K, V>
    where
        Self: 'a;

    /// Creates a mutable iterator over the items of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreIterableMut};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for (key, value) in stash.iter_mut() {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn iter_mut(&mut self) -> Self::IterMut<'_> {
        StoreIterableMut::iter_mut(&mut self.items)
    }
}

impl<K, V, S> StoreKeys<K, V> for Stash<K, V, S>
where
    K: Key,
    S: StoreKeys<K, usize>,
{
    type Keys<'a> = Keys<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the keys of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreKeys};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for key in stash.keys() {
    ///     println!("{key}");
    /// }
    /// ```
    #[inline]
    fn keys(&self) -> Self::Keys<'_> {
        StoreKeys::keys(&self.items)
    }
}

impl<K, V, S> StoreValues<K, V> for Stash<K, V, S>
where
    K: Key,
    V: Value,
    S: StoreValues<K, usize>,
{
    type Values<'a> = Values<'a, K, V>
    where
        Self: 'a;

    /// Creates an iterator over the values of the stash.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::{Stash, StoreValues};
    ///
    /// // Create stash and initial state
    /// let mut stash = Stash::default();
    /// stash.insert("key", 42);
    ///
    /// // Create iterator over the stash
    /// for value in stash.values() {
    ///     println!("{value}");
    /// }
    /// ```
    #[inline]
    fn values(&self) -> Self::Values<'_> {
        StoreValues::values(&self.items)
    }
}
