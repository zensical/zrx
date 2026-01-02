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

//! Scheduler tick.

use crossbeam::channel::{at, never};
use crossbeam::select;
use std::error::Error;
use std::marker::PhantomData;
use std::time::Instant;

use zrx_diagnostic::report::Report;
use zrx_executor::Strategy;

use super::action::Outputs;
use super::executor::ToReceiver;
use super::{Id, Scheduler};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scheduler tick.
#[derive(Debug)]
pub struct Tick<I, S> {
    /// Report.
    report: Report,
    /// Waiting deadline.
    deadline: Option<Instant>,
    /// Type marker.
    marker: PhantomData<(I, S)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, S> Tick<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Creates a scheduler tick.
    pub fn new(deadline: Option<Instant>) -> Self {
        Self {
            report: Report::new(()),
            deadline,
            marker: PhantomData,
        }
    }

    /// Runs the tick, processing all tasks and timers in the scheduler.
    #[inline]
    pub fn run(mut self, scheduler: &mut Scheduler<I, S>) -> Report {
        self.process(scheduler);
        self.report
    }

    /// Processes the scheduler tick.
    fn process(&mut self, scheduler: &mut Scheduler<I, S>) {
        self.process_tasks(scheduler);
        self.process_timers(scheduler);

        if scheduler.executor.can_make_progress() {
            self.running(scheduler);
        } else {
            self.waiting(scheduler);
        }
    }

    /// Processes tasks.
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    fn process_tasks(&mut self, scheduler: &mut Scheduler<I, S>) {
        while let Some((token, res)) = scheduler.tasks.take() {
            match res {
                Err(err) => {
                    handle_error(&err);
                    scheduler.handle(token, Outputs::default());
                }
                Ok(target) => {
                    scheduler.handle(token, self.report.merge(target));
                }
            }
        }

        // Queue next tasks - this method returns whether any new tasks could
        // be submitted, and if there were some in the queue.
        scheduler.tasks.update();
    }

    /// Processes timers.
    #[allow(clippy::unused_self)]
    #[cfg_attr(feature = "tracing", tracing::instrument(skip_all))]
    fn process_timers(&mut self, scheduler: &mut Scheduler<I, S>) {
        while let Some((token, outputs)) = scheduler.timers.take() {
            scheduler.handle(token, outputs);
        }
    }

    /// Running phase.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn running(&mut self, scheduler: &mut Scheduler<I, S>) {
        let mut max = 0;

        // Process a maximum of 16 messages per tick
        while let Some(message) = scheduler.connector.take() {
            scheduler.handle_message(message);
            scheduler.total += 1;
            max += 1;
            if max >= 16 {
                break;
            }
        }

        // Process a maximum of 16 results per tick
        let mut max = 0;
        for (token, res) in scheduler.executor.take() {
            match res {
                Err(err) => {
                    handle_error(&err);
                    scheduler.handle(token, Outputs::default());
                }
                Ok(target) => {
                    scheduler.handle(token, self.report.merge(target));
                }
            }
            max += 1;
            if max >= 16 {
                break;
            }
        }
    }

    /// Waiting phase - the scheduler can't make progress on any of the active
    /// frontiers, either because all frontiers have been processed, or because
    /// the executor is waiting for tasks or timers to be completed. In case a
    /// deadline was provided, and the session connector doesn't have any more
    /// items to emit, the scheduler will wait until the deadline is reached.
    /// Otherwise, the scheduler returns immediately.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn waiting(&mut self, scheduler: &mut Scheduler<I, S>) {
        // Before entering the waiting phase, we only checked if the executor
        // has any frontiers to be processed in its queue, as this is sufficient
        // to discern whether we need to enter the running or waiting phase. If
        // no deadline was provided, we also need to check if the connector has
        // any items waiting to be processed, or tasks and timers are waiting
        // to be completed, as we need to be able to wake up the scheduler.
        let deadline = self.deadline(scheduler);
        select! {
            recv(deadline.map_or_else(never, at)) -> _ => {}

            // When the connector emits an item, process it immediately - note
            // that the connector can never be disconnected, since the sender
            // and receiver are both owned by it, so the channel is only closed
            // when the scheduler terminates and the connector is dropped.
            recv(scheduler.connector.to_receiver()) -> res => {
                let message = res.expect("invariant");
                scheduler.handle_message(message);
            }

            // When the task engine emits, handle the outputs returned - as with
            // the connector, the task engine owns the sender and the receiver
            recv(scheduler.tasks.to_receiver()) -> res => {
                let (token, res) = res.expect("invariant");

                // For the inner result, it's a different story, since it might
                // contain an error that is the result of executing an action,
                // This includes the case when the action panics, which is
                // caught and converted to an error.
                match res {
                    Err(err) => {
                        handle_error(&err);
                        scheduler.handle(token, Outputs::default());
                    }
                    Ok(target) => {
                        scheduler.handle(token, self.report.merge(target));
                    }
                }
            }

            // When the timer engine emits, process all timers that are due, as
            // we don't just get the next timer, but only a notification
            recv(scheduler.timers.to_receiver()) -> _ => {
                self.process_timers(scheduler);
            }
        };
    }

    /// Determines the deadline
    fn deadline(&mut self, scheduler: &mut Scheduler<I, S>) -> Option<Instant> {
        self.deadline.or((scheduler.connector.is_empty()
            && scheduler.tasks.is_empty()
            && scheduler.timers.is_empty())
        .then(Instant::now))
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Handles and prints an error chain.
fn handle_error(err: &dyn Error) {
    let mut current = Some(err as &dyn std::error::Error);
    let mut indent = 0;

    // Handle error chain
    while let Some(error) = current {
        let prefix = if indent == 0 {
            "Error: "
        } else {
            "Caused by: "
        };

        // Print error with indentation
        println!("{:indent$}{}{}", "", prefix, error, indent = indent);
        current = error.source();
        indent += 2;
    }
}
