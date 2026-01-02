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

use crate::scheduler::action::output::{IntoOutputs, Outputs};
use crate::scheduler::action::Result;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Task function.
pub trait TaskFn<I>: Send {
    /// Executes the task function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(self: Box<Self>) -> Result<Outputs<I>>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Task.
///
/// Tasks are designed to be run by an [`Executor`][], providing a convenient
/// way to encapsulate functionality that can be executed independently, and
/// moved to a worker thread for execution. Tasks can capture variables from
/// their environment, and are designed to be executed once.
///
/// [`Executor`]: zrx_executor::Executor
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_scheduler::effect::{Item, Task};
///
/// // Create task returning an item
/// let task = Task::new(|| {
///     Item::new("id", Some(42))
/// });
///
/// // Execute task
/// task.execute()?;
/// # Ok(())
/// # }
/// ```
pub struct Task<I> {
    /// Task function.
    function: Box<dyn TaskFn<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Task<I> {
    /// Creates a task.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::{Item, Task};
    ///
    /// // Create task returning an item
    /// let task = Task::new(|| {
    ///     Item::new("id", Some(42))
    /// });
    /// ```
    pub fn new<F>(f: F) -> Self
    where
        F: TaskFn<I> + 'static,
    {
        Self { function: Box::new(f) }
    }

    /// Executes the task.
    ///
    /// # Errors
    ///
    /// Errors returned by the task are forwarded. Note that panics are not
    /// caught, as this should happen on higher levels for better control.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::effect::{Item, Task};
    ///
    /// // Create task returning an item
    /// let task = Task::new(|| {
    ///     Item::new("id", Some(42))
    /// });
    ///
    /// // Execute task
    /// task.execute()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn execute(self) -> Result<Outputs<I>> {
        self.function.execute()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> fmt::Debug for Task<I> {
    /// Formats the task for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let function = "Box<dyn TaskFn>";
        f.debug_struct("Task") // fmt
            .field("function", &function)
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I> TaskFn<I> for F
where
    F: FnOnce() -> R + Send,
    R: IntoOutputs<I>,
{
    #[inline]
    fn execute(self: Box<Self>) -> Result<Outputs<I>> {
        self().into_outputs()
    }
}
