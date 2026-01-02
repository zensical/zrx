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

//! Coalesce operator.

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

/// Coalesce operator.
struct Coalesce<T> {
    /// Type marker.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn coalesce<S>(&self, streams: S) -> Stream<I, T>
    where
        S: IntoStreamSet<I, T>,
    {
        let set = self.into_stream_set().union(streams);
        self.workflow.add_operator(
            set.into_iter().map(|stream| stream.id),
            Coalesce::<T> { marker: PhantomData },
        )
    }
}

// ----------------------------------------------------------------------------

impl<I, T> StreamSet<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn into_coalesce(self) -> Option<Stream<I, T>> {
        self.get(0)
            .map(|head| head.workflow.clone())
            .map(|workflow| {
                workflow.add_operator(
                    self.into_iter().map(|stream| stream.id),
                    Coalesce::<T> { marker: PhantomData },
                )
            })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Operator<I, T> for Coalesce<T>
where
    I: Id,
    T: Value + Clone,
{
    type Item<'a> = Item<&'a I, Vec<Option<&'a T>>>;

    /// Handles the given item.
    ///
    /// Coalescing is implemented by checking the streams one after another for
    /// the presence of an item, and returning the first occurrence.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        item.map(|data| data.into_iter().find_map(|opt| opt))
            .into_owned()
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
