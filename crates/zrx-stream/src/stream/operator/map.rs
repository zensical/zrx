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

//! Map operator.

use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::{Item, Task};
use zrx_scheduler::{Id, Value};

use crate::stream::function::MapFn;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Map operator.
struct Map<F, U> {
    /// Operator function.
    function: F,
    /// Type marker.
    marker: PhantomData<U>,
    /// Concurrency.
    concurrency: Option<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn map<F, U>(&self, f: F) -> Stream<I, U>
    where
        F: MapFn<I, T, U> + Clone,
        U: Value,
    {
        self.with_operator(Map {
            function: f,
            marker: PhantomData,
            concurrency: None,
        })
    }

    // @todo temporary solution to implement task concurrency, until we've
    // implemented task groups in zrx, so we can properly manage concurrency
    pub fn map_concurrency<F, U>(
        &self, f: F, concurrency: usize,
    ) -> Stream<I, U>
    where
        F: MapFn<I, T, U> + Clone,
        U: Value,
    {
        self.with_operator(Map {
            function: f,
            marker: PhantomData,
            concurrency: Some(concurrency),
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, U> Operator<I, T> for Map<F, U>
where
    I: Id,
    T: Value + Clone,
    F: MapFn<I, T, U> + Clone,
    U: Value,
{
    type Item<'a> = Item<&'a I, &'a T>;

    /// Handles the given item.
    ///
    /// This operator returns a task that produces an output item by applying
    /// the operator function to the input item. The input item is moved into
    /// the task, and the output item is sent back to the main thread when
    /// the worker thread finishes.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.into_owned();
        Task::new({
            let function = self.function.clone();
            move || {
                function.execute(&item.id, item.data).map(|report| {
                    report.map(|data| Item::new(item.id, Some(data)))
                })
            }
        })
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        let mut builder = Descriptor::builder()
            .property(Property::Pure)
            .property(Property::Stable)
            .property(Property::Flush);

        // Limit concurrency, if set
        if let Some(concurrency) = self.concurrency {
            builder = builder.property(Property::Concurrency(concurrency));
        }
        builder.build()
    }
}
