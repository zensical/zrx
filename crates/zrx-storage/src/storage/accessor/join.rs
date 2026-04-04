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

//! Join accessor.

use zrx_store::{Key, Value};

use crate::storage::Storage;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Join accessor.
pub trait Join<'a, K> {
    /// Output type of accessor.
    type Output;

    /// Returns a tuple of references as a result of an inner join.
    ///
    /// This method queries each storage for the given key and returns a tuple
    /// of references if all storages contain the key, or [`None`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::accessor::Join;
    /// use zrx_storage::Storage;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("key", 42)]);
    /// let b = Storage::from_iter([("key", true)]);
    ///
    /// // Obtain references to values
    /// let value = (&a, &b).join(&"key");
    /// assert_eq!(value, Some((&42, &true)));
    /// ```
    #[must_use]
    fn join(&self, key: &K) -> Option<Self::Output>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements join accessor trait.
macro_rules! impl_join {
    ($($V:ident),+ $(,)?) => {
        impl<'a, K, $($V),+> Join<'a, K> for ($(&'a Storage<K, $V>,)+)
        where
            K: Key,
            $($V: Value,)+
        {
            type Output = ($(&'a $V,)+);

            #[inline]
            fn join(&self, key: &K) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                let ($($V,)+) = self;
                Some(($($V.get(key)?,)+))
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_join!(V1, V2);
impl_join!(V1, V2, V3);
impl_join!(V1, V2, V3, V4);
impl_join!(V1, V2, V3, V4, V5);
impl_join!(V1, V2, V3, V4, V5, V6);
impl_join!(V1, V2, V3, V4, V5, V6, V7);
impl_join!(V1, V2, V3, V4, V5, V6, V7, V8);
