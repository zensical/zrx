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

//! Product operator.

use ahash::HashMap;

use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};
use zrx_store::StoreMut;

use crate::stream::operator::Operator;
use crate::stream::value::Delta;
use crate::stream::Stream;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Product operator.
struct Product<I, T, U> {
    /// Store of items (left).
    this: HashMap<I, T>,
    /// Store of items (right).
    that: HashMap<I, U>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone + Eq,
{
    pub fn product<U>(
        &self, stream: &Stream<I, U>,
    ) -> Stream<I, Delta<I, (T, U)>>
    where
        U: Value + Clone + Eq,
    {
        self.workflow.add_operator(
            [self.id, stream.id],
            Product::<I, T, U> {
                this: HashMap::default(),
                that: HashMap::default(),
            },
        )
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, U> Operator<I, T> for Product<I, T, U>
where
    I: Id,
    T: Value + Clone + Eq,
    U: Value + Clone + Eq,
{
    type Item<'a> = Item<&'a I, (Option<&'a T>, Option<&'a U>)>;

    /// Handles the given item.
    ///
    /// Computing the cartesian product of two streams is a stateful operation
    /// that requires maintaining internal stores for both input streams. When
    /// an item is received from either stream, we'll update the corresponding
    /// store and compute the product with all items in the other store. This
    /// ensures that all combinations of items from both streams are emitted
    /// differentially.
    ///
    /// This implementation assumes that the streams are synchronized, which is
    /// given if both input streams either belong to the same frontier, or if
    /// the scheduler ensures that both streams are backed by stores that are
    /// synchronized. If this assumption does not hold, this operator may yield
    /// duplicate emissions, as there's not way for the operator to know whether
    /// absence of one of the input streams is due to a deletion, or because the
    /// item has not yet arrived. We deem this acceptable for now, as it only
    /// affects the efficiency of the operator, but not its correctness.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug" skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let mut items = Vec::new();

        // The left (this) and right (that) values are optional, indicating
        // whether there is an insertion or a deletion. Note that both values
        // can be present or absent at the same time, which can indicate a
        // self-join, but does not necessarily have to.
        let (this, that) = item.data;
        if let Some(data) = this {
            // If the left item value is present, update the left store, and if
            // it changed, compute the product of the item and the right store
            if self.this.insert_if_changed(item.id, data)
                && !self.that.is_empty()
            {
                let iter = self.that.iter().map(|(id, that)| {
                    Item::new(id.clone(), Some((data.clone(), that.clone())))
                });
                items.push(Item::new(
                    item.id.clone(),
                    Some(iter.collect::<Delta<_, _>>()),
                ));
            }
        } else if that.is_none() && self.this.remove(item.id).is_some() {
            // An item was present and was successfully removed, so we compute
            // the product of the removed item and the right store
            let iter = self.that.keys().map(|id| Item::new(id.clone(), None));
            items.push(Item::new(
                item.id.clone(),
                Some(iter.collect::<Delta<_, _>>()),
            ));
        }

        // After processing the left value, we do the same for the right value
        // by updating the right store - basically the other way round
        if let Some(data) = that {
            // If the right item value is present, update the right store, and
            // if it changed, compute the product of the left store and the item
            if self.that.insert_if_changed(item.id, data) {
                items.extend(self.this.iter().map(|(id, this)| {
                    Item::new(
                        id.clone(),
                        Some(Delta::from([Item::new(
                            item.id.clone(),
                            Some((this.clone(), data.clone())),
                        )])),
                    )
                }));
            }
        } else if this.is_none() && self.that.remove(item.id).is_some() {
            // An item was present, and could be removed, so we compute the
            // product of the removed item and all items in the left store
            items.extend(self.this.keys().map(|id| {
                let inner = Item::new(item.id.clone(), None);
                Item::new(id.clone(), Some(Delta::from([inner])))
            }));
        }

        // Return deltas of items
        items
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::default()
    }
}
