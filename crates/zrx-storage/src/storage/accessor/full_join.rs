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

//! Full join accessor.

use zrx_store::{Key, Value};

use crate::storage::Storage;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Full join accessor.
pub trait FullJoin<'a, K> {
    /// Output type of accessor.
    type Output;

    /// Returns a tuple of references as a result of a full inner join.
    ///
    /// This method queries each storage for the given key and returns a tuple
    /// of references if any storage contains the key, or [`None`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::accessor::FullJoin;
    /// use zrx_storage::Storage;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("key", 42)]);
    /// let b = Storage::from_iter([("key", true)]);
    ///
    /// // Obtain references to values
    /// let value = (&a, &b).full_join(&"key");
    /// assert_eq!(value, Some((Some(&42), Some(&true))));
    /// ```
    #[must_use]
    fn full_join(&self, key: &K) -> Option<Self::Output>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements full join accessor trait.
macro_rules! impl_full_join {
    ($($V:ident),+ $(,)?) => {
        impl<'a, K, $($V),+> FullJoin<'a, K> for ($(&'a Storage<K, $V>,)+)
        where
            K: Key,
            $($V: Value,)+
        {
            type Output = ($(Option<&'a $V>,)+);

            #[inline]
            fn full_join(&self, key: &K) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                let ($($V,)+) = self;
                let mut any = false;
                $(
                    #[allow(non_snake_case)]
                    let $V = $V.get(key);
                    any |= $V.is_some();
                )+
                any.then_some(($($V,)+))
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_full_join!(V1, V2);
impl_full_join!(V1, V2, V3);
impl_full_join!(V1, V2, V3, V4);
impl_full_join!(V1, V2, V3, V4, V5);
impl_full_join!(V1, V2, V3, V4, V5, V6);
impl_full_join!(V1, V2, V3, V4, V5, V6, V7);
impl_full_join!(V1, V2, V3, V4, V5, V6, V7, V8);
