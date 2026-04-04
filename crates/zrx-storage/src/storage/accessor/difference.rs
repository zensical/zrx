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

//! Difference accessor.

use zrx_store::{Key, Value};

use crate::storage::Storage;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Difference accessor.
pub trait Difference<'a, K, V> {
    /// Returns a reference to the value as a result of a set difference.
    #[must_use]
    fn difference(&self, key: &K) -> Option<&'a V>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<'a, K, V> Difference<'a, K, V> for [&'a Storage<K, V>]
where
    K: Key,
    V: Value,
{
    /// Returns a reference to the value as a result of a set difference.
    ///
    /// This method queries each storage for the given key and only returns a
    /// reference if no other storage doesn't return a value for it, regardless
    /// of whether it matches. It's functionally equivalent to [`AntiJoin`][],
    /// but implemented for sequences of storage references.
    ///
    /// [`AntiJoin`]: crate::storage::accessor::AntiJoin
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::accessor::Difference;
    /// use zrx_storage::Storage;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("a", 42)]);
    /// let b = Storage::from_iter([("b", 42)]);
    ///
    /// // Obtain reference to value
    /// let value = [&a, &b].difference(&"a");
    /// assert_eq!(value, Some(&42));
    /// ```
    fn difference(&self, key: &K) -> Option<&'a V> {
        let mut iter = self.iter();
        let value = iter.next()?.get(key)?;
        iter.find_map(|store| store.get(key))
            .is_none()
            .then_some(value)
    }
}
