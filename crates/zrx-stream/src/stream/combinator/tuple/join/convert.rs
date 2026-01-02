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

//! Stream tuple join conversions.

use zrx_scheduler::Value;

use crate::stream::function::{FilterFn, FilterMapFn, MapFn, Splat};
use crate::stream::value::tuple::Presence;
use crate::stream::Stream;

use super::StreamTupleJoin;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Stream::join`].
pub trait IntoJoin<I, P>: StreamTupleJoin<I, P>
where
    P: Presence,
{
    fn into_join(self) -> Stream<I, Self::Item>;
}

/// Conversion into [`Stream::join_filter`].
pub trait IntoJoinFilter<I, P>: StreamTupleJoin<I, P>
where
    P: Presence,
{
    fn into_join_filter<F>(self, f: F) -> Stream<I, Self::Item>
    where
        F: FilterFn<I, Splat<Self::Item>> + Clone;
}

/// Conversion into [`Stream::join_filter_map`].
pub trait IntoJoinFilterMap<I, P>: StreamTupleJoin<I, P>
where
    P: Presence,
{
    fn into_join_filter_map<F, U>(self, f: F) -> Stream<I, U>
    where
        F: FilterMapFn<I, Splat<Self::Item>, U> + Clone,
        U: Value;
}

/// Conversion into [`Stream::join_map`].
pub trait IntoJoinMap<I, P>: StreamTupleJoin<I, P>
where
    P: Presence,
{
    fn into_join_map<F, U>(self, f: F) -> Stream<I, U>
    where
        F: MapFn<I, Splat<Self::Item>, U> + Clone,
        U: Value;
}
