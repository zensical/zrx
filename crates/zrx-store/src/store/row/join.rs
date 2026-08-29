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

//! Join projection.

use crate::store::entry::Value;

// -----------------------------------------------------------------------------
// Traits
// -----------------------------------------------------------------------------

/// Join projection over a row.
pub trait Join<'a> {
    /// Output type of projection.
    type Output;

    /// Returns references as a result of an inner join.
    ///
    /// This method returns [`None`] if any of the tuple slots are empty, and
    /// [`Some`] with a tuple of references if all slots are filled.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::row::Join;
    ///
    /// // Create row
    /// let row = (Some(42), Some(true));
    ///
    /// // Obtain references to value
    /// let value = row.join();
    /// assert_eq!(value, Some((&42, &true)));
    /// ```
    #[must_use]
    fn join(&'a self) -> Option<Self::Output>;
}

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

/// Implements join projection trait.
macro_rules! impl_join {
    ($($V:ident),+ $(,)?) => {
        impl<'a, $($V),+> Join<'a> for ($(Option<$V>,)+)
        where
            $($V: Value,)+
        {
            type Output = ($(&'a $V,)+);

            #[inline]
            fn join(&'a self) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                if let ($(Some($V),)+) = self {
                    Some(($($V,)+))
                } else {
                    None
                }
            }
        }
    };
}

// -----------------------------------------------------------------------------

impl_join!(V1, V2);
impl_join!(V1, V2, V3);
impl_join!(V1, V2, V3, V4);
impl_join!(V1, V2, V3, V4, V5);
impl_join!(V1, V2, V3, V4, V5, V6);
impl_join!(V1, V2, V3, V4, V5, V6, V7);
impl_join!(V1, V2, V3, V4, V5, V6, V7, V8);
