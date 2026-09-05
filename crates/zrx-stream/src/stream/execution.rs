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

//! Stream execution.

use crossbeam::channel::Select;
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error as ThisError;

use zrx_executor::Strategy;
use zrx_executor::strategy::WorkSharing;
use zrx_scheduler::action::Port;
use zrx_scheduler::plan::{InputId, OutputId, PlanError};
use zrx_scheduler::{
    CurrentError, Egress, PlanId, Report, Scheduler, SessionError, Value,
};

use super::workflow::{Direction, Input as InputPort, LookupError};
use super::{Id, Key, Workflow};

mod input;
mod lazy;
mod output;

use input::Progress;
pub use input::{Input, Revision};
pub use lazy::{Execution, Scope, run};
pub use output::{Output, Run};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid workflow execution operation.
#[derive(Debug, ThisError)]
pub enum Error {
    /// The closed workflow could not be lowered to a scheduler plan.
    #[error(transparent)]
    Plan(#[from] PlanError),
    /// A scheduler boundary operation failed.
    #[error(transparent)]
    Scheduler(#[from] zrx_scheduler::Error),
    /// A session operation failed after acquisition.
    #[error(transparent)]
    Session(#[from] SessionError),
    /// A unique typed endpoint query failed.
    #[error(transparent)]
    Lookup(#[from] LookupError),
    /// The selected input already has its authoritative session.
    #[error("workflow input {0:?} already has an active session")]
    Claimed(InputId),
    /// The selected output was already taken from this run.
    #[error("workflow output {0:?} was already taken")]
    Taken(OutputId),
    /// One or more input revisions remain open.
    #[error("cannot settle while {0} input revisions remain open")]
    Open(usize),
    /// A sealed revision retained authority without future readiness.
    #[error("workflow cannot make progress toward settlement")]
    Stalled,
}

// ----------------------------------------------------------------------------

/// One observable step of reusable workflow execution.
#[must_use]
pub enum Advance<I>
where
    I: Id,
{
    /// One owned output batch is ready for immediate consumption.
    Output(Egress<Key<I>>),
    /// Scheduler progress produced diagnostics or settlements.
    Progress(Report),
    /// Every closed revision and its derived work have settled.
    Settled,
    /// No work is currently ready; open revisions may admit more input.
    Idle,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Reusable execution owner for one closed workflow.
pub struct Runner<I, S = WorkSharing>
where
    I: Id,
    S: Strategy,
{
    scheduler: Scheduler<Key<I>, S>,
    plan: PlanId,
    inputs: Vec<InputPort>,
    outputs: Vec<super::workflow::Output>,
    claimed: BTreeSet<InputId>,
    progress: Arc<Progress>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Runner<I>
where
    I: Id,
{
    /// Lowers a workflow onto the default work-sharing executor.
    ///
    /// # Errors
    ///
    /// Returns an error if the closed workflow is not a valid scheduler plan.
    pub fn new(workflow: Workflow<I>) -> Result<Self, Error> {
        Self::with_strategy(workflow, WorkSharing::default())
    }
}

impl<I, S> Runner<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Lowers a workflow onto an explicit execution strategy.
    ///
    /// # Errors
    ///
    /// Returns an error if the closed workflow is not a valid scheduler plan.
    pub fn with_strategy(
        workflow: Workflow<I>, strategy: S,
    ) -> Result<Self, Error> {
        let (plan, inputs, outputs) = workflow.lower()?;
        let mut scheduler = Scheduler::new(strategy);
        let plan = scheduler.attach(plan);
        Ok(Self {
            scheduler,
            plan,
            inputs,
            outputs,
            claimed: BTreeSet::new(),
            progress: Arc::new(Progress::default()),
        })
    }

    /// Acquires the sole input carrying `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when no unique matching endpoint exists or its
    /// authoritative session was already acquired.
    pub fn input<T>(&mut self) -> Result<Input<I, T>, Error>
    where
        T: Value,
    {
        let port = Port::of::<Key<I>, T>();
        let mut inputs =
            self.inputs.iter().filter(|input| input.port() == port);
        let Some(input) = inputs.next().copied() else {
            return Err(LookupError::Missing {
                direction: Direction::Input,
                value: std::any::type_name::<T>(),
            }
            .into());
        };
        if inputs.next().is_some() {
            return Err(LookupError::Ambiguous {
                direction: Direction::Input,
                value: std::any::type_name::<T>(),
            }
            .into());
        }
        self.input_at(input)
    }

    /// Acquires one exact erased input through its statically known type.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign or mismatched endpoint, a duplicate
    /// acquisition, or scheduler session failure.
    pub fn input_at<T>(
        &mut self, input: InputPort,
    ) -> Result<Input<I, T>, Error>
    where
        T: Value,
    {
        let expected = Port::of::<Key<I>, T>();
        if input.port() != expected || !self.inputs.contains(&input) {
            return Err(LookupError::Missing {
                direction: Direction::Input,
                value: std::any::type_name::<T>(),
            }
            .into());
        }
        if self.claimed.contains(&input.id()) {
            return Err(Error::Claimed(input.id()));
        }
        let session = self
            .scheduler
            .attachment(self.plan)?
            .session::<T>(input.id())?;
        self.claimed.insert(input.id());
        Ok(Input::new(session, Arc::clone(&self.progress)))
    }

    /// Returns the current errors owned by this workflow execution.
    ///
    /// # Panics
    ///
    /// Panics if the runner's private scheduler plan was detached, which is
    /// not exposed by the runner API.
    #[must_use]
    pub fn errors(&self) -> &[CurrentError<Key<I>>] {
        self.scheduler
            .errors(self.plan)
            .expect("runner plan remains attached")
    }

    /// Drives every sealed revision to settlement and returns lazy outputs.
    ///
    /// # Errors
    ///
    /// Returns an error while a revision remains open (including while its
    /// terminal event is being published), if scheduler access fails, or if
    /// retained authority has no future readiness.
    pub fn settle(&mut self) -> Result<Run<I>, Error> {
        let outputs = self.outputs.clone();
        let mut egress = VecDeque::new();
        let report = self.settle_with(|batch| egress.push_back(batch))?;
        Ok(Run::new(outputs, egress, report))
    }

    /// Drives every sealed revision to settlement and visits each owned output
    /// batch as soon as it becomes available.
    ///
    /// This is the bounded multi-output counterpart to [`Self::settle`]. The
    /// runner retains no egress after the visitor returns; callers control any
    /// collection needed by their output protocol.
    ///
    /// # Errors
    ///
    /// Returns an error while a revision remains open (including while its
    /// terminal event is being published), if scheduler access fails, or if
    /// retained authority has no future readiness.
    pub fn settle_with(
        &mut self, mut visit: impl FnMut(Egress<Key<I>>),
    ) -> Result<Report, Error> {
        let open = self.progress.open();
        if open != 0 {
            return Err(Error::Open(open));
        }

        let mut report = Report::default();
        loop {
            match self.advance()? {
                Advance::Output(batch) => visit(batch),
                Advance::Progress(next) => report.append(next),
                Advance::Settled => return Ok(report),
                Advance::Idle => return Err(Error::Stalled),
            }
        }
    }

    /// Advances execution until one output, report, settlement boundary, or
    /// open-revision idle point becomes observable.
    ///
    /// Unlike [`Self::settle`], this operation is valid while revisions are
    /// open and transfers each output batch directly to the caller. Repeatedly
    /// interleave it with [`Revision::emit_from`] to keep admission and egress
    /// bounded for large revisions or workflows with multiple outputs.
    ///
    /// # Errors
    ///
    /// Returns an error if scheduler access fails.
    pub fn advance(&mut self) -> Result<Advance<I>, Error> {
        let mut attachment = self.scheduler.attachment(self.plan)?;
        if let Some(batch) = attachment.egress() {
            return Ok(Advance::Output(batch));
        }
        drop(attachment);

        if let Some(tick) = self.scheduler.tick() {
            let report = tick.into_report();
            let settled = report.settlements().len();
            if settled != 0 {
                self.progress.settled(settled);
            }
            return Ok(Advance::Progress(report));
        }
        if self.progress.open() == 0 && self.progress.pending() == 0 {
            return Ok(Advance::Settled);
        }
        if self.wait() {
            return Ok(Advance::Progress(Report::default()));
        }
        Ok(Advance::Idle)
    }

    fn wait(&self) -> bool {
        let readiness = self.scheduler.readiness();
        if !readiness.pending() && readiness.deadline().is_none() {
            return false;
        }

        // Both worker and timer waits must observe session events and sender
        // teardown. Registration authenticates readiness; hints cannot replace
        // channel selection when deciding to block.
        let mut select = Select::new();
        let readiness = self.scheduler.register(&mut select);
        if let Some(deadline) = readiness.deadline() {
            let timeout = deadline.saturating_duration_since(Instant::now());
            if let Ok(operation) = select.ready_timeout(timeout) {
                assert!(readiness.contains(operation));
            }
        } else {
            let operation = select.ready();
            assert!(readiness.contains(operation));
        }
        true
    }
}

// ----------------------------------------------------------------------------

impl<I> Workflow<I>
where
    I: Id,
{
    /// Lowers this workflow onto the default work-sharing executor.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow is not a valid scheduler plan.
    pub fn runner(self) -> Result<Runner<I>, Error> {
        Runner::new(self)
    }

    /// Lowers this workflow onto an explicit execution strategy.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow is not a valid scheduler plan.
    pub fn runner_with<S>(self, strategy: S) -> Result<Runner<I, S>, Error>
    where
        S: Strategy,
    {
        Runner::with_strategy(self, strategy)
    }
}

#[cfg(test)]
mod tests {
    use zrx_executor::strategy::Immediate;
    use zrx_scheduler::Settlement;

    use crate::stream::Workflow;
    use crate::stream::{Change, Key};

    use super::Error;

    struct FutureWake(std::time::Instant);

    impl zrx_scheduler::action::Action<Key<u64>> for FutureWake {
        type Inputs = (u64,);
        type Output = u64;

        fn execute(
            &mut self,
            context: zrx_scheduler::action::Context<'_, Key<u64>, Self>,
        ) {
            use zrx_scheduler::action::{Wake, WakeKey};
            let zrx_scheduler::action::Context {
                inputs, output, events, ..
            } = context;
            inputs.for_each(output, |_, emit| {
                emit.wake(Wake::at(WakeKey::new(1), self.0));
                Ok(())
            });
            events.for_each(output, |_, _| Ok(()));
        }
    }

    #[test]
    fn wake_only_wait_must_observe_ready_session_events_before_the_deadline() {
        use crate::stream::operator::Operator;
        use crossbeam::channel::Select;
        use std::time::{Duration, Instant};

        let mut delayed = Vec::new();
        for arrival in ["data", "abort", "writer drop", "session disconnect"] {
            let deadline = Instant::now() + Duration::from_secs(1);
            let workflow = Workflow::<u64>::build(|workflow| {
                let input = workflow.input::<u64>();
                let waking = input.subscribe(FutureWake(deadline));
                workflow.output(&waking);
                let quiet = workflow.input::<String>();
                workflow.output(&quiet);
            });
            let mut runner = workflow.runner_with(Immediate::new()).unwrap();
            let input = runner.input::<u64>().unwrap();
            let quiet = runner.input::<String>().unwrap();
            let mut revision = input.begin().unwrap();
            revision
                .emit_from(&mut std::iter::once(Change::Insert(
                    Key::from(1),
                    1,
                )))
                .unwrap();
            // Drive only ready work; Runner::advance would itself enter wait.
            while runner.scheduler.tick().is_some() {}
            let readiness = runner.scheduler.readiness();
            assert!(!readiness.pending(), "must isolate the wake-only branch");
            assert_eq!(readiness.deadline(), Some(deadline));

            let mut revision = Some(revision);
            let mut quiet = Some(quiet);
            let mut returned_input = None;
            match arrival {
                "data" => {
                    revision
                        .as_mut()
                        .unwrap()
                        .emit_from(&mut std::iter::once(Change::Insert(
                            Key::from(2),
                            2,
                        )))
                        .unwrap();
                }
                "abort" => {
                    returned_input =
                        Some(revision.take().unwrap().abort().unwrap());
                }
                "writer drop" => drop(revision.take()),
                "session disconnect" => drop(quiet.take()),
                _ => unreachable!(),
            }
            // Authenticate channel readiness before calling wait. This is
            // stronger than hoping a sender wins a race against a sleep.
            {
                let mut select = Select::new();
                let readiness = runner.scheduler.register(&mut select);
                let operation = select
                    .ready_timeout(Duration::ZERO)
                    .expect("published event or disconnection is observable");
                assert!(readiness.contains(operation));
            }
            assert!(
                Instant::now() < deadline,
                "test setup exhausted the wake interval"
            );
            assert!(runner.wait());
            if Instant::now() >= deadline {
                delayed.push(arrival);
            }
            drop((revision, quiet, returned_input));
        }
        assert!(
            delayed.is_empty(),
            "ready session events waited until the wake deadline: {delayed:?}"
        );
    }

    #[test]
    fn settle_rejects_an_open_revision_and_drains_its_implicit_abort() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<u64>();
            workflow.output(&input);
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let input = runner.input::<u64>().unwrap();
        let revision = input.begin().unwrap();

        assert!(matches!(runner.settle(), Err(Error::Open(1))));
        drop(revision);

        let run = runner.settle().unwrap();
        assert!(matches!(
            run.report().settlements(),
            [Settlement::Aborted(_)]
        ));
    }

    #[test]
    fn overlapping_input_revisions_settle_in_one_cycle() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let numbers = workflow.input::<u64>();
            let strings = workflow.input::<String>();
            workflow.output(&numbers);
            workflow.output(&strings);
        });
        let inputs: Vec<_> = workflow.inputs().copied().collect();
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let numbers = runner.input_at::<u64>(inputs[0]).unwrap();
        let strings = runner.input_at::<String>(inputs[1]).unwrap();

        let mut numbers = numbers.begin().unwrap();
        let mut strings = strings.begin().unwrap();
        numbers.insert(Key::from(1_u64), 10).unwrap();
        strings
            .insert(Key::from(2_u64), String::from("two"))
            .unwrap();
        let _numbers = numbers.seal().unwrap();
        let _strings = strings.seal().unwrap();

        let mut run = runner.settle().unwrap();
        assert_eq!(run.report().settlements().len(), 2);
        assert!(matches!(
            run.output::<u64>().unwrap().next(),
            Some(Change::Insert(key, 10)) if key == Key::from(1_u64)
        ));
        assert!(matches!(
            run.output::<String>().unwrap().next(),
            Some(Change::Insert(key, value))
                if key == Key::from(2_u64) && value == "two"
        ));
    }
}
