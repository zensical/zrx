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

//! Left join accessor.

use zrx_store::{Key, Value};

use crate::storage::Storage;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Left join accessor.
pub trait LeftJoin<'a, K> {
    /// Output type of accessor.
    type Output;

    /// Returns a tuple of references as a result of a left inner join.
    ///
    /// This method queries each storage for the given key and returns a tuple
    /// of references if the first storage contains the key, or [`None`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::Storage;
    /// use zrx_storage::accessor::LeftJoin;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("key", 42)]);
    /// let b = Storage::from_iter([("key", true)]);
    ///
    /// // Obtain references to values
    /// let value = (&a, &b).left_join(&"key");
    /// assert_eq!(value, Some((&42, Some(&true))));
    /// ```
    #[must_use]
    fn left_join(&self, key: &K) -> Option<Self::Output>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements left join accessor trait.
macro_rules! impl_left_join {
    ($V1:ident $(, $V:ident)+ $(,)?) => {
        impl<'a, K, $V1, $($V,)+> LeftJoin<'a, K>
            for (&'a Storage<K, $V1>, $(&'a Storage<K, $V>,)+)
        where
            K: Key,
            $V1: Value,
            $($V: Value,)+
        {
            type Output = (&'a $V1, $(Option<&'a $V>,)+);

            #[inline]
            fn left_join(&self, key: &K) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                let ($V1, $($V,)+) = self;
                Some(($V1.get(key)?, $($V.get(key),)+))
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_left_join!(V1, V2);
impl_left_join!(V1, V2, V3);
impl_left_join!(V1, V2, V3, V4);
impl_left_join!(V1, V2, V3, V4, V5);
impl_left_join!(V1, V2, V3, V4, V5, V6);
impl_left_join!(V1, V2, V3, V4, V5, V6, V7);
impl_left_join!(V1, V2, V3, V4, V5, V6, V7, V8);
