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

//! Scheduler.

use std::time::{Duration, Instant};

use zrx_diagnostic::report::Report;
use zrx_executor::strategy::WorkSharing;
use zrx_executor::{self, Strategy};

pub mod action;
pub mod effect;
mod executor;
pub mod graph;
pub mod id;
pub mod session;
mod tick;
pub mod value;

use action::{Output, Outputs};
use executor::queue::{Tasks, Timers};
use executor::{Executor, Token};
use graph::Graph;
use id::Id;
use session::{Connector, Message, Result, Session, Sessions};
use tick::Tick;
use value::Value;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scheduler.
#[derive(Debug)]
pub struct Scheduler<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Executor.
    executor: Executor<I>,
    /// Session connector.
    connector: Connector<I>,
    /// Session manager.
    sessions: Sessions,
    /// Task queue.
    tasks: Tasks<I, S>,
    /// Timer queue.
    timers: Timers<I>,
    /// Total items processed.
    total: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Scheduler<I, WorkSharing>
where
    I: Id,
{
    /// Creates a scheduler.
    ///
    /// This method creates a scheduler with an [`Executor`] which utilizes a
    /// [`WorkSharing`] strategy, probably the best choice for most workloads.
    /// The number of workers is determined by the number of logical CPUs minus
    /// one, which reserves one core for the main thread for orchestration.
    #[must_use]
    pub fn new(meta: Graph<I>) -> Self {
        Self::with_executor(meta, zrx_executor::Executor::default())
    }
}

impl<I, S> Scheduler<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Creates a scheduler with the given executor.
    ///
    /// For most workloads, the [`WorkSharing`] scheduling strategy should be
    /// the best choice, as it offers the lowest possible overhead and applies
    /// backpressure by rejecting tasks after the system has reached a certain
    /// load. Thus, start with [`Scheduler::new`] before fine-tuning.
    ///
    /// For more information, see [`WorkSharing::with_capacity`].
    #[must_use]
    pub fn with_executor(
        meta: Graph<I>, executor: zrx_executor::Executor<S>,
    ) -> Self {
        Self {
            executor: Executor::new(meta.actions),
            connector: Connector::new(),
            sessions: Sessions::new(meta.sources),
            tasks: Tasks::new(executor),
            timers: Timers::new(),
            total: 0,
        }
    }

    /// Creates a session.
    ///
    /// __Warning__: Sessions are generally not meant for use on the same thread
    /// as the scheduler. In case the capacity of the [`Connector`] is reached,
    /// a session might block, and thus deadlock the scheduler. Thus, always
    /// move sessions to a separate thread.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Type`][] if the type is unknown.
    ///
    /// [`Error::Type`]: crate::scheduler::session::Error::Type
    #[inline]
    pub fn session<T>(&mut self) -> Result<Session<I, T>>
    where
        T: Value,
    {
        // this API is still not ideal, feels kinda weird
        let session = self.connector.session();
        self.sessions.insert::<T>(session.id()).map(|()| session)
    }

    /// Runs a tick.
    ///
    /// This method processes all [`Tasks`] and [`Timers`] in the scheduler, and
    /// returns a report containing the results of the tick. Note that it never
    /// blocks, which means that it will return immediately after all tasks
    /// and timers have been processed.
    #[inline]
    pub fn tick(&mut self) -> Report {
        Tick::new(None).run(self)
    }

    /// Runs a tick or waits until the given deadline.
    ///
    /// After processing completed tasks and timers, the scheduler waits until
    /// the given deadline if it can't make progress on its own. This means that
    /// this method might block, depending on the state of the queue and whether
    /// there's any new input to be processed.
    #[inline]
    pub fn tick_deadline(&mut self, deadline: Instant) -> Report {
        Tick::new(Some(deadline)).run(self)
    }

    /// Runs a tick or waits until the given timeout.
    ///
    /// After processing completed tasks and timers, the scheduler waits until
    /// the given timeout if it can't make progress on its own. This means that
    /// this method might block, depending on the state of the queue and whether
    /// there's any new input to be processed.
    #[inline]
    pub fn tick_timeout(&mut self, timeout: Duration) -> Report {
        Tick::new(Some(Instant::now() + timeout)).run(self)
    }

    /// Handles the given message.
    ///
    /// This method processes the given message received from a session, either
    /// forwarding a new item to the executor or terminating the session.
    fn handle_message(&mut self, message: Message<I>) {
        match message {
            Message::Item(id, item) => {
                self.executor.submit(item, self.sessions.get(id));
            }
            Message::Drop(id) => {
                self.sessions.remove(id);
            }
        }
    }

    /// Handles the given output.
    ///
    /// This method processes the output of a task, timer, or other effect, and
    /// updates the executor or respective queue in the process.
    fn handle(&mut self, token: Token, outputs: Outputs<I>) {
        let mut items = Vec::new();
        let has_outputs = !outputs.is_empty();
        // println!("?? handle outputs: {:#?}", outputs);
        for output in outputs {
            match output {
                Output::Item(item) => items.push(item),
                Output::Task(task) => self.tasks.submit(token, task),
                Output::Timer(timer) => self.timers.submit(token, timer),
            }
        }

        // there might be only tasks or timers, but no items - we need a better
        // solution for that - note that nodes can only be completed once
        if !has_outputs || !items.is_empty() {
            self.executor.update(token, items);
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, S> Scheduler<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Returns the number of unprocessed items.
    #[inline]
    pub fn len(&self) -> usize {
        self.executor.len()
    }

    /// Returns whether there are any unprocessed items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.executor.is_empty() && self.connector.is_empty()
    }

    /// Returns the total number of items processed since creation.
    #[inline]
    pub fn total(&self) -> usize {
        self.total
    }
}
