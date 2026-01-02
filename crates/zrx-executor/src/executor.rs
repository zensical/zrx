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

//! Executor.

use std::rc::Rc;
use std::thread;
use std::time::Duration;

mod error;
mod signal;
pub mod strategy;
pub mod task;

pub use error::{Error, Result};
use strategy::{Strategy, WorkSharing};
use task::Task;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Executor.
///
/// Executors serve as the primary interface for submitting and monitoring tasks
/// within the system. They act as a frontend to various execution [`Strategy`]
/// implementations, which define how tasks are prioritized and executed. Each
/// execution [`Strategy`] encapsulates an implementation that determines the
/// order and concurrency of execution. Abstracting the execution mechanism
/// allows for flexible and interchangeable task management strategies.
///
/// Additionally, executors implement [`Clone`], which allows to easily share
/// them among different parts of the system without borrowing issues.
///
/// Note that executors are not responsible for managing the lifetime of tasks,
/// as it is assumed that tasks are self-contained and can be run independently.
/// If a [`Task`] is submitted to an executor, it can't be cancelled or stopped,
/// as the executor is not aware of the task's internal state. However, callers
/// can implement fine-grained execution strategies on top of the executor to
/// gain fine-grained control over task execution.
///
/// This is an opinionated implementation that specifically targets the needs of
/// our execution model. It is not meant to be a general-purpose executor.
///
/// # Examples
///
/// Create an executor spawning 1,000 tasks using all CPUs - 1:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use std::thread;
/// use std::time::Duration;
/// use zrx_executor::strategy::WorkStealing;
/// use zrx_executor::Executor;
///
/// // Create executor with strategy
/// let strategy = WorkStealing::default();
/// let executor = Executor::new(strategy);
///
/// // Create 1,000 tasks taking 20ms each
/// for _ in 0..1000 {
///     executor.submit(|| {
///         thread::sleep(Duration::from_millis(20));
///     })?;
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Executor<S>
where
    S: Strategy,
{
    // Execution strategy.
    strategy: Rc<S>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<S> Executor<S>
where
    S: Strategy,
{
    /// Creates an executor.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::WorkSharing;
    /// use zrx_executor::Executor;
    ///
    /// // Create executor with strategy
    /// let executor = Executor::new(WorkSharing::default());
    /// ```
    #[must_use]
    pub fn new(strategy: S) -> Self {
        Self { strategy: Rc::new(strategy) }
    }

    /// Submits a task.
    ///
    /// This method submits a [`Task`], which is executed by one of the worker
    /// threads as soon as possible. If a task computes a result, a [`Sender`][]
    /// can be shared with the task, to send the result back to the caller,
    /// which can then poll a [`Receiver`][].
    ///
    /// Note that tasks are intended to only run once, which is why they are
    /// consumed. If a task needs to be run multiple times, it must be wrapped
    /// in a closure that creates a new task each time. This allows for safe
    /// sharing of state between tasks.
    ///
    /// [`Receiver`]: crossbeam::channel::Receiver
    /// [`Sender`]: crossbeam::channel::Sender
    ///
    /// # Errors
    ///
    /// If the executor encounters a problem during task submission, it will
    /// forward the encountered error to the caller, returning the task. Most
    /// likely, the underlying execution strategy is at capacity, which means
    /// the caller should resubmit the task at a later time. This is possible,
    /// since this method accepts any type that implements the [`Task`] trait
    /// and converts it into a boxed task.
    ///
    /// # Examples
    ///
    /// Submit a task:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_executor::Executor;
    ///
    /// // Create executor and submit task
    /// let executor = Executor::default();
    /// executor.submit(|| println!("Task"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Submit a task returning subtasks:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_executor::Executor;
    ///
    /// // Create executor and submit task
    /// let executor = Executor::default();
    /// executor.submit(|| {
    ///     println!("Task 1");
    ///     || {
    ///         println!("Task 1.1");
    ///         || {
    ///             println!("Task 1.1.1");
    ///         }
    ///     }
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Submit a task returning a task collection:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_executor::task::Tasks;
    /// use zrx_executor::Executor;
    ///
    /// // Create executor and submit task
    /// let executor = Executor::default();
    /// executor.submit(|| {
    ///     println!("Task 1");
    ///
    ///     // Create subtasks
    ///     let mut tasks = Tasks::new();
    ///     tasks.add(|| println!("Task 1.1"));
    ///     tasks.add(|| println!("Task 1.2"));
    ///     tasks.add(|| println!("Task 1.3"));
    ///     tasks
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn submit<T>(&self, task: T) -> Result
    where
        T: Into<Box<dyn Task>>,
    {
        self.strategy.submit(task.into())
    }

    /// Waits for all tasks to finish.
    ///
    /// This method blocks the current thread until all submitted running and
    /// pending tasks have been completed. Calling this method is not necessary,
    /// as it's called automatically when the executor is dropped, but it might
    /// be helpful for testing and debugging purposes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::thread;
    /// use std::time::Duration;
    /// use zrx_executor::strategy::WorkStealing;
    /// use zrx_executor::Executor;
    ///
    /// // Create executor with strategy
    /// let strategy = WorkStealing::default();
    /// let executor = Executor::new(strategy);
    ///
    /// // Create 1,000 tasks taking 20ms each
    /// for _ in 0..1000 {
    ///     executor.submit(|| {
    ///         thread::sleep(Duration::from_millis(20));
    ///     })?;
    /// }
    ///
    /// // Wait for all tasks to finish
    /// executor.wait();
    /// assert!(executor.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub fn wait(&self) {
        let duration = Duration::from_millis(10);
        while !self.is_empty() {
            thread::sleep(duration);
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<S> Executor<S>
where
    S: Strategy,
{
    /// Returns the number of tasks.
    ///
    /// This method returns the total number of tasks currently managed by the
    /// executor, which includes running as well as pending tasks.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Get number of tasks
    /// let executor = Executor::default();
    /// assert_eq!(executor.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.num_tasks_running() + self.num_tasks_pending()
    }

    /// Returns whether there are any tasks.
    ///
    /// This method checks whether the executor has running or pending tasks,
    /// and if not, considers the executor as idle. It's particularly useful
    /// for waiting until an executor has processed all tasks, which is
    /// necessary for implementing schedulers on top of executors.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Check whether executor is idle
    /// let executor = Executor::default();
    /// assert!(executor.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns whether the executor is saturated.
    ///
    /// This method checks whether the executor is at capacity, which means
    /// task submission will fail until a worker has finished a task.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Check whether executor is saturated
    /// let executor = Executor::default();
    /// assert!(!executor.is_saturated());
    /// ```
    #[inline]
    pub fn is_saturated(&self) -> bool {
        self.capacity()
            .is_some_and(|capacity| self.num_tasks_pending() >= capacity)
    }

    /// Returns the number of workers.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::WorkSharing;
    /// use zrx_executor::Executor;
    ///
    /// // Get number of workers
    /// let executor = Executor::new(WorkSharing::new(1));
    /// assert_eq!(executor.num_workers(), 1);
    /// ```
    #[inline]
    pub fn num_workers(&self) -> usize {
        self.strategy.num_workers()
    }

    /// Returns the number of running tasks.
    ///
    /// This method allows to monitor the worker load, as it returns how many
    /// workers are currently actively executing tasks.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Get number of running tasks
    /// let executor = Executor::default();
    /// assert_eq!(executor.num_tasks_running(), 0);
    /// ```
    #[inline]
    pub fn num_tasks_running(&self) -> usize {
        self.strategy.num_tasks_running()
    }

    /// Returns the number of pending tasks.
    ///
    /// This method allows to throttle the submission of tasks, as it returns
    /// how many tasks are currently waiting to be executed.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Get number of pending tasks
    /// let executor = Executor::default();
    /// assert_eq!(executor.num_tasks_pending(), 0);
    /// ```
    #[inline]
    pub fn num_tasks_pending(&self) -> usize {
        self.strategy.num_tasks_pending()
    }

    /// Returns the capacity, if bounded.
    ///
    /// This method returns the maximum number of tasks that can be submitted
    /// at once, which can be used by the strategy for applying backpressure.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Get maximum number of tasks
    /// let executor = Executor::default();
    /// assert!(executor.capacity() >= Some(executor.num_workers()));
    /// ```
    #[inline]
    pub fn capacity(&self) -> Option<usize> {
        self.strategy.capacity()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<S> Clone for Executor<S>
where
    S: Strategy,
{
    /// Clones the executor.
    ///
    /// This method creates a new executor with the same execution strategy,
    /// which allows to share them without borrowing issues.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Create and clone executor
    /// let executor = Executor::default();
    /// executor.clone();
    /// ```
    #[inline]
    fn clone(&self) -> Self {
        Self {
            strategy: Rc::clone(&self.strategy),
        }
    }
}

impl Default for Executor<WorkSharing> {
    /// Creates an executor using the default work-sharing strategy.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::Executor;
    ///
    /// // Create executor
    /// let executor = Executor::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new(WorkSharing::default())
    }
}

impl<S> Drop for Executor<S>
where
    S: Strategy,
{
    /// Waits for all tasks to finish.
    fn drop(&mut self) {
        self.wait();
    }
}
