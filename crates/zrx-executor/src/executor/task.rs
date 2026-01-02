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

use std::fmt;
use std::panic::UnwindSafe;

mod collection;

pub use collection::Tasks;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Task.
///
/// Tasks are units of work that can be submitted to an [`Executor`][], which
/// forwards them for execution to an execution [`Strategy`][] that is set when
/// creating the [`Executor`][]. Moreover, tasks can create and return further
/// [`Tasks`], which are handled by the same execution strategy, allowing for
/// immediate or deferred execution, as implemented by the strategy. If a task
/// panics, it doesn't take the worker thread or executor with it.
///
/// Note that tasks will almost always need to capture environment variables,
/// which is why they're created from [`FnOnce`] and must be [`Send`].
///
/// [`Executor`]: crate::executor::Executor
/// [`Strategy`]: crate::executor::strategy::Strategy
pub trait Task: Send + UnwindSafe + 'static {
    /// Executes the task.
    ///
    /// This methods executes the task, and may return further tasks as part of
    /// a task collection, which are executed by the same execution strategy.
    /// As task execution must be infallible, tasks might use channels in order
    /// to communicate results or errors back to the main thread.
    fn execute(self: Box<Self>) -> Tasks;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<T> for Box<dyn Task>
where
    T: Task,
{
    /// Creates a boxed task from a task.
    ///
    /// This implementation ensures we can comfortably pass bare closures, as
    /// well as boxed tasks to [`Executor::submit`][], which allos to resubmit
    /// tasks that were returned as part of [`Error::Submit`][] due to capacity
    /// limits of the execution strategy.
    ///
    /// [`Executor::submit`]: crate::executor::Executor::submit
    /// [`Error::Submit`]: crate::executor::Error::Submit
    #[inline]
    fn from(task: T) -> Self {
        Box::new(task)
    }
}

// ----------------------------------------------------------------------------

impl fmt::Debug for Box<dyn Task> {
    /// Formats the task for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("Box<dyn Task>")
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R> Task for F
where
    F: FnOnce() -> R + Send + UnwindSafe + 'static,
    R: Into<Tasks>,
{
    #[inline]
    fn execute(self: Box<Self>) -> Tasks {
        self().into()
    }
}
