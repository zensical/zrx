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

//! Transpose operator.

use std::marker::PhantomData;

use zrx_scheduler::action::descriptor::Property;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::{Item, Task};
use zrx_scheduler::{Id, Value};

use crate::stream::value::Delta;
use crate::stream::Stream;

use super::{Operator, OperatorExt};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Transpose operator.
struct Transpose<T> {
    /// Type marker.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, Delta<I, T>>
where
    I: Id,
    T: Value + Clone,
{
    pub fn transpose(&self) -> Stream<I, Delta<I, T>> {
        self.with_operator(Transpose { marker: PhantomData })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Operator<I, Delta<I, T>> for Transpose<T>
where
    I: Id,
    T: Value + Clone,
{
    type Item<'a> = Item<&'a I, &'a Delta<I, T>>;

    /// Handles the given item.
    ///
    /// Transposition is implemented as a swap of the identifiers of a delta of
    /// items, by hoisting the inner items to the outer level, and wrapping the
    /// associated data of the inner items with the outer item. This allows to
    /// effectively invert relationships, adhering to differential semantics.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, fields(id = %item.id))
    )]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {
        let item = item.into_owned();
        Task::new(move || {
            let iter = item.data.into_iter().map(|part| {
                let inner = Item::new(item.id.clone(), part.data);
                Item {
                    id: part.id,
                    data: Some(Delta::from([inner])),
                }
            });

            // Return delta of items
            iter.collect::<Vec<_>>()
        })
    }

    /// Returns the descriptor.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::builder() // fmt
            .property(Property::Pure)
            .build()
    }
}
