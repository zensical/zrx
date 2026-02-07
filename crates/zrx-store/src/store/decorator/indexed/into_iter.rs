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

//! Consuming iterator implementation for [`Indexed`].

use ahash::HashMap;
use std::marker::PhantomData;
use std::vec;

use crate::store::item::Key;
use crate::store::StoreMut;

use super::Indexed;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Consuming iterator over an [`Indexed`] store.
#[derive(Debug)]
pub struct IntoIter<K, V, S = HashMap<K, V>> {
    /// Underlying store.
    store: S,
    /// Ordering of values.
    ordering: vec::IntoIter<K>,
    /// Capture types.
    marker: PhantomData<V>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> IntoIterator for Indexed<K, V, S, C>
where
    K: Key,
    S: StoreMut<K, V>,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V, S>;

    /// Creates a consuming iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Indexed;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Indexed::default();
    /// store.insert("key", 42);
    ///
    /// // Create iterator over the store
    /// for (key, value) in store {
    ///     println!("{key}: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            store: self.store,
            ordering: self.ordering.into_iter(),
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<K, V, S> Iterator for IntoIter<K, V, S>
where
    K: Key,
    S: StoreMut<K, V>,
{
    type Item = (K, V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(key) = self.ordering.next() {
            return self.store.remove(&key).map(|value| (key, value));
        }

        // No more items to return
        None
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.ordering.size_hint()
    }
}

impl<K, V, S> ExactSizeIterator for IntoIter<K, V, S>
where
    K: Key,
    S: StoreMut<K, V>,
{
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.ordering.len()
    }
}
