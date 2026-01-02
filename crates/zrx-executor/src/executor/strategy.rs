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

//! Execution strategies.

use std::fmt::Debug;

use super::error::Result;
use super::signal::Signal;
use super::task::Task;

mod immediate;
mod worker;

pub use immediate::Immediate;
pub use worker::{WorkSharing, WorkStealing};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Execution strategy.
///
/// Besides task submission, strategies must also allow to inspect the number of
/// workers, as well as the number of running and pending tasks, which might be
/// useful for implementing more fine-grained execution strategies. It's also
/// used to determine whether an [`Executor`][] is idle or at capacity.
///
/// [`Executor`]: crate::executor::Executor
pub trait Strategy: Debug {
    /// Submits a task.
    ///
    /// This method submits a [`Task`], which should be executed by one of the
    /// worker threads as soon as possible. How and when the task is executed,
    /// and in what order tasks are executed, is entirely up to the strategy
    /// implementation.
    ///
    /// # Errors
    ///
    /// This method should return an error when a problem is encountered trying
    /// to submit the given task, but not within the task itself.
    fn submit(&self, task: Box<dyn Task>) -> Result;

    /// Returns the number of workers.
    fn num_workers(&self) -> usize;

    /// Returns the number of running tasks.
    fn num_tasks_running(&self) -> usize;

    /// Returns the number of pending tasks.
    fn num_tasks_pending(&self) -> usize;

    /// Returns the capacity, if bounded.
    fn capacity(&self) -> Option<usize>;
}
