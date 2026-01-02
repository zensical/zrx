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

//! Work-sharing execution strategy.

use crossbeam::channel::{bounded, Sender};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, Builder, JoinHandle};
use std::{cmp, fmt, panic};

use crate::executor::strategy::Strategy;
use crate::executor::task::Task;
use crate::executor::Result;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Work-sharing execution strategy.
///
/// This strategy manages its tasks centrally in a single bounded [`crossbeam`]
/// channel, which pull tasks from it and execute them, repeating the process
/// until they are terminated. This is a very simple, yet reasonably efficient
/// strategy in most cases.
///
/// Tasks are processed in the exact same order they were submitted, albeit they
/// might not finish in the same order. As this strategy uses a bounded channel,
/// task submission might fail when the channel's capacity is reached, leading
/// to better performance characteristics due to the use of atomics over locks.
/// As an alternative, the [`WorkStealing`][] strategy can be used, which is
/// built on unbounded channels and allows for more flexible task submission,
/// including automatic load balancing between workers which is particularly
/// useful when tasks create subtasks.
///
/// [`WorkStealing`]: crate::executor::strategy::WorkStealing
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_executor::strategy::{Strategy, WorkSharing};
///
/// // Create strategy and submit task
/// let strategy = WorkSharing::default();
/// strategy.submit(Box::new(|| println!("Task")))?;
/// # Ok(())
/// # }
/// ```
pub struct WorkSharing {
    /// Task submission sender.
    sender: Option<Sender<Box<dyn Task>>>,
    /// Join handles of worker threads.
    threads: Vec<JoinHandle<()>>,
    /// Counter for running tasks.
    running: Arc<AtomicUsize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl WorkSharing {
    /// Creates a work-sharing execution strategy.
    ///
    /// This method creates a strategy with the given number of worker threads,
    /// which are spawned immediately before the method returns. Internally, a
    /// bounded channel is created with a capacity of 8 tasks per worker, so
    /// for 4 workers, the channel will have a capacity of 32 tasks.
    ///
    /// Use [`WorkSharing::with_capacity`] to set a custom capacity.
    ///
    /// # Panics
    ///
    /// Panics if thread creation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::WorkSharing;
    ///
    /// // Create strategy
    /// let strategy = WorkSharing::new(4);
    /// ```
    #[must_use]
    pub fn new(num_workers: usize) -> Self {
        Self::with_capacity(num_workers, 8 * num_workers)
    }

    /// Creates a work-sharing execution strategy with the given capacity.
    ///
    /// This method creates a strategy with the given number of worker threads,
    /// which are spawned immediately before the method returns.
    ///
    /// This strategy makes use of a bounded channel for its better performance
    /// characteristics, since the caller is expected to have control over task
    /// submission, ensuring that the executor can accept new tasks. The given
    /// capacity sets the number of tasks the executor accepts before starting
    /// to reject them, which can be used to apply backpressure. Note that the
    /// capacity is not a per-worker, but a global per-executor limit.
    ///
    /// # Panics
    ///
    /// Panics if thread creation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::WorkSharing;
    ///
    /// // Create strategy with capacity
    /// let strategy = WorkSharing::with_capacity(4, 64);
    /// ```
    #[must_use]
    pub fn with_capacity(num_workers: usize, capacity: usize) -> Self {
        let (sender, receiver) = bounded::<Box<dyn Task>>(capacity);

        // Keep track of running tasks
        let running = Arc::new(AtomicUsize::new(0));

        // Initialize worker threads
        let iter = (0..num_workers).map(|index| {
            let receiver = receiver.clone();

            // Create worker thread and poll the receiver until the sender is
            // dropped, automatically exiting the loop. Additionally, we keep
            // track of the number of running tasks to provide a simple way to
            // monitor the load of the thread pool.
            let running = Arc::clone(&running);
            let h = move || {
                while let Ok(task) = receiver.recv() {
                    running.fetch_add(1, Ordering::Release);

                    // Execute task and immediately execute all subtasks on the
                    // same worker, if any, as the work-sharing strategy has no
                    // means of distributing work to other workers threads. We
                    // also keep the running count due to sequential execution,
                    // and catch panics, as we're running user-land code that
                    // might be sloppy. However, since the executor has no way
                    // of reporting panics, tasks should wrap execution as we
                    // do here, and abort with a proper error.
                    let _ = panic::catch_unwind(|| {
                        let subtasks = task.execute();
                        if !subtasks.is_empty() {
                            // Execution is recursive, so in case a subtask has
                            // further subtasks, they are executed depth-first
                            subtasks.execute();
                        }
                    });

                    // Update number of running tasks
                    running.fetch_sub(1, Ordering::Acquire);
                }
            };

            // We deliberately use unwrap here, as the capability to spawn
            // threads is a fundamental requirement of the executor
            Builder::new()
                .name(format!("zrx/executor/{}", index + 1))
                .spawn(h)
                .unwrap()
        });

        // Create worker threads and return strategy
        let threads = iter.collect();
        Self {
            sender: Some(sender),
            threads,
            running,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Strategy for WorkSharing {
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
    /// If the task cannot be submitted, [`Error::Submit`][] is returned, which
    /// can only happen if the channel is disconnected or at capacity.
    ///
    /// [`Error::Submit`]: crate::executor::Error::Submit
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_executor::strategy::{Strategy, WorkSharing};
    ///
    /// // Create strategy and submit task
    /// let strategy = WorkSharing::default();
    /// strategy.submit(Box::new(|| println!("Task")))?;
    /// # Ok(())
    /// # }
    /// ```
    fn submit(&self, task: Box<dyn Task>) -> Result {
        match self.sender.as_ref() {
            Some(sender) => Ok(sender.try_send(task)?),
            None => unreachable!(),
        }
    }

    /// Returns the number of workers.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Strategy, WorkSharing};
    ///
    /// // Get number of workers
    /// let strategy = WorkSharing::new(1);
    /// assert_eq!(strategy.num_workers(), 1);
    /// ```
    #[inline]
    fn num_workers(&self) -> usize {
        self.threads.len()
    }

    /// Returns the number of running tasks.
    ///
    /// This method allows to monitor the worker load, as it returns how many
    /// workers are currently actively executing tasks.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Strategy, WorkSharing};
    ///
    /// // Get number of running tasks
    /// let strategy = WorkSharing::default();
    /// assert_eq!(strategy.num_tasks_running(), 0);
    /// ```
    #[inline]
    fn num_tasks_running(&self) -> usize {
        self.running.load(Ordering::Relaxed)
    }

    /// Returns the number of pending tasks.
    ///
    /// This method allows to throttle the submission of tasks, as it returns
    /// how many tasks are currently waiting to be executed.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Strategy, WorkSharing};
    ///
    /// // Get number of pending tasks
    /// let strategy = WorkSharing::default();
    /// assert_eq!(strategy.num_tasks_pending(), 0);
    /// ```
    #[inline]
    fn num_tasks_pending(&self) -> usize {
        self.sender.as_ref().map_or(0, Sender::len)
    }

    /// Returns the capacity, if bounded.
    ///
    /// This method returns the maximum number of tasks that can be submitted
    /// at once, which can be used by the strategy for applying backpressure.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Strategy, WorkSharing};
    ///
    /// // Get capacity
    /// let strategy = WorkSharing::default();
    /// assert!(strategy.capacity() >= Some(strategy.num_workers()));
    /// ```
    #[inline]
    fn capacity(&self) -> Option<usize> {
        self.sender.as_ref().and_then(Sender::capacity)
    }
}

// ----------------------------------------------------------------------------

impl Default for WorkSharing {
    /// Creates a work-sharing execution strategy using all CPUs - 1.
    ///
    /// The number of workers is determined by the number of logical CPUs minus
    /// one, which reserves one core for the main thread for orchestration. If
    /// the number of logical CPUs is fewer than 1, the strategy defaults to a
    /// single worker thread.
    ///
    /// __Warning__: this method makes use of [`thread::available_parallelism`]
    /// to determine the number of available cores, which has some limitations.
    /// Please refer to the documentation of that function for more details, or
    /// consider using [`num_cpus`][] as an alternative.
    ///
    /// [`num_cpus`]: https://crates.io/crates/num_cpus
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::WorkSharing;
    ///
    /// // Create strategy
    /// let strategy = WorkSharing::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new(cmp::max(
            thread::available_parallelism()
                .map(|num| num.get().saturating_sub(1))
                .unwrap_or(1),
            1,
        ))
    }
}

impl Drop for WorkSharing {
    /// Terminates and joins all worker threads.
    ///
    /// This method waits for all worker threads to finish executing currently
    /// running tasks, while ignoring any pending tasks. All worker threads are
    /// joined before the method returns. This is necessary to prevent worker
    /// threads from running after the strategy has been dropped.
    fn drop(&mut self) {
        // Dropping the sender causes all receivers to terminate
        if let Some(sender) = self.sender.take() {
            drop(sender);
        }

        // Join all worker threads without panicking on errors
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

// ----------------------------------------------------------------------------

impl fmt::Debug for WorkSharing {
    /// Formats the execution strategy for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WorkSharing")
            .field("workers", &self.num_workers())
            .field("running", &self.num_tasks_running())
            .field("pending", &self.num_tasks_pending())
            .finish()
    }
}
