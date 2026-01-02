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

//! Delta filter operator.

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::{Descriptor, Report};
use zrx_scheduler::effect::{Item, Task};
use zrx_scheduler::{Id, Value};

use crate::stream::function::FilterFn;
use crate::stream::value::Delta;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Delta filter operator.
struct DeltaFilter<F> {
    /// Operator function.
    function: F,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, Delta<I, T>>
where
    I: Id,
    T: Value + Clone,
{
    pub fn delta_filter<F>(&self, f: F) -> Stream<I, Delta<I, T>>
    where
        F: FilterFn<I, T> + Clone,
    {
        self.with_operator(DeltaFilter { function: f })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F> Operator<I, Delta<I, T>> for DeltaFilter<F>
where
    I: Id,
    T: Value + Clone,
    F: FilterFn<I, T> + Clone,
{
    type Item<'a> = Item<&'a I, &'a Delta<I, T>>;

    /// Handles the given item.
    ///
    /// This operator returns a task that passes each item of the delta to the
    /// operator function, only forwarding it if the predicate returns `true`.
    /// Note that filtering might even involve fallible operations, e.g.,
    /// using I/O or network, and can include diagnostics.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.into_owned();
        // We could theoretically assume that filtering is generally a cheap
        // operation, and execute the operation function directly. However, as
        // this would mean that filtering involving I/O or network would block
        // the entire scheduler, we rather pay for cloning.
        Task::new({
            let function = self.function.clone();
            move || {
                let mut report = Report::new(());
                let iter = item.data.into_iter().map(|part| {
                    if let Some(data) = part.data {
                        // When new data arrives, we interpret the incoming item
                        // as an insertion or update, so we pass it to the given
                        // operator function, and then, depending on what the
                        // predicate returns, either forward or filter it
                        let temp = function.execute(&part.id, &data)?;
                        Ok(Item::new(
                            part.id,
                            report.merge(temp).then_some(data),
                        ))
                    } else {
                        // If the incoming item has no data, we interpret this
                        // as a deletion, just forwarding the item
                        Ok(Item::new(part.id, None))
                    }
                });

                // Collect and return delta of items
                iter.collect::<Result<Delta<I, T>, _>>().map(|delta| {
                    report.map(|()| Item::new(item.id, Some(delta)))
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
            .build()
    }
}
