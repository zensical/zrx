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

//! Delta reduce operator.

use ahash::HashMap;
use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};
use zrx_store::StoreMutRef;

use crate::stream::function::SelectFn;
use crate::stream::value::{Collection, Delta};
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Delta reduce operator.
struct DeltaReduce<I, T, F, U> {
    /// Operator function.
    function: F,
    /// Store of items.
    store: HashMap<I, HashMap<I, T>>,
    /// Type marker.
    marker: PhantomData<U>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, Delta<I, T>>
where
    I: Id,
    T: Value + Clone + Eq,
{
    pub fn delta_reduce<F, U>(&self, f: F) -> Stream<I, U>
    where
        F: SelectFn<I, dyn Collection<I, T>, Option<U>>,
        U: Value,
    {
        self.workflow.add_operator(
            [self.id],
            DeltaReduce {
                function: f,
                store: HashMap::default(),
                marker: PhantomData,
            },
        )
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, U> Operator<I, Delta<I, T>> for DeltaReduce<I, T, F, U>
where
    I: Id,
    T: Value + Clone + Eq,
    F: SelectFn<I, dyn Collection<I, T>, Option<U>>,
    U: Value,
{
    type Item<'a> = Item<&'a I, &'a Delta<I, T>>;

    /// Handles the given item.
    ///
    /// This operator keeps track of the current state of deltas of items that
    /// are associated with each identifier. When a new delta is received, it
    /// updates the internal store accordingly, applying the insertions and
    /// deletions, and passes the updated store to the reduction function.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let store = self.store.get_or_insert_default(item.id);

        // Update internal store (chunk) associated with the item's identifier,
        // applying the insertions and deletions as part of the delta of items
        // to it. This allows us to keep track of the current state of items
        // associated with the identifier.
        for part in item.data {
            if let Some(data) = &part.data {
                store.insert(part.id.clone(), data.clone());
            } else {
                store.remove(&part.id);
            }
        }

        // Since we assume that delta of items are differential by design, we
        // do not need to check if something has actually changed
        self.function.execute(item.id, store).map(|report| {
            report.map(|data| Some(Item::new(item.id.clone(), data)))
        })
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder() // fmt
            .property(Property::Stable)
            .property(Property::Flush)
            .build()
    }
}
