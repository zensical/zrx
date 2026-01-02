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

//! Audit operator.

use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::{Descriptor, Outputs};
use zrx_scheduler::effect::timer::IntoDuration;
use zrx_scheduler::effect::{Item, Timer};
use zrx_scheduler::{Id, Value};

use crate::stream::function::SelectFn;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Audit operator.
struct Audit<F, D> {
    /// Operator function.
    function: F,
    /// Type marker.
    marker: PhantomData<D>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn audit<D>(&self, duration: D) -> Stream<I, T>
    where
        D: IntoDuration,
    {
        let duration = duration.into_duration();
        self.with_operator(Audit {
            function: move |_: &T| duration,
            marker: PhantomData,
        })
    }

    pub fn audit_with<F, D>(&self, f: F) -> Stream<I, T>
    where
        F: SelectFn<I, T, D>,
        D: IntoDuration,
    {
        self.with_operator(Audit {
            function: f,
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, D> Operator<I, T> for Audit<F, D>
where
    I: Id,
    T: Value + Clone,
    F: SelectFn<I, T, D>,
    D: IntoDuration,
{
    type Item<'a> = Item<&'a I, &'a T>;

    /// Handles the given item.
    ///
    /// Auditing is implemented with the help of a timer which is set, and can
    /// be considered to be the inverse of throttling, as it will immediately
    /// return the most recent item and start the timer, ignoring subsequent
    /// items until the timer is due.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        self.function.execute(item.id, item.data).map(|report| {
            report.map(|duration| {
                Timer::set(
                    duration,
                    Some(Outputs::from([item.into_owned().map(Some)])),
                )
            })
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
