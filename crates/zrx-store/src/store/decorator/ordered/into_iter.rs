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

//! Consuming iterator implementation for [`Ordered`].

use std::collections::btree_map;
use std::vec;

use crate::store::comparator::{Ascending, Comparable};
use crate::store::item::Key;
use crate::store::Store;

use super::Ordered;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Consuming iterator over an [`Ordered`] store.
#[derive(Debug)]
pub struct IntoIter<K, V, C = Ascending> {
    /// Ordering of values.
    ordering: btree_map::IntoIter<Comparable<V, C>, Vec<K>>,
    /// Current value.
    value: Option<V>,
    /// Current keys.
    keys: vec::IntoIter<K>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S, C> IntoIterator for Ordered<K, V, S, C>
where
    K: Key,
    V: Clone,
    S: Store<K, V>,
{
    type Item = (K, V);
    type IntoIter = IntoIter<K, V, C>;

    /// Creates a consuming iterator over the store.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::decorator::Ordered;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = Ordered::default();
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
            ordering: self.ordering.into_iter(),
            value: None,
            keys: vec::IntoIter::default(),
        }
    }
}

// ----------------------------------------------------------------------------

impl<K, V, C> Iterator for IntoIter<K, V, C>
where
    K: Key,
    V: Clone,
{
    type Item = (K, V);

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Check if we have keys left with the current value
            if let Some(key) = self.keys.next() {
                return self.value.clone().map(|value| (key, value));
            }

            // Fetch the next value and associated keys
            if let Some((value, keys)) = self.ordering.next() {
                self.value = Some(value.into_inner());
                self.keys = keys.into_iter();
            } else {
                break;
            }
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
