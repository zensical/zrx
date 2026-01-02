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

//! Stream operators.

#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

use zrx_scheduler::action::input::TryFromInputItem;
use zrx_scheduler::action::output::IntoOutputs;
use zrx_scheduler::action::Descriptor;
use zrx_scheduler::effect::Signal;
use zrx_scheduler::{Id, Value};

use super::Stream;

mod audit;
mod chunks;
mod coalesce;
mod count;
mod debounce;
mod delta_count;
mod delta_filter;
mod delta_filter_map;
mod delta_map;
mod delta_reduce;
mod difference;
mod fill;
mod filter;
mod filter_map;
mod group;
mod inspect;
mod intersection;
mod join;
mod join_filter;
mod join_filter_map;
mod join_map;
mod lift;
mod map;
mod product;
mod reduce;
mod sample;
mod select;
mod sort;
mod throttle;
mod transpose;
mod union;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Operator.
#[allow(unused_variables)]
pub trait Operator<I, T> {
    /// Item type handled by operator.
    type Item<'a>: TryFromInputItem<'a, I>;

    /// Handles the given item.
    ///
    /// This method is called by the scheduler to handle an item from a stream,
    /// and receives a mutable reference, since operators are allowed to change
    /// their internal state at any given time. Anything which can be converted
    /// into [`Outputs`][] can be returned, e.g., items, signals and tasks.
    ///
    /// We deliberately decided to use an RPIT (return-position impl trait), as
    /// it's the most convenient to work with, instead of requiring yet another
    /// associated type to be defined.
    ///
    /// [`Outputs`]: zrx_scheduler::action::Outputs
    #[inline]
    fn handle(&mut self, item: Self::Item<'_>) -> impl IntoOutputs<I> {}

    /// Notifies the operator of a signal.
    ///
    /// This method allows the operator to react to system messages or custom
    /// events, which can be used to change its internal state or to trigger
    /// specific actions. The default implementation just swallows the signal,
    /// which is suitable for most cases.
    ///
    /// As with [`Operator::handle`], this method is allowed to return anything
    /// that can be converted into [`Outputs`][], expecting it to emit one or
    /// more output items, further signals, tasks or nothing at all.
    ///
    /// __Warning__: only implement this method if the operator is expected to
    /// react to signals. Otherwise, the default implementation is sufficient.
    ///
    /// [`Outputs`]: zrx_scheduler::action::Outputs
    #[inline]
    fn notify(&mut self, signal: Signal<I>) -> impl IntoOutputs<I> {}

    /// Returns the descriptor of the operator.
    fn descriptor(&self) -> Descriptor;
}

// ----------------------------------------------------------------------------

/// Operator extension trait.
pub trait OperatorExt<I, T, U>
where
    I: Id,
    T: Value,
{
    /// Applies the given operator and returns a stream.
    fn with_operator<O>(&self, operator: O) -> Stream<I, U>
    where
        O: Operator<I, T> + 'static,
        U: Value;
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T, U> OperatorExt<I, T, U> for Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Applies the given operator and returns a stream.
    #[inline]
    fn with_operator<O>(&self, operator: O) -> Stream<I, U>
    where
        O: Operator<I, T> + 'static,
        U: Value,
    {
        self.workflow.add_operator([self.id], operator)
    }
}
