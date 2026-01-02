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

//! Action.

use std::fmt;

pub mod descriptor;
mod error;
pub mod input;
pub mod output;
pub mod report;

pub use descriptor::Descriptor;
pub use error::{Error, Result};
pub use input::Input;
use output::IntoOutputs;
pub use output::{Output, Outputs};
pub use report::Report;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Action.
///
/// Actions represent units of work that can be executed by a [`Scheduler`][],
/// and which can hold state between executions, which is also why the execution
/// of an action requires a mutable reference to the action. Any data passed to
/// and returned from an action needs to be type-erased.
///
/// Implementors can embed types into the implementation of the action, which
/// can use the [`TryFromValues`][] trait to convert [`Values`][] contained in
/// the [`Input`] into the expected types. The scheduler as such only ensures
/// the correct order of execution, not the type safety of the data. Downstream
/// actions can receive data from multiple upstream actions, which allows to
/// model complex workflows using a [`Graph`][], created with a [`Builder`][]
/// that is provided to aid in graph construction, representing the workflow of
/// actions. Note that actions themselves are not aware of the structure of the
/// underlying graph they're part of, which is why the [`Builder`][] helps to
/// ensure type safety and coherence.
///
/// [`Builder`]: crate::scheduler::graph::Builder
/// [`Graph`]: crate::scheduler::graph::Graph
/// [`Scheduler`]: crate::scheduler::Scheduler
/// [`TryFromValues`]: crate::scheduler::value::TryFromValues
/// [`Values`]: crate::scheduler::value::Values
pub trait Action<I> {
    /// Executes the action.
    ///
    /// This method executes the action, and is expected to return any number
    /// of [`Outputs`] that will then be handled accordingly by the scheduler.
    ///
    /// # Errors
    ///
    /// Errors returned by the action are forwarded.
    fn execute(&mut self, input: Input<I>) -> Result<Outputs<I>>;

    /// Returns the descriptor.
    ///
    /// This method returns the descriptor of the action, describing properties
    /// of the action that can be used by the scheduler to optimize execution,
    /// as well as interests of the action to instruct the scheduler to send
    /// certain signals to it.
    ///
    /// The default implementation returns a descriptor that marks the action
    /// as neither pure nor stable, and without interests, the safest default.
    /// Note that we always return a new descriptor, since descriptors might
    /// hold dynamically allocated data that is specific to the action.
    #[inline]
    fn descriptor(&self) -> Descriptor {
        Descriptor::default()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> fmt::Debug for Box<dyn Action<I>> {
    /// Formats the action for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Box<dyn Action>")
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I> Action<I> for F
where
    F: FnMut(Input<I>) -> R,
    R: IntoOutputs<I>,
{
    #[inline]
    fn execute(&mut self, input: Input<I>) -> Result<Outputs<I>> {
        self(input).into_outputs()
    }
}
