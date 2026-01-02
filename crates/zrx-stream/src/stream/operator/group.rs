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

//! Group operator.

use ahash::HashMap;

use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::{Descriptor, Report};
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};

use crate::stream::function::SelectFn;
use crate::stream::value::Delta;
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Group operator.
struct Group<F, I> {
    /// Operator function.
    function: F,
    /// Store of group identifiers.
    store: HashMap<I, I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn group<F, U>(&self, f: F) -> Stream<I, Delta<I, U>>
    where
        F: SelectFn<I, T, I>,
        U: Value,
    {
        self.workflow.add_operator(
            [self.id],
            Group {
                function: f,
                store: HashMap::default(),
            },
        )
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F> Operator<I, T> for Group<F, I>
where
    I: Id,
    T: Value + Clone,
    F: SelectFn<I, T, I>,
{
    type Item<'a> = Item<&'a I, Option<&'a T>>;

    /// Handles the given item.
    ///
    /// Grouping is achieved by applying the operator function to each incoming
    /// item to determine its group identifier. An internal mapping of items to
    /// the computed group identifiers is maintained, as items might need to be
    /// migrated between groups when their associated data changes.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        if let Some(data) = item.data {
            // When new data arrives, we apply the operator function to compute
            // the group identifier. If the item was previously associated with
            // another group, we emit a deletion for the prior and an insertion
            // for the new group. Otherwise, we only emit an insertion.
            self.function.execute(item.id, data).map(|report| {
                report.map(|to| {
                    let group = self.store.insert(item.id.clone(), to.clone());
                    let prior = group.filter(|id| *id != to).map(|id| {
                        let inner = Item::new(item.id.clone(), None);
                        Item::new(id, Some(Delta::from([inner])))
                    });

                    // The item returned receives the group identifier, wrapping
                    // the inner item, which is the item passed to the operator
                    let inner = item.into_owned().map(Some);
                    let delta = Delta::from([inner]);
                    prior
                        .into_iter()
                        .chain(Some(Item::new(to, Some(delta))))
                        .collect()
                })
            })
        } else {
            // If the incoming item has no data, interpret this as a deletion,
            // removing the item from its previously allocated group, if any
            let prior = self.store.remove(item.id).map(|id| {
                let inner = Item::new(item.id.clone(), None);
                Item::new(id, Some(Delta::from([inner])))
            });

            // Return delta of items
            Ok(Report::new(prior.into_iter().collect::<Vec<_>>()))
        }
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::default()
    }
}
