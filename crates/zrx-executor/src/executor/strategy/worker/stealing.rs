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

//! Work-stealing execution strategy.

use crossbeam::deque::{Injector, Steal, Stealer, Worker};
use std::iter::repeat_with;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, Builder, JoinHandle};
use std::{cmp, fmt, panic};

use crate::executor::strategy::{Signal, Strategy};
use crate::executor::task::Task;
use crate::executor::Result;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Work-stealing execution strategy.
///
/// This strategy implements work-stealing, where each worker thread has its own
/// local queue. Workers can steal tasks from a central injector or from other
/// workers, if their local queues are empty. This allows for more efficient
/// execution if there's a large number of workers and tasks.
///
/// Work stealing enhances load balancing by allowing idle workers to take on
/// tasks from busier peers, which helps to reduce idle time and can improve
/// overall throughput. Unlike the simpler [`WorkSharing`][] strategy that uses
/// a central queue where workers may become stuck waiting for new tasks, this
/// method can yield better resource utilization. Additionally, work-stealing
/// helps mitigate contention over shared resources (i.e. channels), which can
/// become a bottleneck in central queueing systems, allowing each worker to
/// primarily operate on its local queue. However, if task runtimes are short,
/// the utilization can be lower than with a central queue, since stealing uses
/// an optimistic strategy and is a best-effort operation. When task runtimes
/// are long, work stealing can be more efficient than central queueing.
///
/// This reduced contention is particularly beneficial in dynamic environments
/// with significantly fluctuating workloads, enabling faster task completion
/// as workers can quickly adapt to take on shorter or less complex tasks as
/// they become available.
///
/// [`WorkSharing`]: crate::executor::strategy::WorkSharing
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_executor::strategy::{Strategy, WorkStealing};
///
/// // Create strategy and submit task
/// let strategy = WorkStealing::default();
/// strategy.submit(Box::new(|| println!("Task")))?;
/// # Ok(())
/// # }
/// ```
pub struct WorkStealing {
    /// Injector for task submission.
    injector: Arc<Injector<Box<dyn Task>>>,
    /// Signal for synchronization.
    signal: Arc<Signal>,
    /// Join handles of worker threads.
    threads: Vec<JoinHandle<Result>>,
    /// Counter for running tasks.
    running: Arc<AtomicUsize>,
    /// Counter for pending tasks.
    pending: Arc<AtomicUsize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl WorkStealing {
    /// Creates a work-stealing execution strategy.
    ///
    /// This method creates a strategy with the given number of worker threads,
    /// which are spawned immediately before the method returns. Note that this
    /// strategy uses an unbounded channel, so there're no capacity limits as
    /// for the [`WorkSharing`][] execution strategy.
    ///
    /// [`WorkSharing`]: crate::executor::strategy::WorkSharing
    ///
    /// # Panics
    ///
    /// Panics if thread creation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::WorkStealing;
    ///
    /// // Create strategy
    /// let strategy = WorkStealing::new(4);
    /// ```
    #[must_use]
    pub fn new(num_workers: usize) -> Self {
        let injector = Arc::new(Injector::new());
        let signal = Arc::new(Signal::new());

        // Create worker queues
        let mut workers = Vec::with_capacity(num_workers);
        for _ in 0..num_workers {
            workers.push(Worker::new_fifo());
        }

        // Obtain stealers from worker queues - note that we collect stealers
        // into a slice and not a vector, as we won't change the data after
        // initializing it, so we can share the stealers among workers without
        // the need for synchronization.
        let stealers: Arc<[Stealer<Box<dyn Task>>]> =
            Arc::from(workers.iter().map(Worker::stealer).collect::<Vec<_>>());

        // Keep track of running and pending tasks
        let running = Arc::new(AtomicUsize::new(0));
        let pending = Arc::new(AtomicUsize::new(0));

        // Initialize worker threads
        let iter = workers.into_iter().enumerate().map(|(index, worker)| {
            let injector = Arc::clone(&injector);
            let stealers = Arc::clone(&stealers);
            let signal = Arc::clone(&signal);

            // Create worker thread and obtain references to injector and
            // stealers, which we need to retrieve the next task
            let running = Arc::clone(&running);
            let pending = Arc::clone(&pending);
            let h = move || {
                let injector = injector.as_ref();
                let stealers = stealers.as_ref();

                // Try to fetch the next task, either from the local queue, or
                // from the injector or another worker. Additionally, we keep
                // track of the number of running tasks to provide a simple way
                // to monitor the load of the thread pool.
                loop {
                    let Some(task) = get(&worker, injector, stealers) else {
                        // No more tasks, so we wait for the executor to signal
                        // if the worker should continue or terminate. This can
                        // fail due to a poisoned lock, in which case we need
                        // to terminate gracefully as well.
                        if signal.should_terminate()? {
                            break;
                        }

                        // Return to waiting for next task
                        continue;
                    };

                    // Update number of pending and running tasks
                    pending.fetch_sub(1, Ordering::Acquire);
                    running.fetch_add(1, Ordering::Release);

                    // Execute task, but ignore panics, since the executor has
                    // no way of reporting them, and they're printed anyway
                    let subtasks = panic::catch_unwind(|| task.execute())
                        .unwrap_or_default();

                    // Update number of running tasks
                    running.fetch_sub(1, Ordering::Acquire);

                    // In case the task returned further subtasks, we add them
                    // to the local queue, so they are executed by the current
                    // worker, or can be stolen by another worker in case the
                    // current worker thread is busy
                    if !subtasks.is_empty() {
                        let added = subtasks
                            .into_iter()
                            .map(|subtask| worker.push(subtask))
                            .count();

                        // Update number of running and pending tasks, and wake
                        // other workers threads to allow for stealing
                        pending.fetch_add(added, Ordering::Release);
                        signal.notify();
                    }
                }

                // No errors occurred
                Ok(())
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
            injector,
            signal,
            threads,
            running,
            pending,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Strategy for WorkStealing {
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
    /// This method is infallible, and will always return [`Ok`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_executor::strategy::{Strategy, WorkStealing};
    ///
    /// // Create strategy and submit task
    /// let strategy = WorkStealing::default();
    /// strategy.submit(Box::new(|| println!("Task")))?;
    /// # Ok(())
    /// # }
    /// ```
    fn submit(&self, task: Box<dyn Task>) -> Result {
        // As workers can steal tasks from the injector, we must manually track
        // the number of pending tasks. For this reason, we increment the count
        // by one to signal a new task was added, hand the task to the injector,
        // and then wake any waiting worker threads.
        self.injector.push(task);
        self.pending.fetch_add(1, Ordering::Release);
        self.signal.notify();

        // No errors occurred
        Ok(())
    }

    /// Returns the number of workers.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Strategy, WorkStealing};
    ///
    /// // Get number of workers
    /// let strategy = WorkStealing::new(1);
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
    /// use zrx_executor::strategy::{Strategy, WorkStealing};
    ///
    /// // Get number of running tasks
    /// let strategy = WorkStealing::default();
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
    /// use zrx_executor::strategy::{Strategy, WorkStealing};
    ///
    /// // Get number of pending tasks
    /// let strategy = WorkStealing::default();
    /// assert_eq!(strategy.num_tasks_pending(), 0);
    /// ```
    #[inline]
    fn num_tasks_pending(&self) -> usize {
        self.pending.load(Ordering::Relaxed)
    }

    /// Returns the capacity, if bounded.
    ///
    /// The work-stealing execution strategy does not impose a hard limit on
    /// the number of tasks. Thus, this strategy should only be used if tasks
    /// are not produced faster than they can be executed, or the number of
    /// tasks is limited by some other means.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_executor::strategy::{Strategy, WorkStealing};
    ///
    /// // Get capacity
    /// let strategy = WorkStealing::default();
    /// assert_eq!(strategy.capacity(), None);
    /// ```
    #[inline]
    fn capacity(&self) -> Option<usize> {
        None
    }
}

// ----------------------------------------------------------------------------

impl Default for WorkStealing {
    /// Creates a work-stealing execution strategy using all CPUs - 1.
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
    /// use zrx_executor::strategy::WorkStealing;
    ///
    /// // Create strategy
    /// let strategy = WorkStealing::default();
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

impl Drop for WorkStealing {
    /// Terminates and joins all worker threads.
    ///
    /// This method waits for all worker threads to finish executing currently
    /// running tasks, while ignoring any pending tasks. All worker threads are
    /// joined before the method returns. This is necessary to prevent worker
    /// threads from running after the strategy has been dropped.
    fn drop(&mut self) {
        let _ = self.signal.terminate();

        // Join all worker threads without panicking on errors
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}

// ----------------------------------------------------------------------------

impl fmt::Debug for WorkStealing {
    /// Formats the execution strategy for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WorkStealing")
            .field("workers", &self.num_workers())
            .field("running", &self.num_tasks_running())
            .field("pending", &self.num_tasks_pending())
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Attempts to get the next available task, either from the worker's own queue
/// or by stealing from the injector or other stealers if needed. Note that this
/// code was taken almost verbatim from the [`crossbeam`] docs, specifically
/// from [`crossbeam::deque`](crossbeam::deque#examples), but cut smaller.
fn get<T>(
    worker: &Worker<T>, injector: &Injector<T>, stealers: &[Stealer<T>],
) -> Option<T> {
    worker
        .pop()
        .or_else(|| steal_or_retry(worker, injector, stealers))
}

/// Repeatedly attempts to steal a task from the injector or stealers until a
/// non-retryable steal result is found, returning the successful task if any.
fn steal_or_retry<T>(
    worker: &Worker<T>, injector: &Injector<T>, stealers: &[Stealer<T>],
) -> Option<T> {
    repeat_with(|| steal(worker, injector, stealers))
        .find(|steal| !steal.is_retry())
        .and_then(Steal::success)
}

/// Tries to steal a task from the injector or, if unavailable, from each
/// stealer in sequence, collecting any valid tasks or signaling retry.
fn steal<T>(
    worker: &Worker<T>, injector: &Injector<T>, stealers: &[Stealer<T>],
) -> Steal<T> {
    injector
        .steal_batch_and_pop(worker)
        .or_else(|| stealers.iter().map(Stealer::steal).collect())
}
