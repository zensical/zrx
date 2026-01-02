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

//! Store delta behavior.

use crate::store::{StoreIterable, StoreMut};
use crate::Key;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Store delta behavior.
///
/// When creating chunks of items from a stream of items, we sometimes need to
/// convert this to deltas of items, i.e., items representing the changes in the
/// store. This trait allows to compute deltas for a store after applying a set
/// of changes, emitting the insertions and deletions that were made.
///
/// __Warning__: Implementations are expected to consolidate the inputs sourced
/// from the iterator, so only the last instance from the iterator is persisted
/// If inputs are not consolidated, deltas of items with duplicate keys may be
/// yielded, which might lead to unnecessary computations.
pub trait StoreDelta<I, A>
where
    A: Eq,
{
    /// Updates the store and returns the changes as an iterator.
    fn changes<T>(&mut self, iter: T) -> impl Iterator<Item = (I, Option<A>)>
    where
        T: IntoIterator<Item = (I, A)>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K, V, S> StoreDelta<K, V> for S
where
    K: Key,
    V: Clone + Eq,
    S: StoreMut<K, V> + StoreIterable<K, V>,
{
    /// Updates the store and returns the changes as an iterator.
    ///
    /// This method consumes an iterator of key-value pairs, applies the items
    /// to the store, and returns an iterator over the changes that were made.
    /// Note that the returned iterator yields both insertions and deletions,
    /// while the input iterator is expected to only yield insertions.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use zrx_store::behavior::StoreDelta;
    /// use zrx_store::StoreMut;
    ///
    /// // Create store and initial state
    /// let mut store = HashMap::new();
    /// store.insert("a", 42);
    /// store.insert("b", 84);
    ///
    /// // Update store and inspect changes
    /// for (key, value) in store.changes([("a", 42)]) {
    ///   println!("{key}: {value:?}");
    /// }
    /// ```
    fn changes<T>(&mut self, iter: T) -> impl Iterator<Item = (K, Option<V>)>
    where
        T: IntoIterator<Item = (K, V)>,
    {
        // This implementation is based on the assumption that change sets are
        // small, which allows for some optimizations. We assume that there are
        // likely no more than 10 changes in a delta, so we can use a vector of
        // keys instead of a hash map for delta computation.
        let mut keys = Vec::new();

        // Phase 1: Generate change sets processing the inputs. Unfortunately,
        // we have to collect all items, and can't just lazily yield them since
        // we're moving the temporary set of keys into the next closure, while
        // also consuming it afterwards to generate deletions.
        let inserts = iter
            .into_iter()
            .filter_map(|(key, value)| {
                keys.push(key.clone());
                self.insert_if_changed(&key, &value)
                    .then(|| (key, Some(value)))
            })
            .collect::<Vec<_>>()
            .into_iter();

        // Determine which keys were not found in the items returned by the
        // given iterator, so we can remove them in the next step
        let iter = self.iter().filter_map(|(key, _)| {
            match keys.iter().position(|check| check == key) {
                None => Some(key.clone()),
                Some(n) => {
                    keys.swap_remove(n);
                    None
                }
            }
        });

        // Phase 2: Generate a deletion for each key that was not found in the
        // items returned by the given iterator, and remove them from the store.
        // We also need to collect the keys here, or we would return references,
        // which would make usage much more complicated.
        let deletes = iter
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|key| self.remove(&key).map(|_| (key, None)));

        // Return iterator over changes
        inserts.chain(deletes)
    }
}
