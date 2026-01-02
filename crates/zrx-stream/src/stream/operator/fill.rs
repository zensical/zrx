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

//! Fill operator.

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::{Descriptor, Report};
use zrx_scheduler::effect::{Item, Task};
use zrx_scheduler::{Id, Value};

use crate::stream::function::DefaultFn;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Fill operator.
struct Fill<F> {
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
    pub fn fill(&self, default: T) -> Stream<I, T> {
        self.fill_with(move || Some(default.clone()))
    }

    pub fn fill_with<F>(&self, f: F) -> Stream<I, T>
    where
        F: DefaultFn<I, T> + Clone,
    {
        self.with_operator(Fill { function: f })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F> Operator<I, T> for Fill<F>
where
    I: Id,
    T: Value + Clone,
    F: DefaultFn<I, T> + Clone,
{
    type Item<'a> = Item<&'a I, Option<&'a T>>;

    /// Handles the given item.
    ///
    /// If the given item holds associated data, this operator just passes it
    /// through. Otherwise, it invokes the operator function. If the function
    /// returns a value, it is set as the associated data for the item. Note
    /// that the default might be specific to the identifier.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.into_owned();
        Task::new({
            let function = self.function.clone();
            move || {
                if item.data.is_some() {
                    Ok(Report::new(item))
                } else {
                    function.execute(&item.id).map(|report| {
                        report.map(|data| Item::new(item.id, data))
                    })
                }
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
