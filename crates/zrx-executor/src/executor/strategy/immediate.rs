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

//! Immediate execution strategy.

use std::fmt;

use crate::executor::strategy::Strategy;
use crate::executor::task::Task;
use crate::executor::Result;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Immediate execution strategy.
///
/// This strategy executes a given task immediately, and is primarily intended
/// for testing and debugging purposes. No threading is involved, which makes
/// it much easier to reason about execution order. Additionally, it can act
/// as a baseline for comparison with worker-based strategies.
///
/// __Warning__: When channels are involved, this strategy might deadlock, i.e.,
/// when a task from a worker thread tries to send on a bounded channel that is
/// full. In this case, make sure to either use unbounded channels, or actively
/// drain the channel before sending on it.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_executor::strategy::{Immediate, Strategy};
///
/// // Create strategy and submit task
/// let strategy = Immediate::default();
/// strategy.submit(Box::new(|| println!("Task")))?;
/// # Ok(())
/// # }
/// ```
pub struct Immediate;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Immediate {
    /// Creates an immediate execution strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::Immediate;
    ///
    /// // Create strategy
    /// let strategy = Immediate::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Strategy for Immediate {
    /// Submits a task.
    ///
    /// This method immediately executes the given [`Task`], so that it runs on
    /// the main thread, which is primarily intended for testing and debugging.
    ///
    /// Note that tasks are intended to only run once, which is why they are
    /// consumed. If a task needs to be run multiple times, it must be wrapped
    /// in a closure that creates a new task each time. This allows for safe
    /// sharing of state between tasks.
    ///
    /// # Errors
    ///
    /// This method is infallible, and will always return [`Ok`].
    ///
    /// Errors
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_executor::strategy::{Immediate, Strategy};
    ///
    /// // Create strategy and submit task
    /// let strategy = Immediate::default();
    /// strategy.submit(Box::new(|| println!("Task")))?;
    /// # Ok(())
    /// # }
    /// ```
    fn submit(&self, task: Box<dyn Task>) -> Result {
        let subtasks = task.execute();
        if !subtasks.is_empty() {
            // Subtasks are executed recursively, so in case a subtask produces
            // further subtasks, they are executed in depth-first order
            subtasks.execute();
        }

        // No errors occurred
        Ok(())
    }

    /// Returns the number of workers.
    ///
    /// The number of usable workers is always `1` for this strategy, as tasks
    /// are immediately executed on the main thread when submitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Immediate, Strategy};
    ///
    /// // Get number of workers
    /// let strategy = Immediate::new();
    /// assert_eq!(strategy.num_workers(), 1);
    /// ```
    #[inline]
    fn num_workers(&self) -> usize {
        1
    }

    /// Returns the number of running tasks.
    ///
    /// The number of running tasks is always `0` for this strategy, as tasks
    /// are immediately executed on the main thread when submitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Immediate, Strategy};
    ///
    /// // Get number of running tasks
    /// let strategy = Immediate::default();
    /// assert_eq!(strategy.num_tasks_running(), 0);
    /// ```
    #[inline]
    fn num_tasks_running(&self) -> usize {
        0
    }

    /// Returns the number of pending tasks.
    ///
    /// The number of pending tasks is always `0` for this strategy, as tasks
    /// are immediately executed on the main thread when submitted.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Immediate, Strategy};
    ///
    /// // Get number of pending tasks
    /// let strategy = Immediate::default();
    /// assert_eq!(strategy.num_tasks_pending(), 0);
    /// ```
    #[inline]
    fn num_tasks_pending(&self) -> usize {
        0
    }

    /// Returns the capacity, if bounded.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Immediate, Strategy};
    ///
    /// // Get capacity
    /// let strategy = Immediate::default();
    /// assert_eq!(strategy.capacity(), None);
    /// ```
    #[inline]
    fn capacity(&self) -> Option<usize> {
        None
    }
}

// ----------------------------------------------------------------------------

impl Default for Immediate {
    /// Creates an immediate execution strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::Immediate;
    ///
    /// // Create strategy
    /// let strategy = Immediate::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl fmt::Debug for Immediate {
    /// Formats the execution strategy for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Immediate")
            .field("workers", &self.num_workers())
            .field("running", &self.num_tasks_running())
            .field("pending", &self.num_tasks_pending())
            .finish()
    }
}
