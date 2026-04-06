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

//! Join conversions.

use zrx_scheduler::{Id, Value};

use crate::stream::operator::Join;
use crate::stream::operator::Operator;
use crate::stream::Stream;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Stream::join`].
pub trait IntoJoin<I> {
    /// Output type.
    type Output: Value;

    /// Joins a tuple of streams.
    fn into_join(self) -> Stream<I, Self::Output>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements join conversion trait.
macro_rules! impl_into_join {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> IntoJoin<I> for ($(Stream<I, $T>,)+)
        where
            I: Id,
            $($T: Value,)+
        {
            type Output = ($($T,)+);

            #[inline]
            fn into_join(self) -> Stream<I, Self::Output> {
                self.subscribe(Join::<Self::Output>::new())
            }
        }
    }
}

// ----------------------------------------------------------------------------

impl_into_join!(T1, T2);
impl_into_join!(T1, T2, T3);
impl_into_join!(T1, T2, T3, T4);
impl_into_join!(T1, T2, T3, T4, T5);
impl_into_join!(T1, T2, T3, T4, T5, T6);
impl_into_join!(T1, T2, T3, T4, T5, T6, T7);
impl_into_join!(T1, T2, T3, T4, T5, T6, T7, T8);
