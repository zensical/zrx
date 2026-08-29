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

//! Coalesce projection.

use crate::store::entry::Value;

// -----------------------------------------------------------------------------
// Traits
// -----------------------------------------------------------------------------

/// Coalesce projection over a row.
pub trait Coalesce<'a> {
    /// Output type of projection.
    type Output;

    /// Returns a reference as a result of a coalesce projection.
    ///
    /// This method returns [`None`] if every tuple slot is empty, and [`Some`]
    /// with a reference to the first value in tuple order otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::row::Coalesce;
    ///
    /// // Create row
    /// let row = (None::<u64>, Some(42), Some(84));
    ///
    /// // Obtain reference to value
    /// let value = row.coalesce();
    /// assert_eq!(value, Some(&42));
    /// ```
    #[must_use]
    fn coalesce(&'a self) -> Option<Self::Output>;
}

// -----------------------------------------------------------------------------
// Macros
// -----------------------------------------------------------------------------

/// Expands homogeneous row slot.
macro_rules! slot {
    ($N:ident, $V:ident) => {
        Option<$V>
    };
}

/// Implements coalesce projection trait.
macro_rules! impl_coalesce {
    ($V1:ident $(, $Vn:ident)+ $(,)?) => {
        impl<'a, V> Coalesce<'a>
            for (slot!($V1, V), $(slot!($Vn, V),)+)
        where
            V: Value,
        {
            type Output = &'a V;

            #[inline]
            fn coalesce(&'a self) -> Option<Self::Output> {
                #[allow(non_snake_case)]
                let ($V1, $($Vn,)+) = self;
                $V1.as_ref()$(.or_else(|| $Vn.as_ref()))+
            }
        }
    };
}

// -----------------------------------------------------------------------------

impl_coalesce!(V1, V2);
impl_coalesce!(V1, V2, V3);
impl_coalesce!(V1, V2, V3, V4);
impl_coalesce!(V1, V2, V3, V4, V5);
impl_coalesce!(V1, V2, V3, V4, V5, V6);
impl_coalesce!(V1, V2, V3, V4, V5, V6, V7);
impl_coalesce!(V1, V2, V3, V4, V5, V6, V7, V8);
