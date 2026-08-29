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

//! Left-join projection.

use crate::store::entry::Value;

// -----------------------------------------------------------------------------
// Traits
// -----------------------------------------------------------------------------

/// Left-join projection over a row.
pub trait LeftJoin<'a> {
    /// Output type of projection.
    type Output;

    /// Returns references as a result of a left join.
    ///
    /// This method returns [`None`] if the first tuple slot is empty, and
    /// [`Some`] with the first reference and optional remaining references if
    /// the first slot is filled.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::row::LeftJoin;
    ///
    /// // Create row
    /// let row = (Some(42), None::<bool>);
    ///
    /// // Obtain references to value
    /// let value = row.left_join();
    /// assert_eq!(value, Some((&42, None)));
    /// ```
    #[must_use]
    fn left_join(&'a self) -> Option<Self::Output>;
}

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

/// Implements left-join projection trait.
macro_rules! impl_left_join {
    ($V1:ident $(, $V:ident)+ $(,)?) => {
        impl<'a, $V1, $($V),+> LeftJoin<'a>
            for (Option<$V1>, $(Option<$V>,)+)
        where
            $V1: Value,
            $($V: Value,)+
        {
            type Output = (&'a $V1, $(Option<&'a $V>,)+);

            #[inline]
            fn left_join(&'a self) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                if let (Some($V1), $($V,)+) = self {
                    Some(($V1, $($V.as_ref(),)+))
                } else {
                    None
                }
            }
        }
    };
}

// -----------------------------------------------------------------------------

impl_left_join!(V1, V2);
impl_left_join!(V1, V2, V3);
impl_left_join!(V1, V2, V3, V4);
impl_left_join!(V1, V2, V3, V4, V5);
impl_left_join!(V1, V2, V3, V4, V5, V6);
impl_left_join!(V1, V2, V3, V4, V5, V6, V7);
impl_left_join!(V1, V2, V3, V4, V5, V6, V7, V8);
