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

//! Join filter operator.

use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::{Item, Task};
use zrx_scheduler::{Id, Value};

use crate::stream::combinator::tuple::cons::IntoStreamTupleCons;
use crate::stream::combinator::tuple::join::IntoJoinFilter;
use crate::stream::combinator::tuple::StreamTupleJoin;
use crate::stream::function::{FilterFn, Splat};
use crate::stream::value::tuple::{All, Any, First, Presence};
use crate::stream::value::Tuple;
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Join filter operator.
struct JoinFilter<F, P> {
    /// Operator function.
    function: F,
    /// Type marker.
    marker: PhantomData<P>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    pub fn join_filter<S, O, F>(&self, streams: S, f: F) -> Stream<I, O::Item>
    where
        S: IntoStreamTupleCons<I, T, Output = O>,
        O: IntoJoinFilter<I, All>,
        F: FilterFn<I, Splat<O::Item>> + Clone,
    {
        streams
            .into_stream_tuple_cons(self.clone())
            .into_join_filter(f)
    }

    pub fn left_join_filter<S, O, F>(
        &self, streams: S, f: F,
    ) -> Stream<I, O::Item>
    where
        S: IntoStreamTupleCons<I, T, Output = O>,
        O: IntoJoinFilter<I, First>,
        F: FilterFn<I, Splat<O::Item>> + Clone,
    {
        streams
            .into_stream_tuple_cons(self.clone())
            .into_join_filter(f)
    }

    pub fn full_join_filter<S, O, F>(
        &self, streams: S, f: F,
    ) -> Stream<I, O::Item>
    where
        S: IntoStreamTupleCons<I, T, Output = O>,
        O: IntoJoinFilter<I, Any>,
        F: FilterFn<I, Splat<O::Item>> + Clone,
    {
        streams
            .into_stream_tuple_cons(self.clone())
            .into_join_filter(f)
    }
}

// ----------------------------------------------------------------------------
// Tuple implementations
// ----------------------------------------------------------------------------

impl<I, T, F, P> Operator<I, T> for JoinFilter<F, P>
where
    I: Id,
    T: Tuple<P>,
    F: FilterFn<I, Splat<T>> + Clone,
{
    type Item<'a> = Item<&'a I, T::Arguments<'a>>;

    /// Handles the given item.
    ///
    /// This operator returns a task that passes the given item to the operator
    /// function, only forwarding it if the predicate returns `true`. Note that
    /// filtering might even involve fallible operations, e.g., using I/O or
    /// network, and can include diagnostics.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug" skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.into_owned().map(Splat::from);
        // We could theoretically assume that filtering is generally a cheap
        // operation, and execute the operation function directly. However, as
        // this would mean that filtering involving I/O or network would block
        // the entire scheduler, we rather pay for cloning.
        Task::new({
            let function = self.function.clone();
            move || {
                function.execute(&item.id, &item.data).map(|report| {
                    report.map(|keep| {
                        keep.then(|| item.map(|data| Some(data.into_inner())))
                    })
                })
            }
        })
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

impl<S, I, P> IntoJoinFilter<I, P> for S
where
    S: StreamTupleJoin<I, P>,
    I: Id,
    P: Presence,
{
    fn into_join_filter<F>(self, f: F) -> Stream<I, Self::Item>
    where
        F: FilterFn<I, Splat<Self::Item>> + Clone,
    {
        self.workflow().add_operator(
            self.ids(),
            JoinFilter {
                function: f,
                marker: PhantomData,
            },
        )
    }
}
