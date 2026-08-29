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

//! Semi-join projection.

use crate::store::entry::Value;

// -----------------------------------------------------------------------------
// Traits
// -----------------------------------------------------------------------------

/// Semi-join projection over a row.
pub trait SemiJoin<'a> {
    /// Output type of projection.
    type Output;

    /// Returns a reference as a result of a semi join.
    ///
    /// This method returns [`None`] if any tuple slot is empty, and [`Some`]
    /// with a reference to the first value if every slot is filled.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::row::SemiJoin;
    ///
    /// // Create row
    /// let row = (Some(42), Some(true));
    ///
    /// // Obtain reference to value
    /// let value = row.semi_join();
    /// assert_eq!(value, Some(&42));
    /// ```
    #[must_use]
    fn semi_join(&'a self) -> Option<Self::Output>;
}

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

/// Implements semi-join projection trait.
macro_rules! impl_semi_join {
    ($V1:ident $(, $V:ident)+ $(,)?) => {
        impl<'a, $V1, $($V),+> SemiJoin<'a>
            for (Option<$V1>, $(Option<$V>,)+)
        where
            $V1: Value,
            $($V: Value,)+
        {
            type Output = &'a $V1;

            #[inline]
            fn semi_join(&'a self) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                if let (Some($V1), $($V,)+) = self {
                    ($($V.is_some())&&+).then_some($V1)
                } else {
                    None
                }
            }
        }
    };
}

// -----------------------------------------------------------------------------

impl_semi_join!(V1, V2);
impl_semi_join!(V1, V2, V3);
impl_semi_join!(V1, V2, V3, V4);
impl_semi_join!(V1, V2, V3, V4, V5);
impl_semi_join!(V1, V2, V3, V4, V5, V6);
impl_semi_join!(V1, V2, V3, V4, V5, V6, V7);
impl_semi_join!(V1, V2, V3, V4, V5, V6, V7, V8);
