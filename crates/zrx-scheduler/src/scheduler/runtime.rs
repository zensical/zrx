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

//! Minimal resident kernel over static routes and FIFO input lanes.

use crossbeam::channel::Select;
use std::num::NonZeroUsize;
use std::panic;
use std::time::Instant;
use thiserror::Error;

use zrx_executor::Strategy;
use zrx_executor::strategy::Immediate;
use zrx_graph::Graph;

use crate::scheduler::Id;

use super::action::EvaluationChange;
use super::action::control::ProgressEvent;
use super::action::{Port, Segment, WakeKey, WakeRequest};
use super::plan::{InputId, InputIndex, NodePlan, Plan};
use super::{
    Admit, CurrentError, InvocationReport, Report, RevisionId, Settlement,
};

mod execution;
mod frame;
mod ingress;
mod ordered;
mod progress;
mod transport;
mod wake;

pub use execution::Backend;
use execution::InvocationClass;
use execution::{
    Access, Completion, Dispatch, Execution, InputAuthority, Invocation,
    ProgressContinuation, Reconciliation, Return, Returned, Started,
    Submission,
};
use ingress::Sources;
use progress::{
    Obligation, Obligations, ProgressBranches, ProgressIdentity, Progresses,
    Revisions,
};
use transport::{
    Credit, Data, DestinationReservation, Entry, OutputReservations, Pruned,
    Transport,
};
use wake::{Due, Wakes, deduplicate};

use ingress::Error as IngressError;
use progress::Error as RevisionError;
pub use transport::{Egress, EgressIter};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid external operation on the resident runtime.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OperationError {
    /// Source ingress rejected the operation.
    #[error(transparent)]
    Ingress(#[from] IngressError),
    /// Revision progress rejected the transition.
    #[error(transparent)]
    Revision(#[from] RevisionError),
}

// ----------------------------------------------------------------------------

enum Ready {
    Progress(ProgressReady),
    Data { lane: usize },
    Wake,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ProgressReady {
    Direct {
        lane: usize,
        identity: ProgressIdentity,
    },
    Arrival {
        lane: usize,
        identity: ProgressIdentity,
    },
    Complete(ProgressIdentity),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One bounded scheduler tick and its owning report drain.
#[must_use]
pub struct Tick {
    progressed: bool,
    report: Report,
}

// ----------------------------------------------------------------------------

/// Fenced attached plan that may only reconcile committed invocations.
#[must_use]
pub struct Retiring<I, S = Immediate>
where
    I: Id,
    S: Strategy,
{
    runtime: Runtime<I, S>,
    report: Report,
}

// ----------------------------------------------------------------------------

/// Borrow-free classification of one runtime's selectable future readiness.
#[derive(Clone, Copy, Debug)]
pub struct Readiness {
    completion: Option<usize>,
    pending: bool,
    deadline: Option<Instant>,
}

// ----------------------------------------------------------------------------

/// Single-threaded orchestration state for one attached plan.
pub struct Runtime<I, S = Immediate>
where
    I: Id,
    S: Strategy,
{
    graph: Graph<NodePlan>,
    sources: Sources,
    progresses: Progresses,
    progress_branches: ProgressBranches,
    transport: Transport<I>,
    execution: Execution<I, S>,
    revisions: Revisions,
    wakes: Wakes,
    report: Report,
    current_errors: Vec<CurrentError<I>>,
    retiring: bool,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl ProgressReady {
    const fn delivery(self) -> Option<ProgressIdentity> {
        match self {
            Self::Direct { identity, .. } | Self::Complete(identity) => {
                Some(identity)
            }
            Self::Arrival { .. } => None,
        }
    }
}

// ----------------------------------------------------------------------------

impl Tick {
    pub(super) fn admitted() -> Self {
        Self {
            progressed: true,
            report: Report::default(),
        }
    }

    pub(super) fn available(&self) -> bool {
        self.progressed || !self.report.is_empty()
    }

    /// Returns whether the runtime consumed, submitted, or completed work.
    #[must_use]
    pub const fn progressed(&self) -> bool {
        self.progressed
    }

    /// Consumes the tick and returns its owning report.
    pub fn into_report(self) -> Report {
        self.report
    }
}

// ----------------------------------------------------------------------------

impl<I, S> Retiring<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Reconciles at most one immediately available committed invocation.
    ///
    /// # Panics
    ///
    /// Resumes an action panic and panics if resident ownership is internally
    /// inconsistent.
    pub fn tick(&mut self) -> bool {
        let tick = self.runtime.tick();
        let progressed = tick.progressed();
        self.report.append(tick.into_report());
        progressed
    }

    /// Returns whether all committed work has returned and settled.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.runtime.revisions.is_empty()
    }

    pub fn register<'a>(&'a self, select: &mut Select<'a>) -> Readiness {
        self.runtime.register(select)
    }

    /// Attempts to finish retirement, returning the fenced runtime unchanged
    /// while committed work remains.
    ///
    /// # Errors
    ///
    /// Returns this retirement unchanged while committed work remains.
    ///
    /// # Panics
    ///
    /// Panics if a completely drained retirement produced external output.
    #[allow(
        clippy::result_large_err,
        reason = "the affine transition must return the unboxed runtime intact"
    )]
    pub fn try_finish(mut self) -> Result<Report, Self> {
        if !self.is_complete() {
            return Err(self);
        }
        assert!(self.runtime.egress().is_none());
        Ok(self.report)
    }
}

// ----------------------------------------------------------------------------

impl Readiness {
    pub const fn completion(self) -> Option<usize> {
        self.completion
    }

    pub const fn pending(self) -> bool {
        self.pending
    }

    /// Returns the earliest wake deadline captured during registration.
    #[must_use]
    pub const fn deadline(self) -> Option<Instant> {
        self.deadline
    }
}

// ----------------------------------------------------------------------------

impl<I, S> Runtime<I, S>
where
    I: Id,
    S: Strategy,
{
    pub fn install(plan: Plan<I>, backend: Backend<S>) -> Self {
        let Plan {
            graph,
            jobs,
            inputs,
            inputs_by_id,
            outputs,
            progress,
            progress_by_input,
        } = plan;
        let nodes = jobs.len();
        let lane_capacity = progress_by_input
            .iter()
            .map(|progress| 1 + usize::from(progress.is_some()))
            .max()
            .unwrap_or(1);
        for binding in &inputs {
            debug_assert!(
                graph
                    .as_ref()
                    .get(binding.route.node)
                    .and_then(|node| node.inputs.get(binding.route.lane))
                    .is_some(),
                "plan retained an invalid input binding"
            );
        }
        let transport = Transport::new(&graph, lane_capacity, outputs);
        let execution = Execution::new(jobs, backend);
        let progresses = Progresses::new(progress, progress_by_input);
        Self {
            graph,
            sources: Sources::new(inputs, inputs_by_id),
            progresses,
            progress_branches: ProgressBranches::new(nodes),
            transport,
            execution,
            revisions: Revisions::default(),
            wakes: Wakes::new(nodes),
            report: Report::default(),
            current_errors: Vec::new(),
            retiring: false,
        }
    }

    /// Opens one source revision and queues its progress begin frame.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown input or a second open revision from
    /// the same source. Revisions may overlap after the earlier revision is
    /// sealed, or when they belong to different inputs, but one input has at
    /// most one open revision.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "revision.begin",
            skip_all,
            fields(input = ?input)
        )
    )]
    pub fn begin(
        &mut self, input: InputId,
    ) -> Result<Admit<RevisionId, InputId>, OperationError> {
        let (index, source) = self.sources.available(input)?;
        let count = usize::from(self.progresses.contains(index));
        let Some(reservations) =
            self.transport.reserve_repeated(source.route, count)
        else {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "revision.blocked",
                tracing::Level::TRACE,
                operation = "begin",
                ?input,
            );
            return Ok(Admit::Full(input));
        };
        let revision = self.revisions.begin(index);
        self.sources.open(index, revision);
        self.enqueue_boundary(
            revision,
            index,
            ProgressEvent::Begin,
            reservations,
        )?;
        Ok(Admit::Accepted(revision))
    }

    /// Admits one typed root segment at any installed graph position.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown revision, node, or lane, a mismatched
    /// segment port.
    ///
    /// # Panics
    ///
    /// Panics if internal admission accounting does not create the reserved
    /// data authority.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "ingress",
            skip_all,
            fields(
                revision = %revision,
                batch_items = segment.len(),
            )
        )
    )]
    pub fn ingress(
        &mut self, revision: RevisionId, segment: Segment<I>,
    ) -> Result<Admit<(), Segment<I>>, OperationError> {
        let index = self
            .revisions
            .input(revision)
            .ok_or(RevisionError::Inactive(revision))?;
        let source = self
            .sources
            .active(index, revision)
            .ok_or(RevisionError::Inactive(revision))?;
        if segment.port() != source.port {
            return Err(IngressError::Port {
                node: source.route.node,
                lane: source.route.lane,
            }
            .into());
        }
        let Some(mut reservations) =
            self.transport.reserve_repeated(source.route, 1)
        else {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "ingress.blocked",
                tracing::Level::TRACE,
                revision = %revision,
                batch_items = segment.len(),
            );
            return Ok(Admit::Full(segment));
        };
        let mut obligations = self.revisions.admit_many(revision, 1)?;
        let data = obligations
            .next()
            .expect("data admission created one obligation");
        let data_reservation = reservations
            .next()
            .expect("data admission reserved one destination");
        self.commit_destination(
            data_reservation,
            Some(Entry::data(segment, data)),
        );
        assert!(obligations.next().is_none());
        assert!(reservations.next().is_none());
        Ok(Admit::Accepted(()))
    }

    /// Seals one revision and records immediate empty settlement.
    ///
    /// # Errors
    ///
    /// Returns an error when the revision is unknown.
    ///
    /// # Panics
    ///
    /// Panics if active source attribution is inconsistent with the revision.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "revision.seal",
            skip_all,
            fields(revision = %id)
        )
    )]
    pub fn seal(
        &mut self, id: RevisionId,
    ) -> Result<Admit<(), RevisionId>, OperationError> {
        let index = self
            .revisions
            .input(id)
            .ok_or(RevisionError::Inactive(id))?;
        let source = self
            .sources
            .active(index, id)
            .ok_or(RevisionError::Inactive(id))?;
        let count = usize::from(self.progresses.contains(index));
        let Some(reservations) =
            self.transport.reserve_repeated(source.route, count)
        else {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "revision.blocked",
                tracing::Level::TRACE,
                operation = "seal",
                revision = %id,
            );
            return Ok(Admit::Full(id));
        };
        self.enqueue_boundary(id, index, ProgressEvent::End, reservations)?;
        let settlement = self.revisions.seal(id)?;
        assert!(
            self.sources.close(index, id),
            "sealed source revision was not open"
        );
        self.record_settlement(settlement);
        Ok(Admit::Accepted(()))
    }

    /// Aborts an open or sealed revision and prunes every undispatched
    /// segment, external output, and wake.
    ///
    /// Already dispatched action batches remain committed and settle through
    /// their ordinary completion path. Abort does not roll back action-owned
    /// state mutated by those batches; that state can affect later revisions.
    ///
    /// # Errors
    ///
    /// Returns an error when the revision is unknown or already terminal.
    ///
    /// # Panics
    ///
    /// Panics if active source attribution is inconsistent with the revision.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "revision.abort",
            skip_all,
            fields(revision = %id)
        )
    )]
    pub fn abort(
        &mut self, id: RevisionId,
    ) -> Result<Admit<(), RevisionId>, OperationError> {
        let index = self
            .revisions
            .input(id)
            .ok_or(RevisionError::Inactive(id))?;
        let source = self.sources.source_at(index);
        if self.sources.active(index, id).is_some() {
            let count = usize::from(self.progresses.contains(index));
            let Some(reservations) =
                self.transport.reserve_repeated(source.route, count)
            else {
                #[cfg(feature = "tracing")]
                tracing::event!(
                    name: "revision.blocked",
                    tracing::Level::TRACE,
                    operation = "abort",
                    revision = %id,
                );
                return Ok(Admit::Full(id));
            };
            self.enqueue_boundary(
                id,
                index,
                ProgressEvent::Abort,
                reservations,
            )?;
            assert!(
                self.sources.close(index, id),
                "aborted source revision was not open"
            );
        } else {
            self.transport.abort_revision_end(id);
            self.progress_branches.abort_revision_end(id);
        }
        let settlement = self.revisions.abort(id)?;
        self.record_settlement(settlement);
        self.prune_transport(id);
        self.prune_wakes(id);
        Ok(Admit::Accepted(()))
    }

    pub(super) fn input_port(&self, input: InputId) -> Option<Port> {
        self.sources.source(input).map(|source| source.port)
    }

    /// Accepts the next fairly selected committed external output batch.
    ///
    /// Acceptance transfers the segment to the caller, releases its boundary
    /// credit, and retires its scheduler revision obligation. Any resulting
    /// settlement is delivered only through the owning [`Report`].
    ///
    /// # Panics
    ///
    /// Panics if the boundary's visible-entry classification is inconsistent.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "egress", skip_all)
    )]
    pub fn egress(&mut self) -> Option<Egress<I>> {
        let (source, egress, obligation) = self.transport.egress()?;
        self.schedule(source);
        let settlement = self.revisions.retire(obligation);
        self.record_settlement(settlement);
        Some(egress)
    }

    /// Registers future worker readiness in a caller-owned selector.
    ///
    /// Selection consumes nothing. After dropping the selector, call
    /// [`Self::tick`] to import and authenticate current work. The returned
    /// deadline is used as the selector timeout alongside external sources.
    pub fn register<'a>(&'a self, select: &mut Select<'a>) -> Readiness {
        Readiness {
            completion: self
                .execution
                .receiver()
                .map(|receiver| select.recv(receiver)),
            pending: self.execution.pending(),
            deadline: self.wakes.next_deadline(),
        }
    }

    /// Performs at most one bounded scheduler tick and drains its reports.
    pub fn tick(&mut self) -> Tick {
        let progressed = self.run_one();
        Tick {
            progressed,
            report: std::mem::take(&mut self.report),
        }
    }

    fn run_one(&mut self) -> bool {
        if let Some(returned) = self.execution.receive() {
            self.accept(returned);
            return true;
        }
        if self.execution.retry() {
            return true;
        }
        if self.activate_due_wake() {
            return true;
        }
        if !self.execution.accepts() {
            return false;
        }
        while let Some(node) = self.execution.next() {
            let Some(ready) = self.select(node) else {
                continue;
            };
            match ready {
                Ready::Progress(ready) => {
                    if !self.run_progress(node, ready) {
                        continue;
                    }
                }
                Ready::Data { lane } => {
                    if !self.dispatch_data(node, lane) {
                        continue;
                    }
                }
                Ready::Wake => {
                    if !self.dispatch_wake(node) {
                        continue;
                    }
                }
            }
            self.schedule(node);
            return true;
        }
        false
    }

    /// Fences all ingress, aborts retained work, and enters completion-only
    /// retirement.
    ///
    /// No new action is dispatched after this transition.
    ///
    /// # Panics
    ///
    /// Panics if resident progress, transport, or source ownership is
    /// internally inconsistent.
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "retirement", skip_all)
    )]
    pub fn begin_retirement(mut self) -> Retiring<I, S> {
        self.retiring = true;
        self.current_errors.clear();
        let mut report = std::mem::take(&mut self.report);
        while let Some(old) = self.egress() {
            drop(old);
        }
        report.append(std::mem::take(&mut self.report));

        // Retirement does not deliver abort events into actions whose whole
        // state is about to disappear. Fence every revision directly, prune
        // undispatched transport, then wait only for committed callbacks.
        let revisions: Vec<_> = self.revisions.ids().collect();
        for revision in revisions {
            self.fence_revision(revision);
        }
        report.append(std::mem::take(&mut self.report));
        Retiring { runtime: self, report }
    }

    pub(super) fn errors(&self) -> &[CurrentError<I>] {
        &self.current_errors
    }

    fn apply_evaluations(
        &mut self, node: usize,
        evaluations: super::action::EvaluationChanges<I>,
    ) {
        if self.retiring {
            return;
        }
        for change in evaluations {
            match change {
                EvaluationChange::Reject { evaluation, error } => {
                    if let Some(current) =
                        self.current_errors.iter_mut().find(|current| {
                            current.node == node
                                && evaluation
                                    .matches(current.domain, &current.key)
                        })
                    {
                        current.error = error;
                    } else {
                        self.current_errors.push(CurrentError {
                            node,
                            domain: evaluation.domain,
                            key: evaluation.key,
                            error,
                        });
                    }
                }
                EvaluationChange::Resolve(evaluation) => {
                    if let Some(index) =
                        self.current_errors.iter().position(|current| {
                            current.node == node
                                && evaluation
                                    .matches(current.domain, &current.key)
                        })
                    {
                        self.current_errors.remove(index);
                    }
                }
            }
        }
    }

    fn enqueue_boundary(
        &mut self, revision: RevisionId, input: InputIndex,
        event: ProgressEvent, mut reservations: OutputReservations,
    ) -> Result<(), OperationError> {
        let count = usize::from(self.progresses.contains(input));
        let obligations = self.revisions.admit_many(revision, count)?;
        let mut obligations = obligations.into_iter();
        let progress = self.progresses.boundary(input, event);
        if let Some(frame) = progress {
            let obligation = obligations
                .next()
                .expect("progress boundary created its obligation");
            let reservation = reservations
                .next()
                .expect("progress boundary reserved its destination");
            self.commit_destination(
                reservation,
                Some(Entry::progress(frame, obligation)),
            );
        }
        assert!(obligations.next().is_none());
        assert!(reservations.next().is_none());
        Ok(())
    }

    fn schedule(&mut self, node: usize) {
        let progress = self.ready_progress(node);
        let progress_ready = progress.is_some_and(|ready| {
            let Some(identity) = ready.delivery() else {
                return true;
            };
            !self.progresses.node(identity.progress(), node).subscriber
                || self.execution.ready(node, Access::Exclusive)
        });
        let data_ready = self.execution.ready(node, Access::Shared)
            && self.ready_data(node).is_some();
        let wake_ready = self.execution.ready(node, Access::Exclusive)
            && self.wakes.has_due(node);
        if progress_ready || data_ready || wake_ready {
            self.execution.enqueue(node);
        }
    }

    fn ready_progress(&self, node: usize) -> Option<ProgressReady> {
        if let Some(identity) = self.progress_branches.ready(node) {
            return Some(ProgressReady::Complete(identity));
        }
        (0..self.transport.lane_count(node)).find_map(|lane| {
            let identity = self.transport.front_progress(node, lane)?;
            let plan = self.progresses.node(identity.progress(), node);
            assert!(
                plan.lanes.contains(&lane),
                "progress reached an irrelevant lane"
            );
            // Invocation outcomes are authoritative only after the action work
            // that preceded this frame on the same FIFO lane reconciles.
            if self.transport.progress_predecessor(node, lane).is_some_and(
                |sequence| !self.execution.reconciled(node, sequence),
            ) {
                return None;
            }
            if self.transport.front_progress_is_end(node, lane)
                && self.wakes.holds_end(node, identity.revision())
            {
                return None;
            }
            if plan.lanes.len() == 1 {
                Some(ProgressReady::Direct { lane, identity })
            } else {
                Some(ProgressReady::Arrival { lane, identity })
            }
        })
    }

    fn select(&mut self, node: usize) -> Option<Ready> {
        for class in self.execution.classes(node) {
            let ready = match class {
                InvocationClass::Event => self
                    .ready_progress(node)
                    .filter(|ready| {
                        let Some(identity) = ready.delivery() else {
                            return true;
                        };
                        !self
                            .progresses
                            .node(identity.progress(), node)
                            .subscriber
                            || self.execution.ready(node, Access::Exclusive)
                    })
                    .map(Ready::Progress),
                InvocationClass::Data => self
                    .execution
                    .ready(node, Access::Shared)
                    .then(|| self.ready_data(node))
                    .flatten()
                    .map(|lane| Ready::Data { lane }),
                InvocationClass::Wake => {
                    (self.execution.ready(node, Access::Exclusive)
                        && self.wakes.has_due(node))
                    .then_some(Ready::Wake)
                }
            };
            if let Some(ready) = ready {
                self.execution.select(node, class);
                return Some(ready);
            }
        }
        None
    }

    fn ready_data(&self, node: usize) -> Option<usize> {
        let lane_count = self.transport.lane_count(node);
        (0..lane_count).find_map(|offset| {
            let lane = (self.execution.lane(node) + offset) % lane_count;
            self.transport.front_data(node, lane).map(|_| lane)
        })
    }

    fn dispatch_data(&mut self, node: usize, lane: usize) -> bool {
        let Some(reservations) = self
            .transport
            .reserve_destinations(&self.graph[node].destinations)
        else {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "dispatch.blocked",
                tracing::Level::TRACE,
                kind = "data",
                node,
                lane,
            );
            return false;
        };
        let lane_count = self.transport.lane_count(node);
        self.execution.advance(node, lane, lane_count);
        let Data { segment, obligation, quantum } = self
            .transport
            .take_data(node, lane)
            .expect("selected data remains at the lane front");
        // Choose the quantum once for this arriving segment. Recomputing it
        // from every shrinking tail geometrically fragments batches at each
        // graph hop and eventually degrades pipelines into one-item jobs.
        let quantum = quantum.unwrap_or_else(|| {
            NonZeroUsize::new(self.execution.slice(node, segment.len()))
                .expect("non-empty data has a non-zero scheduling quantum")
        });
        let (slice, tail) = segment.split_prefix(quantum.get());
        let obligation = if let Some(tail) = tail {
            let (mut obligations, settlement) =
                self.revisions.replace(obligation, 2);
            self.record_settlement(settlement);
            let slice = obligations
                .next()
                .expect("slice replacement created two obligations");
            let tail_obligation = obligations
                .next()
                .expect("slice replacement created two obligations");
            self.transport.restore_data(
                node,
                lane,
                Data::tail(tail, tail_obligation, quantum),
            );
            slice
        } else {
            self.release_lane(node, lane);
            obligation
        };
        // Generate lane positions directly into Invocation's inline-or-spill
        // input storage. Fixed actions stay inline; only homogeneous fan-in
        // wider than eight allocates its lane table.
        let mut slice = Some(slice);
        let segments = (0..lane_count).map(|position| {
            if position == lane { slice.take() } else { None }
        });

        let connected = !reservations.is_empty();
        let Started { sequence, job } =
            self.execution.start(node, Access::Shared);
        self.transport.record_dispatch(node, lane, sequence);
        let revision = obligation.revision();
        let invocation =
            Invocation::new(revision, node, job, segments, connected);
        self.dispatch(
            obligation.into(),
            reservations,
            None,
            invocation,
            sequence,
            Access::Shared,
        );
        true
    }

    fn dispatch_wake(&mut self, node: usize) -> bool {
        let Some(reservations) = self
            .transport
            .reserve_destinations(&self.graph[node].destinations)
        else {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "dispatch.blocked",
                tracing::Level::TRACE,
                kind = "wake",
                node,
            );
            return false;
        };
        let scheduled = self
            .wakes
            .take_due(node)
            .expect("ready wake remains resident");
        #[cfg(feature = "tracing")]
        tracing::event!(
            name: "wake.fired",
            tracing::Level::DEBUG,
            node,
            key = ?scheduled.key,
        );
        assert_eq!(scheduled.owner, node, "wake reached another job");
        debug_assert!(scheduled.deadline <= Instant::now());
        let connected = !reservations.is_empty();
        let Started { sequence, job } =
            self.execution.start(node, Access::Exclusive);
        let revision = scheduled.authority.revision();
        let invocation = Invocation::wake(
            revision,
            node,
            job,
            scheduled.key,
            scheduled.deadline,
            connected,
        );
        self.dispatch(
            scheduled.authority.fire().into(),
            reservations,
            None,
            invocation,
            sequence,
            Access::Exclusive,
        );
        true
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "dispatch",
            skip_all,
            fields(
                node = invocation.node(),
                sequence,
                revision = %obligations.revision(),
                access = access.as_str(),
            )
        )
    )]
    fn dispatch(
        &mut self, obligations: Obligations, reservations: OutputReservations,
        progress: Option<ProgressContinuation>, invocation: Invocation<I>,
        sequence: u64, access: Access,
    ) {
        let dispatch = Dispatch::new(
            invocation,
            InputAuthority { obligations },
            reservations,
            progress,
            sequence,
            access,
        );
        if let Submission::Inline(returned) = self.execution.submit(dispatch) {
            self.accept(returned);
        }
    }

    fn accept(&mut self, returned: Return<Returned<I>>) {
        let returned = match returned {
            Return::Completed(returned) => returned,
            Return::Panicked(payload) => panic::resume_unwind(payload),
        };
        let Returned {
            completion,
            inputs,
            outputs,
            progress,
            sequence,
            access,
        } = returned;
        let Completion {
            node,
            job,
            output,
            outcomes,
            evaluations,
            instrumentation,
            wakes,
        } = completion;
        let ready = self.execution.complete(
            node,
            sequence,
            access,
            job,
            Reconciliation {
                output,
                outcomes,
                evaluations,
                instrumentation,
                wakes,
                inputs,
                outputs,
                progress,
            },
        );
        self.reconcile(node, ready);
        self.schedule(node);
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "reconcile",
            skip_all,
            fields(node)
        )
    )]
    fn reconcile(
        &mut self, node: usize, mut reconciliation: Option<Reconciliation<I>>,
    ) {
        while let Some(current) = reconciliation {
            let Reconciliation {
                output,
                outcomes,
                evaluations,
                instrumentation,
                wakes,
                inputs,
                outputs,
                progress,
            } = current;
            let revision = inputs.obligations.revision();
            self.apply_evaluations(node, evaluations);
            if !outcomes.is_empty() || !instrumentation.is_empty() {
                self.report.invocations.push(InvocationReport {
                    revision,
                    node,
                    outcomes,
                    instrumentation,
                });
            }
            self.route_data(node, inputs, outputs, progress, output, wakes);
            reconciliation = self.execution.reconcile(node);
        }
    }

    fn route_data(
        &mut self, node: usize, inputs: InputAuthority,
        outputs: OutputReservations, progress: Option<ProgressContinuation>,
        output: Option<Segment<I>>, wakes: Vec<WakeRequest>,
    ) {
        let InputAuthority { obligations } = inputs;
        let revision = obligations.revision();
        let reservations = outputs;
        let aborted = self.revisions.is_aborted(revision);
        let mut wakes = wakes;
        deduplicate(&mut wakes);
        let routes = if self.retiring {
            0
        } else {
            output.as_ref().map_or(0, |_| reservations.len())
        };
        let forward_progress = !self.retiring
            && progress.as_ref().is_some_and(|progress| {
                !aborted || progress.frame.is_end() || progress.frame.is_abort()
            });
        let progress_routes = progress
            .as_ref()
            .filter(|_| forward_progress)
            .map_or(0, |progress| progress.routes.len());
        let timers = if aborted {
            0
        } else {
            wakes
                .iter()
                .filter(|wake| wake.deadline().is_some())
                .count()
        };
        let (authorities, settlement) = self
            .revisions
            .replace_many(obligations, routes + progress_routes + timers);
        self.record_settlement(settlement);
        let mut authorities = authorities.into_iter();
        if let Some(output) = output.filter(|_| !self.retiring) {
            // fan_out is an owning iterator. Keep this zip streaming so the
            // common one-destination path never allocates a lease vector.
            let segments = output.fan_out(reservations.len());
            for (reservation, segment) in reservations.into_iter().zip(segments)
            {
                let obligation = authorities
                    .next()
                    .expect("destination successor authority was allocated");
                self.commit_destination(
                    reservation,
                    Some(Entry::data(segment, obligation)),
                );
            }
        } else {
            for reservation in reservations {
                self.commit_destination(reservation, None);
            }
        }
        if let Some(mut progress) = progress {
            if forward_progress {
                if aborted && progress.frame.is_end() {
                    progress.frame.abort_end();
                }
                for reservation in progress.routes {
                    let obligation = authorities
                        .next()
                        .expect("progress successor authority was allocated");
                    self.commit_destination(
                        reservation,
                        Some(Entry::progress(
                            progress.frame.clone(),
                            obligation,
                        )),
                    );
                }
            } else {
                for reservation in progress.routes {
                    self.commit_destination(reservation, None);
                }
            }
        }
        for request in wakes {
            let (key, deadline) = request.into_parts();
            match deadline {
                Some(deadline) if !aborted => {
                    let obligation = authorities
                        .next()
                        .expect("wake successor authority was allocated");
                    self.install_wake(node, key, deadline, obligation);
                }
                Some(_) => {}
                None => {
                    self.clear_current_wake(node, key);
                }
            }
        }
        assert!(
            authorities.next().is_none(),
            "successor authority was not installed"
        );
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "progress",
            skip_all,
            fields(node)
        )
    )]
    fn run_progress(&mut self, node: usize, ready: ProgressReady) -> bool {
        if let ProgressReady::Arrival { lane, identity } = ready {
            let expected =
                self.progresses.node(identity.progress(), node).lanes.len();
            let (frame, obligation) =
                self.transport.take_progress(node, lane, identity);
            self.release_lane(node, lane);
            self.progress_branches
                .arrive(node, lane, expected, frame, obligation);
            return true;
        }
        let identity = ready
            .delivery()
            .expect("progress delivery lost its identity");
        let lane = match ready {
            ProgressReady::Direct { lane, .. } => Some(lane),
            ProgressReady::Complete(_) => None,
            ProgressReady::Arrival { .. } => {
                unreachable!("progress arrival reached delivery")
            }
        };
        let plan = self.progresses.node(identity.progress(), node);
        let subscriber = plan.subscriber;
        let reservations = if subscriber {
            self.transport.reserve_action_and_progress(
                &self.graph[node].destinations,
                &plan.routes,
            )
        } else {
            self.transport
                .reserve_routes(&plan.routes)
                .map(|routes| (OutputReservations::empty(), routes))
        };
        let Some((outputs, routes)) = reservations else {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "progress.blocked",
                tracing::Level::TRACE,
                node,
                revision = %identity.revision(),
                progress = ?identity.progress(),
            );
            return false;
        };
        let (frame, obligations) = if let Some(lane) = lane {
            let (frame, obligation) =
                self.transport.take_progress(node, lane, identity);
            self.release_lane(node, lane);
            (frame, obligation.into())
        } else {
            self.progress_branches.take(node, identity)
        };
        if subscriber {
            let connected = !outputs.is_empty();
            let event = frame.event().clone();
            let Started { sequence, job } =
                self.execution.start(node, Access::Exclusive);
            let invocation = Invocation::progress(
                identity.revision(),
                node,
                job,
                event,
                connected,
            );
            self.dispatch(
                obligations,
                outputs,
                Some(ProgressContinuation { frame, routes }),
                invocation,
                sequence,
                Access::Exclusive,
            );
            return true;
        }
        let successors = routes.len();
        let (authorities, settlement) =
            self.revisions.replace_many(obligations, successors);
        self.record_settlement(settlement);
        for (reservation, obligation) in routes.zip(authorities) {
            self.commit_destination(
                reservation,
                Some(Entry::progress(frame.clone(), obligation)),
            );
        }
        true
    }

    fn record_settlement(&mut self, settlement: Option<Settlement>) {
        if let Some(settlement) = settlement {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "revision.settled",
                tracing::Level::DEBUG,
                ?settlement,
            );
            self.report.settlements.push(settlement);
        }
    }

    fn fence_revision(&mut self, revision: RevisionId) {
        if self.revisions.is_aborted(revision) {
            return;
        }
        let input = self
            .revisions
            .input(revision)
            .expect("resident revision retains source attribution");
        if self.sources.active(input, revision).is_some() {
            assert!(
                self.sources.close(input, revision),
                "retired source revision was not open"
            );
        } else {
            self.transport.abort_revision_end(revision);
            self.progress_branches.abort_revision_end(revision);
        }
        let settlement = self
            .revisions
            .abort(revision)
            .expect("resident revision remains valid during retirement");
        self.record_settlement(settlement);
        self.prune_transport_all(revision);
        self.prune_wakes(revision);
    }

    fn commit_destination(
        &mut self, reservation: DestinationReservation, entry: Option<Entry<I>>,
    ) {
        let update = self.transport.commit(reservation, entry);
        if let Some(credit) = update.credit {
            match credit {
                Credit::Lane(route) => {
                    self.release_lane(route.node, route.lane);
                }
                Credit::Output(source) => self.schedule(source),
            }
        }
        if let Some(node) = update.ready {
            self.schedule(node);
        }
    }

    fn release_lane(&mut self, node: usize, _lane: usize) {
        let count = self.graph.adjacent(node).1.incoming.len();
        for index in 0..count {
            let producer = self.graph.adjacent(node).1.incoming[index];
            self.schedule(producer);
        }
    }

    fn activate_due_wake(&mut self) -> bool {
        let Some(owner) = self.wakes.mark_due() else {
            return false;
        };
        if let Due::Current(owner) = owner {
            self.schedule(owner);
        }
        true
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "wake.install",
            skip_all,
            fields(
                node,
                key = ?key,
                revision = %obligation.revision(),
            )
        )
    )]
    fn install_wake(
        &mut self, node: usize, key: WakeKey, deadline: Instant,
        obligation: Obligation,
    ) {
        let (_, replaced) = self.wakes.install(node, key, deadline, obligation);
        if let Some(scheduled) = replaced {
            #[cfg(feature = "tracing")]
            tracing::event!(
                name: "wake.replaced",
                tracing::Level::DEBUG,
                node,
                key = ?scheduled.key,
            );
            let settlement = scheduled.authority.clear(&mut self.revisions);
            self.record_settlement(settlement);
        }
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug",
            name = "wake.clear",
            skip_all,
            fields(node, key = ?key)
        )
    )]
    fn clear_current_wake(&mut self, node: usize, key: WakeKey) {
        let Some(scheduled) = self.wakes.clear(node, key) else {
            return;
        };
        let settlement = scheduled.authority.clear(&mut self.revisions);
        self.record_settlement(settlement);
    }

    fn prune_transport(&mut self, revision: RevisionId) {
        self.prune_transport_with(revision, true);
    }

    fn prune_transport_all(&mut self, revision: RevisionId) {
        self.prune_transport_with(revision, false);
    }

    fn prune_transport_with(
        &mut self, revision: RevisionId, preserve_abort: bool,
    ) {
        let Pruned {
            released_lanes,
            released_outputs,
            obligations,
        } = if preserve_abort {
            self.transport.prune(revision)
        } else {
            self.transport.prune_all(revision)
        };
        for route in released_lanes {
            self.release_lane(route.node, route.lane);
        }
        for source in released_outputs {
            self.schedule(source);
        }
        for obligation in obligations {
            let settlement = self.revisions.retire(obligation);
            self.record_settlement(settlement);
        }
        let obligations = if preserve_abort {
            self.progress_branches.prune(revision)
        } else {
            self.progress_branches.prune_all(revision)
        };
        for obligation in obligations {
            let settlement = self.revisions.retire(obligation);
            self.record_settlement(settlement);
        }
        for node in 0..self.graph.len() {
            self.schedule(node);
        }
    }

    fn prune_wakes(&mut self, revision: RevisionId) {
        let revisions = &mut self.revisions;
        let mut settlement = None;
        self.wakes.prune(revision, |scheduled| {
            let current = scheduled.authority.clear(revisions);
            settlement = settlement.or(current);
        });
        self.record_settlement(settlement);
        for node in 0..self.graph.len() {
            self.schedule(node);
        }
    }
}
