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

//! Sort operator.

use std::cmp::Ordering;
use std::ops::Range;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};
use zrx_store::decorator::Indexed;
use zrx_store::Store;

use crate::stream::value::Position;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Sort operator.
struct Sort<I, T>
where
    I: Id,
{
    /// Store of items.
    store: Indexed<I, T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone + Ord,
{
    pub fn sort(&self) -> Stream<I, Position<T>> {
        self.with_operator(Sort { store: Indexed::default() })
    }

    pub fn sort_with<F>(&self, f: F) -> Stream<I, Position<T>>
    where
        F: Fn(&T, &T) -> Ordering + 'static,
    {
        self.with_operator(Sort { store: Indexed::with_order(f) })
    }

    pub fn sort_by<F, K>(&self, f: F) -> Stream<I, Position<T>>
    where
        F: Fn(&T) -> K + 'static,
        K: Ord,
    {
        self.with_operator(Sort {
            store: Indexed::with_order(move |a, b| f(a).cmp(&f(b))),
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Operator<I, T> for Sort<I, T>
where
    I: Id,
    T: Value + Clone + Ord,
{
    type Item<'a> = Item<&'a I, Option<&'a T>>;

    /// Handles the given item.
    ///
    /// Sorting a stream involves maintaining an internal store of items, and
    /// updating the positions of items whenever an item is inserted, updated,
    /// or removed. The returned items are annotated with their new positions,
    /// which reflect their order in the sorted stream.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let len = self.store.len();

        // After determining the current length of the store, which we need to
        // discern insertions from in-place updates, we either insert into or
        // remove the item from the store, which gives us the affected range of
        // indices. When the range is `None`, it means that nothing changed.
        match item.data {
            Some(data) => self.store.insert_if_changed(item.id, data),
            None => self.store.remove(item.id).map(|n| n..n),
        }
        // If nothing changed, we can return early. Otherwise, if the number of
        // items in the store changed, we must update the positions of all items
        // that come after the affected range. In this case, we need to extend
        // the range to cover the end of the store.
        .map(|Range { start, mut end }| {
            if len != self.store.len() {
                end = self.store.len();
            }

            // If an item was deleted from the store, we need to include the
            // deletion with all items that changed their positions
            let mut items = Vec::with_capacity(end - start);
            if len > self.store.len() {
                items.push(Item::new(item.id.clone(), None));
            }

            // Now, we can iterate over the affected range and create new items
            // with updated positions beginning at the start of the range
            for (index, pair) in self.store.range(start..end).enumerate() {
                let item = Item::from(pair).into_owned();
                items.push(
                    item.map(|data| Some(Position::new(start + index, data))),
                );
            }

            // Return items
            items
        })
        .unwrap_or_default()
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder() // fmt
            .property(Property::Flush)
            .build()
    }
}
