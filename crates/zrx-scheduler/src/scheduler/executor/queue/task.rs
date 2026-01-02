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

//! Task queue.

use crossbeam::channel::{bounded, Receiver, Sender};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::panic::AssertUnwindSafe;

use zrx_executor::strategy::WorkSharing;
use zrx_executor::{self as executor, Error, Executor, Strategy};

use crate::scheduler::action::{Outputs, Result};
use crate::scheduler::effect::Task;

use super::{ToReceiver, Token};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Task queue.
///
/// This data type submits [`Task`] instances to an [`Executor`], and collects
/// their outputs as they complete. Each task is identified by a [`Token`] in
/// order to uniquely associate it to a node within a frontier. In case the
/// executor is at capacity, tasks are queued for later submission.
#[derive(Debug)]
pub struct Tasks<I, S>
where
    S: Strategy,
{
    /// Executor.
    executor: Executor<S>,
    /// Queue of tasks.
    queue: VecDeque<Box<dyn executor::Task>>,
    /// Job submission sender.
    sender: Sender<Job<I>>,
    /// Job completion receiver.
    receiver: Receiver<Job<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, S> Tasks<I, S>
where
    S: Strategy,
{
    /// Creates a task queue.
    ///
    /// In case the execution strategy defines a capacity limit, this capacity
    /// is also used for the task queue in order to apply backpressure to task
    /// submission. Otherwise, a capacity of 8 tasks per worker is used, so
    /// for 4 workers, the task queue will have a capacity of 32 tasks.
    ///
    /// If the executor is at capacity, tasks are queued and submitted to the
    /// executor once the capacity is available again.
    #[must_use]
    pub fn new(executor: Executor<S>) -> Self {
        let capacity =
            executor.capacity().unwrap_or(8 * executor.num_workers());
        Self::with_capacity(executor, capacity)
    }

    /// Creates a task queue with the given capacity.
    ///
    /// The given capacity sets the number of tasks the task queue can accept
    /// before starting to reject them, which is used to apply backpressure.
    /// Note that the capacity is not a per-worker, but a global limit, so
    /// set it accordingly.
    #[must_use]
    pub fn with_capacity(executor: Executor<S>, capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self {
            executor,
            queue: VecDeque::new(),
            sender,
            receiver,
        }
    }

    /// Submits a task.
    ///
    /// Note that this method will not signal to the user whether the task was
    /// submitted or needed to be queued. This is because the task queue, as the
    /// other queues, always accepts tasks. It's the scheduler's responsibility
    /// to select the optimal action at the right time, maximizing throughput.
    pub fn submit(&mut self, token: Token, task: Task<I>)
    where
        I: Send + 'static,
    {
        match self.executor.submit({
            // We maintain a single receiver, that collects the results of tasks
            // which are executed by the executor. In case the executor defines
            // a capacity, we're using a bounded channel. Otherwise, the channel
            // is unbounded. In both cases, the channel can theoretically become
            // disconnected, but this can practically never happen, because the
            // executor will catch panics in user-provided tasks. If the channel
            // is bounded, the send operation could theoretically fail, but as
            // the sender can only carry as many tasks as can be submitted, this
            // can never happen. Thus, it's safe to ignore the result.
            let sender = self.sender.clone();
            AssertUnwindSafe(move || {
                let _ = sender.send((token, task.execute()));
            })
        }) {
            // Task submission succeeded
            Ok(()) => (),

            // Task submission failed, as the executor has a capacity limit and
            // is currently at capacity, so the task is queued for later retry
            Err(Error::Submit(task)) => {
                self.queue.push_back(task);
            }

            // Task executor encountered an unrecoverable error when trying to
            // synchronize its worker threads, which should never happen, since
            // the executor is designed with resilience in mind. Thus, if we run
            // into this error, it denotes a bug in our implementation.
            Err(Error::Signal) => panic!("invariant"),
        }
    }

    /// Updates the task queue.
    ///
    /// This method is called at a specific time during scheduling, and tries to
    /// submit as many previously queued tasks as possible to the executor that
    /// was at capacity before. The number of tasks that could be submitted is
    /// returned, or [`None`], if the queue was empty.
    pub fn update(&mut self) -> Option<usize> {
        (!self.queue.is_empty()).then(|| {
            let mut count = 0;
            while let Some(task) = self.queue.pop_front() {
                // If task submission fails, we push it back to the queue and
                // stop task submission, so that we can retry it later
                if let Err(Error::Submit(task)) = self.executor.submit(task) {
                    self.queue.push_front(task);
                    break;
                }
                count += 1;
            }
            count
        })
    }

    /// Returns the next completed task.
    #[inline]
    pub fn take(&self) -> Option<Job<I>> {
        self.receiver.try_recv().ok()
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, S> Tasks<I, S>
where
    S: Strategy,
{
    // /// Returns the number of tasks.
    // #[inline]
    // pub fn len(&self) -> usize {
    //     self.executor.len() + self.queue.len() + self.receiver.len()
    // }

    /// Returns whether there are any tasks.
    ///
    /// This method returns `true` if the following criteria are met:
    ///
    /// 1. The executor has no more running or pending tasks
    /// 2. The receiver has no more results to be collected
    /// 3. The queue has no more pending task submissions
    #[allow(clippy::needless_return)]
    #[inline]
    pub fn is_empty(&self) -> bool {
        return self.executor.is_empty()
            && self.receiver.is_empty()
            && self.queue.is_empty();
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, S> ToReceiver<I> for Tasks<I, S>
where
    S: Strategy,
{
    type Item = Job<I>;

    /// Returns the receiver of the task queue.
    #[inline]
    fn to_receiver(&self) -> Cow<'_, Receiver<Self::Item>> {
        Cow::Borrowed(&self.receiver)
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Tasks<I, WorkSharing> {
    /// Creates a task queue using the default work-sharing strategy.
    #[inline]
    fn default() -> Self {
        Self::new(Executor::default())
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Task queue job.
pub type Job<I> = (Token, Result<Outputs<I>>);
