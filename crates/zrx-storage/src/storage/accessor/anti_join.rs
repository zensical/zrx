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

//! Anti-join accessor.

use zrx_store::{Key, Value};

use crate::storage::Storage;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Anti-join accessor.
pub trait AntiJoin<'a, K> {
    /// Output type of accessor.
    type Output;

    /// Returns a reference to the value identified by the key under anti-join.
    ///
    /// This method queries each storage for the given key, returning a value
    /// reference to the value of the first storage if no storage contains the
    /// key except for the first. It implements the semantics of an anti-join,
    /// which means only the first value is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::accessor::AntiJoin;
    /// use zrx_storage::Storage;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("a", 42)]);
    /// let b = Storage::from_iter([("b", true)]);
    ///
    /// // Obtain reference to value
    /// let value = (&a, &b).anti_join(&"a");
    /// assert_eq!(value, Some(&42));
    /// ```
    #[must_use]
    fn anti_join(&self, key: &K) -> Option<Self::Output>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements anti-join accessor trait.
macro_rules! impl_anti_join {
    ($V1:ident $(, $V:ident)+ $(,)?) => {
        impl<'a, K, $V1, $($V,)+> AntiJoin<'a, K>
            for (&'a Storage<K, $V1>, $(&'a Storage<K, $V>,)+)
        where
            K: Key,
            $V1: Value,
            $($V: Value,)+
        {
            type Output = &'a $V1;

            #[inline]
            fn anti_join(&self, key: &K) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                let ($V1, $($V,)+) = self;
                let value = $V1.get(key)?;
                $(
                    if $V.contains_key(key) {
                        return None;
                    }
                )+
                Some(value)
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_anti_join!(V1, V2);
impl_anti_join!(V1, V2, V3);
impl_anti_join!(V1, V2, V3, V4);
impl_anti_join!(V1, V2, V3, V4, V5);
impl_anti_join!(V1, V2, V3, V4, V5, V6);
impl_anti_join!(V1, V2, V3, V4, V5, V6, V7);
impl_anti_join!(V1, V2, V3, V4, V5, V6, V7, V8);
