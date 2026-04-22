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

use crossbeam::channel::Select;
use std::fmt::Debug;
use std::time::Instant;

use zrx_executor::strategy::WorkSharing;
use zrx_executor::{Executor, Strategy};

pub mod action;
mod engine;
pub mod router;
pub mod schedule;
pub mod session;
pub mod signal;
pub mod step;

use engine::queue::{Tasks, Timers};
use engine::{Actions, AsReceiver, Token, TokenFull};
use router::Router;
use schedule::Schedule;
use session::Session;
use signal::{Id, Value};
use step::effect::timer::{IntoDuration, IntoInstant};
use step::effect::Effect;
use step::{Step, Steps};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scheduler.
#[derive(Debug)]
pub struct Scheduler<I, S = WorkSharing>
where
    S: Strategy,
{
    /// Router.
    router: Router<I>,
    /// Action queue.
    actions: Actions<I>,
    /// Timer queue.
    timers: Timers<I>,
    /// Task queue.
    tasks: Tasks<I, S>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, S> Scheduler<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Creates a scheduler.
    #[must_use]
    pub fn new(executor: Executor<S>) -> Self {
        Self {
            router: Router::default(),
            actions: Actions::new(),
            timers: Timers::new(),
            tasks: Tasks::new(executor),
        }
    }

    /// Attaches a schedule to the scheduler.
    #[inline]
    pub fn attach<W>(&mut self, schedule: W) -> usize
    where
        W: Into<Schedule<I>>,
    {
        let s = self.actions.attach(schedule.into());

        // Obtain workflow and register all sources in the router
        let schedule = &mut self.actions[s];
        let graph = &mut schedule.graph;
        for n in graph.sources().collect::<Vec<_>>() {
            if let Some(source) = graph[n].as_source_mut() {
                self.router
                    .add(Token { module: s, node: n }, source.sender());
            }
        }

        // Return index
        s
    }

    /// Detaches a schedule from the scheduler.
    #[inline]
    pub fn detach(&mut self, index: usize) -> Option<Schedule<I>> {
        self.actions.detach(index)
    }

    /// Creates a session.
    #[inline]
    #[must_use]
    pub fn session<T>(&mut self) -> Session<I, T>
    where
        T: Value,
    {
        self.router.session()
    }

    /// Processes the next tick.
    #[inline]
    pub fn tick(&mut self) {
        self.process(None);
    }

    /// Processes the next tick, waiting until the given deadline.
    #[inline]
    pub fn tick_deadline<T>(&mut self, deadline: T)
    where
        T: IntoInstant,
    {
        self.process(Some(deadline.into_instant()));
    }

    /// Processes the next tick, waiting for the given timeout.
    #[inline]
    pub fn tick_timeout<T>(&mut self, timeout: T)
    where
        T: IntoDuration,
    {
        self.process(Some(timeout.into_duration().into_instant()));
    }

    /// Processes the next tick.
    ///
    /// [`Timer`][] and [`Task`][] effects are drained first, as they represent
    /// work that is already complete or due. Handling them before pulling new
    /// items from sessions feeds their effects back into the underlying graph
    /// immediately, potentially unblocking further frontiers and giving the
    /// executor new work. Pulling from sessions first would load new work onto
    /// a system that may already have actionable results waiting.
    ///
    /// Timers in particular are time-sensitive — they are already late by the
    /// time this runs, so any delay adds latency to time-critical paths.
    ///
    /// [`Task`]: crate::scheduler::step::effect::Task
    /// [`Timer`]: crate::scheduler::step::effect::Timer
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn process(&mut self, deadline: Option<Instant>) {
        self.process_timers();
        self.process_tasks();

        // Check, if there's at least one action that can be scheduled, and if
        // so, run it. Otherwise, if the queue is empty, enter waiting phase.
        if self.actions.is_empty() {
            self.waiting(deadline.or(self.is_empty().then(Instant::now)));
        } else {
            self.running();
        }
    }

    /// Processes timers.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn process_timers(&mut self) {
        while let Some((token, steps)) = self.timers.take() {
            self.handle(
                Token {
                    module: token.module,
                    node: token.node,
                },
                steps,
            );
        }
    }

    /// Processes tasks.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn process_tasks(&mut self) {
        while let Some((token, res)) = self.tasks.take() {
            self.handle(
                Token {
                    module: token.module,
                    node: token.node,
                },
                res.unwrap(),
            );
        }

        // Queue next tasks - this method returns whether any new tasks could
        // be submitted, and if there were some in the queue.
        self.tasks.update();
    }

    /// Waiting phase - the scheduler can't make progress on any of the active
    /// frontiers, either because all frontiers have been processed, or because
    /// the executor is waiting for tasks or timers to be completed. In case a
    /// deadline was provided, and the sessions don't have any more items to
    /// emit, the scheduler will wait until the deadline is reached. Otherwise,
    /// the scheduler returns immediately.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn waiting(&mut self, deadline: Option<Instant>) {
        let mut select = Select::new();
        assert_eq!(0, select.recv(self.timers.as_receiver()));
        assert_eq!(1, select.recv(self.tasks.as_receiver()));
        self.router.add_to_select(&mut select);

        // Poll until deadline is reached, if given
        let n = if let Some(deadline) = deadline {
            match select.ready_deadline(deadline) {
                Ok(ready) => ready,
                Err(_) => return,
            }
        } else {
            select.ready()
        };

        // Poll all channels
        match n {
            0 => self.process_timers(),
            1 => self.process_tasks(),
            n => {
                for token in self.router.poll(n - 2) {
                    self.actions.submit(token);
                }
            }
        }
    }

    /// Running phase
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all)
    )]
    fn running(&mut self) {
        while let Some((token, steps)) = self.actions.take() {
            for res in steps {
                self.handle(token, res.unwrap());
            }

            // In case timers or tasks became ready in the meantime, we break
            // the loop to process them first, as they may unblock frontiers
            if self.tasks.is_ready() || self.timers.is_ready() {
                break;
            }
        }
    }

    /// Handle the given steps for the given token.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "trace", skip_all)
    )]
    fn handle(&mut self, token: Token, steps: Steps<I>) {
        for step in steps {
            let Step { mut scope, effect } = step;

            // Ensure the scope has a frontier attached
            let scope = self.actions.ensure(token, &mut scope);
            match effect {
                Effect::Then(then) => {
                    let (token, inner) = self.actions.resume(token, then);
                    for res in inner {
                        self.handle(token, res.unwrap());
                    }
                }
                Effect::Timer(timer) => {
                    self.timers.submit(
                        TokenFull {
                            module: token.module,
                            node: token.node,
                            frontier: scope.id().expect("invariant"),
                        },
                        timer,
                    );
                }
                Effect::Task(task) => {
                    self.tasks.submit(
                        TokenFull {
                            module: token.module,
                            node: token.node,
                            frontier: scope.id().expect("invariant"),
                        },
                        task,
                    );
                }
                Effect::Done => {
                    self.actions.complete(token, &scope);
                }
            }
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, S> Scheduler<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Returns the number of work to be done.
    #[inline]
    pub fn len(&self) -> usize {
        self.router.len()
            + self.actions.len()
            + self.timers.len()
            + self.tasks.len()
    }

    /// Returns whether there is any work to be done.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.router.is_empty()
            && self.actions.is_empty()
            && self.timers.is_empty()
            && self.tasks.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, S> Default for Scheduler<I, S>
where
    I: Id,
    S: Strategy + Default,
{
    /// Creates a scheduler using the default work-sharing strategy.
    #[inline]
    fn default() -> Self {
        Self::new(Executor::new(S::default()))
    }
}
