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

//! Difference operator.

use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};

use crate::stream::combinator::{IntoStreamSet, StreamSet};
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Difference operator.
struct Difference<T> {
    /// Type marker.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone + Eq,
{
    pub fn difference<S>(&self, streams: S) -> Stream<I, T>
    where
        S: IntoStreamSet<I, T>,
    {
        let set = self.into_stream_set().union(streams);
        self.workflow.add_operator(
            set.into_iter().map(|stream| stream.id),
            Difference::<T> { marker: PhantomData },
        )
    }
}

// ----------------------------------------------------------------------------

impl<I, T> StreamSet<I, T>
where
    I: Id,
    T: Value + Clone + Eq,
{
    pub fn into_difference(self) -> Option<Stream<I, T>> {
        self.get(0)
            .map(|head| head.workflow.clone())
            .map(|workflow| {
                workflow.add_operator(
                    self.into_iter().map(|stream| stream.id),
                    Difference::<T> { marker: PhantomData },
                )
            })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Operator<I, T> for Difference<T>
where
    I: Id,
    T: Value + Clone + Eq,
{
    type Item<'a> = Item<&'a I, Vec<Option<&'a T>>>;

    /// Handles the given item.
    ///
    /// Differences of streams are computed by checking that each subsequent
    /// stream does not have an exact copy of the item of the first stream.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.map(|data| {
            let mut iter = data.into_iter();
            iter.next()?.and_then(|head| {
                iter.all(|opt| opt != Some(head)).then_some(head)
            })
        });

        // Return item
        item.into_owned()
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
