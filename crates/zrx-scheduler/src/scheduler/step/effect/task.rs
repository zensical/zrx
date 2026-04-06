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

//! Task.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use crate::scheduler::engine::Tag;
use crate::scheduler::step::{Result, Steps};

mod builder;

pub use builder::Builder;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Task function.
trait TaskFn<I>: Send {
    /// Executes the task function.
    ///
    /// # Errors
    ///
    /// Returns [`Error`][] if the function fails to execute.
    ///
    /// [`Error`]: crate::scheduler::action::Error
    fn execute(self: Box<Self>) -> Result<Steps<I>>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Task.
///
/// Tasks are intended to be run by an [`Executor`][], providing a convenient
/// way to encapsulate functionality that doesn't require shared state, moving
/// it to a worker thread for parallel execution. Tasks can capture variables
/// from their environment, and are designed to be executed once.
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of effects inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
///
/// [`Executor`]: zrx_executor::Executor
pub struct Task<I, C = ()> {
    /// Task function.
    function: Box<dyn TaskFn<I>>,
    /// Capture types.
    marker: PhantomData<C>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, C> Task<I, C> {
    /// Executes the task.
    ///
    /// # Errors
    ///
    /// Errors returned by the task are forwarded. Note that panics are not
    /// caught, as this should happen on higher levels for better control.
    #[inline]
    pub fn execute(self) -> Result<Steps<I, C>> {
        self.function.execute().map(Steps::tag)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, C> Tag<I, C> for Task<I, C> {
    type Target<T> = Task<I, T>;

    /// Tags the task with the given type.
    #[inline]
    fn tag<T>(self) -> Self::Target<T> {
        Task {
            function: self.function,
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Debug for Task<I, C> {
    /// Formats the task for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let function = "Box<dyn TaskFn>";
        f.debug_struct("Task")
            .field("function", &function)
            .field("marker", &self.marker)
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I, C> TaskFn<I> for (F, PhantomData<C>)
where
    F: FnOnce() -> Result<Steps<I, C>> + Send,
    C: Send,
{
    #[inline]
    fn execute(self: Box<Self>) -> Result<Steps<I>> {
        let (f, _) = *self;
        f().map(Steps::tag)
    }
}
