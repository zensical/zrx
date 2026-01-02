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

//! Stream tuple construction.

use crate::stream::combinator::StreamTuple;
use crate::stream::Stream;

mod convert;

pub use convert::IntoStreamTupleCons;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Stream tuple construction.
///
/// This trait is used to prepend streams to existings stream tuples, which is
/// necessary to implement stream methods that take a tuple of streams, as they
/// need to be combined with the current stream (i.e. `self`) to construct a
/// stream tuple for further consumption.
pub trait StreamTupleCons<I, T> {
    /// Output type of construction.
    type Output: StreamTuple<I>;

    /// Prepends a stream to the stream tuple.
    ///
    /// This method is used to prepend a stream to the tuple and consume it,
    /// constructing a new stream tuple that includes the given stream.
    fn cons(head: Stream<I, T>, tail: Self) -> Self::Output;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements stream tuple construction trait.
macro_rules! impl_stream_tuple_cons {
    ($T1:ident $(, $T:ident)* $(,)?) => {
        impl<I, $T1, $($T,)*> StreamTupleCons<I, $T1> for ($(Stream<I, $T>,)*) {
            type Output = (Stream<I, $T1>, $(Stream<I, $T>,)*);

            #[inline]
            fn cons(head: Stream<I, $T1>, tail: Self) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($T,)*) = tail;
                (head, $($T,)*)
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_stream_tuple_cons!(T1);
impl_stream_tuple_cons!(T1, T2);
impl_stream_tuple_cons!(T1, T2, T3);
impl_stream_tuple_cons!(T1, T2, T3, T4);
impl_stream_tuple_cons!(T1, T2, T3, T4, T5);
impl_stream_tuple_cons!(T1, T2, T3, T4, T5, T6);
impl_stream_tuple_cons!(T1, T2, T3, T4, T5, T6, T7);
impl_stream_tuple_cons!(T1, T2, T3, T4, T5, T6, T7, T8);
