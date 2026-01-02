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

//! Join operator.

use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};

use crate::stream::combinator::tuple::cons::IntoStreamTupleCons;
use crate::stream::combinator::tuple::join::IntoJoin;
use crate::stream::combinator::tuple::StreamTupleJoin;
use crate::stream::value::tuple::{All, Any, First, Presence};
use crate::stream::value::Tuple;
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Join operator.
struct Join<T, P> {
    /// Type marker.
    marker: PhantomData<(T, P)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    pub fn join<S, O>(&self, streams: S) -> Stream<I, O::Item>
    where
        S: IntoStreamTupleCons<I, T, Output = O>,
        O: IntoJoin<I, All>,
    {
        streams // fmt
            .into_stream_tuple_cons(self.clone())
            .into_join()
    }

    pub fn left_join<S, O>(&self, streams: S) -> Stream<I, O::Item>
    where
        S: IntoStreamTupleCons<I, T, Output = O>,
        O: IntoJoin<I, First>,
    {
        streams // fmt
            .into_stream_tuple_cons(self.clone())
            .into_join()
    }

    pub fn full_join<S, O>(&self, streams: S) -> Stream<I, O::Item>
    where
        S: IntoStreamTupleCons<I, T, Output = O>,
        O: IntoJoin<I, Any>,
    {
        streams // fmt
            .into_stream_tuple_cons(self.clone())
            .into_join()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, P> Operator<I, T> for Join<T, P>
where
    I: Id,
    T: Tuple<P>,
{
    type Item<'a> = Item<&'a I, T::Arguments<'a>>;

    /// Handles the given item.
    ///
    /// Joins are one of the most complex operations in stream processing, yet
    /// the implementation here is surprisingly straightforward. The core idea
    /// is to leverage the [`Tuple`] trait to represent the combined state of
    /// all streams being joined.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug" skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        item.into_owned().map(Some)
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder()
            .property(Property::Pure)
            .property(Property::Stable)
            .property(Property::Flush)
            .build()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<S, I, P> IntoJoin<I, P> for S
where
    S: StreamTupleJoin<I, P>,
    I: Id,
    P: Presence,
{
    fn into_join(self) -> Stream<I, Self::Item> {
        self.workflow().add_operator(
            self.ids(),
            Join::<Self::Item, P> { marker: PhantomData },
        )
    }
}
