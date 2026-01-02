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

//! Inspect operator.

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::{Item, Task};
use zrx_scheduler::{Id, Value};

use crate::stream::function::InspectFn;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Inspect operator.
struct Inspect<F> {
    /// Operator function.
    function: F,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn inspect<F>(&self, f: F) -> Stream<I, T>
    where
        F: InspectFn<I, T> + Clone,
    {
        self.with_operator(Inspect { function: f })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F> Operator<I, T> for Inspect<F>
where
    I: Id,
    T: Value + Clone,
    F: InspectFn<I, T> + Clone,
{
    type Item<'a> = Item<&'a I, &'a T>;

    /// Handles the given item.
    ///
    /// This operator allows to inspect each item passing through the stream
    /// without modifying it. The provided function is executed for the given
    /// item as part of a task, allowing for side effects such as logging or
    /// debugging to be performed.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.into_owned();
        Task::new({
            let function = self.function.clone();
            move || {
                function
                    .execute(&item.id, &item.data)
                    .map(|report| report.map(|()| item.map(Some)))
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
