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

//! Select operator.

use ahash::{HashMap, HashSet};

use zrx_scheduler::action::descriptor::Interest;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::{Item, Signal};
use zrx_scheduler::{Id, Value};

use crate::stream::barrier::{Barrier, Condition};
use crate::stream::value::Delta;
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Select operator.
struct Select<I, T>
where
    I: Id,
{
    /// Items.
    items: HashMap<I, T>,
    /// Barriers.
    barriers: HashMap<I, Barrier<I>>,
    /// Observed ids.
    ids: HashSet<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn select(
        &self, selector: &Stream<I, Condition<I>>,
    ) -> Stream<I, Delta<I, T>> {
        self.workflow.add_operator(
            [self.id, selector.id],
            Select::<I, T> {
                items: HashMap::default(),
                barriers: HashMap::default(),
                ids: HashSet::default(),
            },
        )
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Operator<I, T> for Select<I, T>
where
    I: Id,
    T: Value + Clone,
{
    type Item<'a> = Item<&'a I, (Option<&'a T>, Option<&'a Condition<I>>)>;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let (data, condition) = item.data;

        // Update internal state
        if let Some(data) = data {
            self.items.insert(item.id.clone(), data.clone());
        } else {
            self.items.remove(item.id);
        }

        // In case there's a condition, register a new barrier
        if let Some(condition) = condition {
            self.barriers
                .insert(item.id.clone(), Barrier::new(condition.clone()));

            // Register items in barrier
            for id in &self.ids {
                if let Some(barrier) = self.barriers.get_mut(item.id) {
                    if !self.items.contains_key(id) {
                        barrier.insert(id);
                    }
                }
            }
        }

        // Check barriers
        let mut items = vec![];
        for (id, barrier) in &mut self.barriers {
            barrier.remove(item.id);
            if barrier.is_empty() {
                let iter = self.items.iter();
                let delta = iter
                    .filter(|(key, _)| barrier.satisfies(key))
                    .map(|(key, value)| {
                        Item::new((*key).clone(), Some(value.clone()))
                    })
                    .collect::<Delta<_, _>>();

                // Add delta to items
                items.push(Item::new(id.clone(), Some(delta)));
            }
        }

        // Return selected items.
        items
    }

    /// Notifies the operator of a signal.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn notify(&mut self, signal: Signal<I>) -> impl IntoOutputs<I> {
        if let Signal::Submit(id) = signal {
            self.ids.insert(id.clone());
            for barrier in self.barriers.values_mut() {
                barrier.insert(id);
            }
        }
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder() // fmt
            .interest(Interest::Submit)
            .build()
    }
}
