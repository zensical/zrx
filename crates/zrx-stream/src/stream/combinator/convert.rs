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

//! Stream tuple conversions.

use zrx_scheduler::Value;

use crate::stream::Stream;

use super::tuple::StreamTupleCons;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into a tuple of [`Stream`] references.
pub trait IntoStreamTuple<I> {
    /// Output type of conversion.
    type Output;

    /// Converts into a tuple of stream references.
    fn into_stream_tuple(self) -> Self::Output;
}

/// Conversion into [`StreamTupleCons`].
pub trait IntoStreamTupleCons<I, T> {
    /// Output type of conversion.
    type Output;

    /// Combines a stream with a tuple of stream references.
    ///
    /// While this method's signature looks like it should rather be inverted,
    /// this trait is solely intended as a helper trait to combine stream tuples
    /// with a stream. It combines [`IntoStreamTuple`] and [`StreamTupleCons`],
    /// allowing to keep the user-facing API ergonomic and convenient.
    fn into_stream_tuple_cons(self, stream: Stream<I, T>) -> Self::Output;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> IntoStreamTuple<I> for &Stream<I, T>
where
    T: Value,
{
    type Output = (Stream<I, T>,);

    /// Converts a stream reference into a stream tuple.
    ///
    /// Albeit this conversion is trivial, it allows to pass stream references
    /// to functions that expect tuples, which can be quite convenient.
    #[inline]
    fn into_stream_tuple(self) -> Self::Output {
        (self.clone(),)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<I, T, S, C> IntoStreamTupleCons<I, T> for S
where
    S: IntoStreamTuple<I, Output = C>,
    C: StreamTupleCons<I, T>,
{
    type Output = C::Output;

    #[inline]
    fn into_stream_tuple_cons(self, stream: Stream<I, T>) -> Self::Output {
        StreamTupleCons::cons(stream, self.into_stream_tuple())
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements stream tuple conversion trait.
macro_rules! impl_into_stream_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> IntoStreamTuple<I> for ($(&Stream<I, $T>,)+)
        where
            $($T: Value),+
        {
            type Output = ($(Stream<I, $T>,)+);

            #[inline]
            fn into_stream_tuple(self) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($T,)+) = self;
                ($($T.clone(),)+)
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_into_stream_tuple!(T1);
impl_into_stream_tuple!(T1, T2);
impl_into_stream_tuple!(T1, T2, T3);
impl_into_stream_tuple!(T1, T2, T3, T4);
impl_into_stream_tuple!(T1, T2, T3, T4, T5);
impl_into_stream_tuple!(T1, T2, T3, T4, T5, T6);
impl_into_stream_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_into_stream_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
