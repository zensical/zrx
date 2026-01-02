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

//! Reduce operator.

use ahash::HashMap;
use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::{Descriptor, Report};
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};
use zrx_store::StoreMut;

use crate::stream::function::SelectFn;
use crate::stream::value::Collection;
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Reduce operator.
struct Reduce<I, T, F, U> {
    /// Identifier.
    id: I,
    /// Operator function.
    function: F,
    /// Store of items.
    store: HashMap<I, T>,
    /// Type marker.
    marker: PhantomData<U>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone + Eq,
{
    pub fn reduce<F, U>(&self, id: I, f: F) -> Stream<I, U>
    where
        F: SelectFn<I, dyn Collection<I, T>, Option<U>>,
        U: Value,
    {
        self.workflow.add_operator(
            [self.id],
            Reduce {
                id,
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

impl<I, T, F, U> Operator<I, T> for Reduce<I, T, F, U>
where
    I: Id,
    T: Value + Clone + Eq,
    F: SelectFn<I, dyn Collection<I, T>, Option<U>>,
    U: Value,
{
    type Item<'a> = Item<&'a I, Option<&'a T>>;

    /// Handles the given item.
    ///
    /// Reductions should be used only sparingly, as they require to store all
    /// items that are flowing through the stream in the operator, because the
    /// reduction is computed on the entire store. This makes sure that the
    /// differential semantics of the stream are preserved.
    ///
    /// If we'd provide an operator for differential reductions (also known as
    /// scanning), the user would be responsible for ensuring the differential
    /// invariant, which might lead to subtle, hard to detect bugs. There are
    /// several other operators that provide case-by-case scan-like semantics,
    /// which are almost always a better choice than using a reduction, as they
    /// are much more efficient and easier to reason about. When this operator
    /// is used incorrectly, it might lead to unbounded memory consumption,
    /// so use it with care.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let has_changed = if let Some(data) = item.data {
            self.store.insert_if_changed(item.id, data)
        } else {
            self.store.remove(item.id).is_some()
        };

        // If the store has changed, we pass it to the operator function in
        // order to compute a new output value. The operator function returns
        // an option to indicate the presence or abscence of a value for the
        // identifier. If nothing has changed, nothing is emitted.
        if has_changed {
            self.function.execute(&self.id, &self.store).map(|report| {
                report.map(|data| Some(Item::new(self.id.clone(), data)))
            })
        } else {
            Ok(Report::new(None))
        }
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder() // fmt
            .property(Property::Flush)
            .build()
    }
}
