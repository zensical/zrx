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

//! Stream tuple join.

use crate::stream::combinator::StreamTuple;
use crate::stream::value::tuple::{All, Any, First, Presence};
use crate::stream::value::Tuple;
use crate::stream::Stream;

mod convert;

pub use convert::*;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Stream tuple join.
///
/// Stream tuples are heterogeneous collections of streams, which is why they
/// are the natural choice to represent joins. This trait is implemented for
/// tuples of streams in sizes of 1 to 8, and is further specialized with the
/// [`Presence`] markers to implement the following join strategies:
///
/// - [`All`]: All items are required (inner join).
/// - [`First`]: Only the first item is required (left join).
/// - [`Any`]: All items are optional (outer join)
///
/// Extensions of this trait include:
///
/// - [`IntoJoin`]
/// - [`IntoJoinFilter`]
/// - [`IntoJoinFilterMap`]
/// - [`IntoJoinMap`]
///
/// Keeping the concretization of tuples of sizes 1 to 8 together with presence
/// markers in this base trait allows to keep the join implementations focused.
pub trait StreamTupleJoin<I, P>: StreamTuple<I>
where
    P: Presence,
{
    /// Item type.
    type Item: Tuple<P>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements stream join trait with all items required.
macro_rules! impl_stream_join_all {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> StreamTupleJoin<I, All>
            for ($(Stream<I, $T>,)+)
        where
            ($($T,)+): Tuple<All>,
        {
            type Item = ($($T,)+);
        }
    };
}

/// Implements stream join trait with first item required.
macro_rules! impl_stream_join_first {
    ($T1:ident $(, $T:ident)* $(,)?) => {
        impl<I, $T1 $(, $T)*> StreamTupleJoin<I, First>
            for (Stream<I, $T1>, $(Stream<I, $T>),*)
        where
            ($T1, $(Option<$T>),*): Tuple<First>,
        {
            type Item = ($T1, $(Option<$T>),*);
        }
    };
}

/// Implements stream join trait with all items optional.
macro_rules! impl_stream_join_any {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> StreamTupleJoin<I, Any>
            for ($(Stream<I, $T>,)+)
        where
            ($(Option<$T>,)+): Tuple<Any>,
        {
            type Item = ($(Option<$T>,)+);
        }
    };
}

/// Implements stream join traits.
macro_rules! impl_stream_join {
    ($($T:ident),+ $(,)?) => {
        impl_stream_join_all!($($T),+);
        impl_stream_join_first!($($T),+);
        impl_stream_join_any!($($T),+);
    };
}

// ----------------------------------------------------------------------------

impl_stream_join!(T1);
impl_stream_join!(T1, T2);
impl_stream_join!(T1, T2, T3);
impl_stream_join!(T1, T2, T3, T4);
impl_stream_join!(T1, T2, T3, T4, T5);
impl_stream_join!(T1, T2, T3, T4, T5, T6);
impl_stream_join!(T1, T2, T3, T4, T5, T6, T7);
impl_stream_join!(T1, T2, T3, T4, T5, T6, T7, T8);
