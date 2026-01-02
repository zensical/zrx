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

//! Lift operator.

use ahash::HashMap;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::{Descriptor, Report};
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};
use zrx_store::behavior::StoreDelta;
use zrx_store::StoreMutRef;

use crate::stream::function::LiftFn;
use crate::stream::value::Delta;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Lift operator.
struct Lift<F, I, U> {
    /// Operator function.
    function: F,
    /// Store of change sets.
    store: HashMap<I, HashMap<I, U>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    pub fn lift<F, U>(&self, f: F) -> Stream<I, Delta<I, U>>
    where
        F: LiftFn<I, T, U>,
        U: Value + Clone + Eq,
    {
        self.with_operator(Lift {
            function: f,
            store: HashMap::default(),
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, U> Operator<I, T> for Lift<F, I, U>
where
    I: Id,
    T: Value,
    F: LiftFn<I, T, U>,
    U: Value + Clone + Eq,
{
    type Item<'a> = Item<&'a I, Option<&'a T>>;

    /// Handles the given item.
    ///
    /// Lifting is an essential operation in stream processing, as it allows an
    /// input item to be transformed into multiple related output items with a
    /// user-defined function. This operator always returns a delta of items,
    /// ensuring that the related items can be processed differentially.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        if let Some(data) = item.data {
            // When new data arrives, we apply the operator function to generate
            // related items. We then compute the delta between the previous set
            // of related items and the returned set, ensuring that only changes
            // are propagated downstream.
            self.function.execute(item.id, data).map(|report| {
                report.map(|data| {
                    let store = self.store.get_or_insert_default(item.id);
                    let delta = store
                        .changes(data.into_iter().map(Item::into_parts))
                        .map(|(id, data)| Item::new(id, data))
                        .collect();

                    // Return delta of items
                    Item::new(item.id.clone(), Some(delta))
                })
            })
        } else {
            // If the incoming item has no data, interpret this as a deletion,
            // removing any previously stored related items. By emitting a delta
            // of items instead of a deletion, we can ensure that all downstream
            // operators can update their internal state accordingly.
            let store = self.store.remove(item.id);
            let delta = store.map_or_else(Delta::default, |inner| {
                inner.into_keys().map(|id| Item::new(id, None)).collect()
            });

            // Return delta of items
            Ok(Report::new(Item::new(item.id.clone(), Some(delta))))
        }
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder() // fmt
            .property(Property::Stable)
            .build()
    }
}
