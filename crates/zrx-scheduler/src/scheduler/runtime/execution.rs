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

//! Affine ownership carried from resident selection through reconciliation.

use crossbeam::channel::Receiver;

use zrx_executor::Strategy;

use crate::scheduler::Id;
use crate::scheduler::action::{
    EvaluationChanges, Instrumentation, Job, Outcomes, Segment, WakeRequest,
};

use super::frame::ProgressFrame;
use super::progress::Obligations;
use super::transport::OutputReservations;

mod invocation;
mod jobs;
mod placement;
mod scheduling;

pub use invocation::{Completion, Invocation};
use jobs::Jobs;
pub use jobs::{Access, Started};
use placement::Placement;
pub use placement::{Backend, Return, Submission};
pub use scheduling::InvocationClass;
use scheduling::Scheduling;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Ownership of node jobs, runnable order, and physical placement.
pub struct Execution<I, S>
where
    I: Id,
    S: Strategy,
{
    jobs: Jobs<I>,
    placement: Placement<Returned<I>, S>,
    scheduling: Scheduling,
}

// ----------------------------------------------------------------------------

pub struct InputAuthority {
    pub obligations: Obligations,
}

// ----------------------------------------------------------------------------

pub struct Dispatch<I>
where
    I: Id,
{
    pub invocation: Invocation<I>,
    pub inputs: InputAuthority,
    pub outputs: OutputReservations,
    pub progress: Option<ProgressContinuation>,
    pub sequence: u64,
    pub access: Access,
}

// ----------------------------------------------------------------------------

pub struct Returned<I>
where
    I: Id,
{
    pub completion: Completion<I>,
    pub inputs: InputAuthority,
    pub outputs: OutputReservations,
    pub progress: Option<ProgressContinuation>,
    pub sequence: u64,
    pub access: Access,
}

// ----------------------------------------------------------------------------

pub struct Reconciliation<I>
where
    I: Id,
{
    pub output: Option<Segment<I>>,
    pub outcomes: Outcomes,
    pub evaluations: EvaluationChanges<I>,
    pub instrumentation: Instrumentation,
    pub wakes: Vec<WakeRequest>,
    pub inputs: InputAuthority,
    pub outputs: OutputReservations,
    pub progress: Option<ProgressContinuation>,
}

// ----------------------------------------------------------------------------

pub struct ProgressContinuation {
    pub frame: ProgressFrame,
    pub routes: OutputReservations,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, S> Execution<I, S>
where
    I: Id,
    S: Strategy,
{
    pub fn new(jobs: Vec<Job<I>>, backend: Backend<S>) -> Self {
        let nodes = jobs.len();
        let parallelism = backend.workers().max(1);
        Self {
            jobs: Jobs::new(jobs, parallelism),
            placement: Placement::new(backend),
            scheduling: Scheduling::new(nodes),
        }
    }

    pub fn slice(&self, node: usize, items: usize) -> usize {
        Scheduling::slice(items, self.jobs.parallelism(node))
    }

    pub fn receiver(&self) -> Option<&Receiver<Return<Returned<I>>>> {
        self.placement.receiver()
    }

    pub fn pending(&self) -> bool {
        !self.placement.is_idle()
    }

    pub fn accepts(&self) -> bool {
        self.placement.accepts()
    }

    pub fn retry(&mut self) -> bool {
        self.placement.retry()
    }

    pub fn receive(&mut self) -> Option<Return<Returned<I>>> {
        self.placement.try_recv()
    }

    pub fn submit(&mut self, dispatch: Dispatch<I>) -> Submission<Returned<I>> {
        self.placement.submit(move || dispatch.run())
    }

    pub fn enqueue(&mut self, node: usize) {
        self.scheduling.enqueue(node);
    }

    pub fn next(&mut self) -> Option<usize> {
        self.scheduling.pop()
    }

    pub fn classes(&self, node: usize) -> [InvocationClass; 3] {
        self.scheduling.classes(node)
    }

    pub fn select(&mut self, node: usize, class: InvocationClass) {
        self.scheduling.selected(node, class);
    }

    pub fn lane(&self, node: usize) -> usize {
        self.scheduling.next_lane(node)
    }

    pub fn advance(&mut self, node: usize, lane: usize, lanes: usize) {
        self.scheduling.selected_lane(node, lane, lanes);
    }

    pub fn ready(&self, node: usize, access: Access) -> bool {
        self.jobs.ready(node, access)
    }

    pub fn reconciled(&self, node: usize, sequence: u64) -> bool {
        self.jobs.reconciled(node, sequence)
    }

    pub fn start(&mut self, node: usize, access: Access) -> Started<I> {
        self.jobs.start(node, access)
    }

    pub fn complete(
        &mut self, node: usize, sequence: u64, access: Access, job: Job<I>,
        reconciliation: Reconciliation<I>,
    ) -> Option<Reconciliation<I>> {
        self.jobs
            .complete(node, sequence, access, job, reconciliation)
    }

    pub fn reconcile(&mut self, node: usize) -> Option<Reconciliation<I>> {
        self.jobs.pop_ready(node)
    }
}

// ----------------------------------------------------------------------------

impl<I> Dispatch<I>
where
    I: Id,
{
    pub fn new(
        invocation: Invocation<I>, inputs: InputAuthority,
        outputs: OutputReservations, progress: Option<ProgressContinuation>,
        sequence: u64, access: Access,
    ) -> Self {
        Self {
            invocation,
            inputs,
            outputs,
            progress,
            sequence,
            access,
        }
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "action",
            parent = None,
            skip_all,
            fields(
                node = self.invocation.node(),
                sequence = self.sequence,
                revision = %self.inputs.obligations.revision(),
                batch_items = self.invocation.batch_items(),
                access = self.access.as_str(),
            )
        )
    )]
    pub fn run(self) -> Returned<I> {
        let Self {
            invocation,
            inputs,
            outputs,
            progress,
            sequence,
            access,
        } = self;
        let completion = invocation.run();
        Returned {
            completion,
            inputs,
            outputs,
            progress,
            sequence,
            access,
        }
    }
}
