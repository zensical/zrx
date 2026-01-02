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

//! Schedulable action.

use std::marker::PhantomData;

use zrx_scheduler::action::input::TryFromInputItem;
use zrx_scheduler::action::output::{IntoOutputs, Outputs};
use zrx_scheduler::action::{Descriptor, Error, Input, Result};
use zrx_scheduler::{Action, Id};

use crate::stream::operator::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Schedulable action.
///
/// This struct wraps an operator implementation, allowing it to be used as a
/// type-erased action in the scheduler. It implements the [`Action`] trait and
/// represents the executable form of a stream operator. The type marker `T` is
/// used to bind the type of the operator.
pub struct Schedulable<O, T> {
    /// Operator.
    operator: O,
    /// Type marker.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<O, T> Schedulable<O, T> {
    /// Creates a schedulable action.
    #[must_use]
    pub fn new(operator: O) -> Self {
        Self { operator, marker: PhantomData }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, O, T> Action<I> for Schedulable<O, T>
where
    I: Id,
    O: Operator<I, T>,
{
    /// Executes the action with the given input.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn execute(&mut self, input: Input<I>) -> Result<Outputs<I>> {
        match input {
            // Handle item
            Input::Item(item) => match O::Item::try_from_input_item(item) {
                Ok(item) => self.operator.handle(item).into_outputs(),
                Err(err) => Err(Error::Value(err)),
            },

            // Handle signal
            Input::Signal(signal) => {
                self.operator.notify(signal).into_outputs()
            }
        }
    }

    /// Returns the descriptor of the action.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        self.operator.descriptor()
    }
}
