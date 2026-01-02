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

//! Stream tuple extensions.

use zrx_scheduler::{Id, Value};

use crate::stream::combinator::tuple::join::{
    IntoJoin, IntoJoinFilter, IntoJoinFilterMap, IntoJoinMap,
};
use crate::stream::function::{FilterFn, FilterMapFn, MapFn, Splat};
use crate::stream::value::tuple::{All, Any, First};
use crate::stream::Stream;

use super::convert::IntoStreamTuple;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Extension of [`StreamTuple`][].
///
/// While conceptually, this extension trait does belong to [`StreamTuple`][],
/// allowing to conveniently work with data types that can be converted into a
/// tuple of streams, it is deliberately implemented for anything that converts
/// via [`IntoStreamTuple`], as this offers more flexibilty.
///
/// [`StreamTuple`]: crate::stream::combinator::StreamTuple
pub trait StreamTupleExt<I, S>: IntoStreamTuple<I, Output = S> + Sized
where
    I: Id,
{
    fn join(self) -> Stream<I, S::Item>
    where
        S: IntoJoin<I, All>,
    {
        self.into_stream_tuple() // fmt
            .into_join()
    }

    fn join_filter<F>(self, f: F) -> Stream<I, S::Item>
    where
        S: IntoJoinFilter<I, All>,
        F: FilterFn<I, Splat<S::Item>> + Clone,
    {
        self.into_stream_tuple() // fmt
            .into_join_filter(f)
    }

    fn join_filter_map<F, U>(self, f: F) -> Stream<I, U>
    where
        S: IntoJoinFilterMap<I, All>,
        F: FilterMapFn<I, Splat<S::Item>, U> + Clone,
        U: Value,
    {
        self.into_stream_tuple() // fmt
            .into_join_filter_map(f)
    }

    fn join_map<F, U>(self, f: F) -> Stream<I, U>
    where
        S: IntoJoinMap<I, All>,
        F: MapFn<I, Splat<S::Item>, U> + Clone,
        U: Value,
    {
        self.into_stream_tuple() // fmt
            .into_join_map(f)
    }

    fn left_join(self) -> Stream<I, S::Item>
    where
        S: IntoJoin<I, First>,
    {
        self.into_stream_tuple() // fmt
            .into_join()
    }

    fn left_join_filter<F>(self, f: F) -> Stream<I, S::Item>
    where
        S: IntoJoinFilter<I, First>,
        F: FilterFn<I, Splat<S::Item>> + Clone,
    {
        self.into_stream_tuple() // fmt
            .into_join_filter(f)
    }

    fn left_join_filter_map<F, U>(self, f: F) -> Stream<I, U>
    where
        S: IntoJoinFilterMap<I, First>,
        F: FilterMapFn<I, Splat<S::Item>, U> + Clone,
        U: Value,
    {
        self.into_stream_tuple() // fmt
            .into_join_filter_map(f)
    }

    fn left_join_map<F, U>(self, f: F) -> Stream<I, U>
    where
        S: IntoJoinMap<I, First>,
        F: MapFn<I, Splat<S::Item>, U> + Clone,
        U: Value,
    {
        self.into_stream_tuple() // fmt
            .into_join_map(f)
    }

    fn full_join(self) -> Stream<I, S::Item>
    where
        S: IntoJoin<I, Any>,
    {
        self.into_stream_tuple() // fmt
            .into_join()
    }

    fn full_join_filter<F>(self, f: F) -> Stream<I, S::Item>
    where
        S: IntoJoinFilter<I, Any>,
        F: FilterFn<I, Splat<S::Item>> + Clone,
    {
        self.into_stream_tuple() // fmt
            .into_join_filter(f)
    }

    fn full_join_filter_map<F, U>(self, f: F) -> Stream<I, U>
    where
        S: IntoJoinFilterMap<I, Any>,
        F: FilterMapFn<I, Splat<S::Item>, U> + Clone,
        U: Value,
    {
        self.into_stream_tuple() // fmt
            .into_join_filter_map(f)
    }

    fn full_join_map<F, U>(self, f: F) -> Stream<I, U>
    where
        S: IntoJoinMap<I, Any>,
        F: MapFn<I, Splat<S::Item>, U> + Clone,
        U: Value,
    {
        self.into_stream_tuple() // fmt
            .into_join_map(f)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T, I, S> StreamTupleExt<I, S> for T
where
    I: Id,
    T: IntoStreamTuple<I, Output = S>,
{
}
