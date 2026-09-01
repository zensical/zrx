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

//! Attachment and fair execution of isolated plans.

use crossbeam::channel::Select;
use std::any::TypeId;
use std::collections::VecDeque;
use std::fmt::{self, Display};
use std::ops::Range;
use std::time::{Duration, Instant};
use thiserror::Error as ThisError;

use zrx_executor::Strategy;
use zrx_executor::strategy::{Immediate, WorkSharing};
use zrx_store::stash::{Slab, Slot};

pub mod action;
mod event;
pub mod plan;
mod runtime;
mod session;

pub use event::Change;
pub use plan::Plan;
pub use runtime::{Egress, EgressIter};
pub use session::{Error as SessionError, Session, Writer};

use self::action::{Instrumentation, Outcomes};
use self::plan::InputId;
use runtime::{Backend, Retiring, Runtime};
use session::Inboxes;

/// Implements value for the given concrete types.
macro_rules! impl_values {
    ($($T:ty),+ $(,)?) => {
        $(impl Value for $T {})+
    };
}

/// Implements value for tuples.
macro_rules! impl_value_for_tuple {
    ($($T:ident),+) => {
        impl<$($T),+> Value for ($($T,)+)
        where
            $($T: Value),+
        {
        }
    };
}

impl_values!((), bool, char);

impl_values!(u8, u16, u32, u64, u128, usize);

impl_values!(i8, i16, i32, i64, i128, isize);

impl_values!(f32, f64);

impl_values!(&'static str, String);

impl_values!(Duration, Instant);

impl_value_for_tuple!(T1);

impl_value_for_tuple!(T1, T2);

impl_value_for_tuple!(T1, T2, T3);

impl_value_for_tuple!(T1, T2, T3, T4);

impl_value_for_tuple!(T1, T2, T3, T4, T5);

impl_value_for_tuple!(T1, T2, T3, T4, T5, T6);

impl_value_for_tuple!(T1, T2, T3, T4, T5, T6, T7);

impl_value_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);

// A finite private quantum is the fairness invariant. Runtime measurement may
// adapt this bootstrap later without changing the attached-plan protocol.
const INGRESS_QUANTUM: u8 = 16;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Value that can be transported between scheduler threads.
///
/// Values are cloneable so a shared segment lease can yield independent owned
/// values. Transport still moves values whenever it can recover ownership.
///
/// This trait deliberately has no blanket implementation. In particular,
/// [`Result`] must remain distinct from ordinary values so callback return
/// values can be normalized into action failures without type ambiguity.
pub trait Value: Clone + Send + Sync + 'static {}

// ----------------------------------------------------------------------------

/// Complete opaque identity transported with one scheduler value.
pub trait Id: Value {}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Terminal outcome of one scheduler revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Settlement {
    /// The sealed revision physically drained, including reported failures.
    Complete(RevisionId),
    /// The aborted revision drained all retained work.
    Aborted(RevisionId),
}

// ----------------------------------------------------------------------------

/// Ownership-preserving admission under bounded capacity.
#[must_use]
pub enum Admit<A, T> {
    /// The scheduler accepted ownership and produced the operation result.
    Accepted(A),
    /// Capacity is currently unavailable and ownership remains with the caller.
    Full(T),
}

// ----------------------------------------------------------------------------

/// Invalid operation on an attached scheduler plan.
#[derive(Clone, Debug, PartialEq, Eq, ThisError)]
pub enum Error {
    /// The plan identity is stale or unknown.
    #[error("plan {0:?} is stale or unknown")]
    Plan(PlanId),
    /// A typed external session rejected the operation.
    #[error(transparent)]
    Session(#[from] SessionError),
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Candidate {
    Attached(PlanId),
    Retiring(Slot),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Generational identity of one plan-local scheduler revision.
///
/// Every data invocation, wake, progress event, and settlement derived from
/// one admitted source revision carries this identity. It is scheduler
/// provenance, not an external provider correlation identifier.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct RevisionId(Slot);

// ----------------------------------------------------------------------------

/// Observable records produced by one node invocation.
#[derive(Debug)]
pub struct InvocationReport {
    /// Revision whose homogeneous input authority ran.
    pub revision: RevisionId,
    /// Installed node that ran.
    pub node: usize,
    /// Sparse ordinary failures from this invocation.
    pub outcomes: Outcomes,
    /// Ordered diagnostics and author annotations.
    pub instrumentation: Instrumentation,
}

// ----------------------------------------------------------------------------

/// One scheduler-owned current action error.
#[derive(Clone, Debug)]
pub struct CurrentError<I> {
    node: usize,
    domain: TypeId,
    key: I,
    error: action::Error,
}

// ----------------------------------------------------------------------------

/// Owning consequences produced by one scheduler tick.
#[derive(Debug, Default)]
#[must_use]
pub struct Report {
    settlements: Vec<Settlement>,
    invocations: Vec<InvocationReport>,
}

// ----------------------------------------------------------------------------

/// Scheduler-local generational identity of one attached plan.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlanId(Slot);

// ----------------------------------------------------------------------------

/// One immediately available fair scheduler tick.
#[must_use]
pub struct Tick {
    plan: PlanId,
    progressed: bool,
    report: Report,
}

// ----------------------------------------------------------------------------

/// Borrow-free classification of scheduler-wide future readiness.
#[derive(Debug)]
pub struct Readiness {
    operations: Range<usize>,
    pending: bool,
    deadline: Option<Instant>,
}

// ----------------------------------------------------------------------------

struct Retirement<I, S>
where
    I: Id,
    S: Strategy,
{
    plan: PlanId,
    runtime: Retiring<I, S>,
}

// ----------------------------------------------------------------------------

struct AttachedPlan<I, S>
where
    I: Id,
    S: Strategy,
{
    runtime: Runtime<I, S>,
    inboxes: Inboxes<I>,
    ingress: u8,
}

// ----------------------------------------------------------------------------

/// Checked mutable scope over one currently attached plan.
#[must_use]
pub struct Attachment<'a, I, S>
where
    I: Id,
    S: Strategy,
{
    scheduler: &'a mut Scheduler<I, S>,
    plan: PlanId,
}

// ----------------------------------------------------------------------------

/// Shared execution owner for independently attached immutable plans.
pub struct Scheduler<I, S = WorkSharing>
where
    I: Id,
    S: Strategy,
{
    plans: Slab<AttachedPlan<I, S>>,
    retirements: Slab<Retirement<I, S>>,
    order: VecDeque<Candidate>,
    backend: Backend<S>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

#[cfg(test)]
impl RevisionId {
    pub(crate) fn test(index: usize) -> Self {
        Self(Slot::from_parts(index, 0))
    }
}

// ----------------------------------------------------------------------------

impl<I> CurrentError<I> {
    /// Returns the installed node whose evaluation is currently rejected.
    #[must_use]
    pub const fn node(&self) -> usize {
        self.node
    }

    /// Returns the operator-defined semantic evaluation key.
    #[must_use]
    pub const fn key(&self) -> &I {
        &self.key
    }

    /// Returns the latest error for this evaluation identity.
    #[must_use]
    pub const fn error(&self) -> &action::Error {
        &self.error
    }
}

// ----------------------------------------------------------------------------

impl Report {
    /// Returns revision settlements in occurrence order.
    #[must_use]
    pub fn settlements(&self) -> &[Settlement] {
        &self.settlements
    }

    /// Returns nonempty invocation reports in reconciliation order.
    #[must_use]
    pub fn invocations(&self) -> &[InvocationReport] {
        &self.invocations
    }

    /// Returns whether the report contains no observable consequences.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.settlements.is_empty() && self.invocations.is_empty()
    }

    /// Appends another owning report in observation order.
    pub fn append(&mut self, mut other: Self) {
        if self.is_empty() {
            *self = other;
            return;
        }
        self.settlements.append(&mut other.settlements);
        self.invocations.append(&mut other.invocations);
    }
}

// ----------------------------------------------------------------------------

impl Readiness {
    /// Returns whether a selected operation belongs to the scheduler.
    #[must_use]
    pub fn contains(&self, operation: usize) -> bool {
        self.operations.contains(&operation)
    }

    /// Returns whether an accepted worker invocation can still complete.
    #[must_use]
    pub const fn pending(&self) -> bool {
        self.pending
    }

    /// Returns the earliest wake deadline captured during registration.
    #[must_use]
    pub const fn deadline(&self) -> Option<Instant> {
        self.deadline
    }
}

// ----------------------------------------------------------------------------

impl Tick {
    /// Returns the plan that produced this tick.
    #[must_use]
    pub const fn plan(&self) -> PlanId {
        self.plan
    }

    /// Returns whether the selected plan made physical progress.
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

impl<I, S> AttachedPlan<I, S>
where
    I: Id,
    S: Strategy,
{
    fn tick(&mut self) -> runtime::Tick {
        if self.ingress != 0 && self.inboxes.admit(&mut self.runtime) {
            self.ingress -= 1;
            return runtime::Tick::admitted();
        }
        let tick = self.runtime.tick();
        if tick.available() {
            self.ingress = INGRESS_QUANTUM;
            return tick;
        }
        if self.inboxes.admit(&mut self.runtime) {
            self.ingress = INGRESS_QUANTUM - 1;
            return runtime::Tick::admitted();
        }
        self.ingress = INGRESS_QUANTUM;
        tick
    }
}

// ----------------------------------------------------------------------------

impl<I, S> Attachment<'_, I, S>
where
    I: Id,
    S: Strategy,
{
    /// Creates the sole typed transferable session for one installed input.
    ///
    /// Individual writer changes are accumulated into bounded internal
    /// batches and sent through a channel sized from the execution backend.
    /// The receiving runtime admits each batch as one ordinary input segment.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown input, mismatched value type, or second
    /// session for the same authoritative input.
    pub fn session<V>(&mut self, input: InputId) -> Result<Session<I, V>, Error>
    where
        V: Value,
    {
        let attached = self.attached();
        let port = attached
            .runtime
            .input_port(input)
            .ok_or(SessionError::Input(input))?;
        Ok(attached.inboxes.install(input, port)?)
    }

    /// Accepts the next fairly selected committed output.
    pub fn egress(&mut self) -> Option<Egress<I>> {
        self.attached().runtime.egress()
    }

    /// Returns the current errors owned by this attached plan.
    ///
    /// # Panics
    ///
    /// Panics if the scoped plan is removed while the attachment exists,
    /// which would violate its exclusive borrowing invariant.
    #[must_use]
    pub fn errors(&self) -> &[CurrentError<I>] {
        self.scheduler
            .plans
            .get(self.plan.0)
            .expect("scoped plan remains attached")
            .runtime
            .errors()
    }

    fn attached(&mut self) -> &mut AttachedPlan<I, S> {
        self.scheduler
            .plans
            .get_mut(self.plan.0)
            .expect("scoped plan remains attached")
    }

    /// Immediately fences this attachment and starts asynchronous retirement.
    ///
    /// Only work already committed may subsequently run or reconcile.
    pub fn detach(self) {
        self.scheduler.detach(self.plan);
    }
}

// ----------------------------------------------------------------------------

impl<I, S> Scheduler<I, S>
where
    I: Id,
    S: Strategy,
{
    /// Creates a scheduler backed by the given execution strategy.
    #[must_use]
    pub fn new(strategy: S) -> Self {
        Self {
            plans: Slab::new(),
            retirements: Slab::new(),
            order: VecDeque::new(),
            backend: Backend::worker(strategy),
        }
    }

    /// Attaches one complete plan with fresh, isolated runtime state.
    #[must_use]
    pub fn attach(&mut self, plan: Plan<I>) -> PlanId {
        let runtime = Runtime::install(plan, self.backend.clone());
        let id = PlanId(self.plans.insert(AttachedPlan {
            runtime,
            inboxes: Inboxes::default(),
            ingress: INGRESS_QUANTUM,
        }));
        self.order.push_back(Candidate::Attached(id));
        id
    }

    /// Returns a checked mutable scope over one current plan.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Plan`] if the plan identity is stale.
    pub fn attachment(
        &mut self, plan: PlanId,
    ) -> Result<Attachment<'_, I, S>, Error> {
        self.plans.get(plan.0).ok_or(Error::Plan(plan))?;
        Ok(Attachment { scheduler: self, plan })
    }

    /// Returns the current errors owned by an attached plan.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Plan`] if the plan identity is stale.
    pub fn errors(&self, plan: PlanId) -> Result<&[CurrentError<I>], Error> {
        self.plans
            .get(plan.0)
            .map(|attached| attached.runtime.errors())
            .ok_or(Error::Plan(plan))
    }

    /// Registers all scheduler-owned operations and the earliest wake deadline.
    ///
    /// # Panics
    ///
    /// Panics if the selector does not assign contiguous indexes to
    /// consecutive registrations.
    pub fn register<'a>(&'a self, select: &mut Select<'a>) -> Readiness {
        fn include(range: &mut Option<Range<usize>>, operation: usize) {
            let range = range.get_or_insert(operation..operation);
            assert_eq!(
                range.end, operation,
                "scheduler registrations must remain contiguous"
            );
            range.end += 1;
        }

        let mut operations = None::<Range<usize>>;
        let mut pending = false;
        let mut deadline = None;
        for (_, resident) in &self.plans {
            resident.inboxes.register(select, |operation| {
                include(&mut operations, operation);
            });
            let readiness = resident.runtime.register(select);
            if let Some(completion) = readiness.completion() {
                include(&mut operations, completion);
            }
            pending |= readiness.pending();
            if let Some(candidate) = readiness.deadline() {
                deadline = Some(deadline.map_or(candidate, |current| {
                    std::cmp::min(current, candidate)
                }));
            }
        }
        for (_, retirement) in &self.retirements {
            let readiness = retirement.runtime.register(select);
            if let Some(completion) = readiness.completion() {
                include(&mut operations, completion);
            }
            pending |= readiness.pending();
            if let Some(candidate) = readiness.deadline() {
                deadline = Some(deadline.map_or(candidate, |current| {
                    std::cmp::min(current, candidate)
                }));
            }
        }
        Readiness {
            operations: operations.unwrap_or(0..0),
            pending,
            deadline,
        }
    }

    /// Runs one immediately available scheduler tick in round-robin order.
    ///
    /// Returns `None` when no attached or retiring runtime can currently make
    /// progress or report a consequence. A worker may still be running.
    ///
    /// # Panics
    ///
    /// Resumes an action panic and panics if scheduler ownership is internally
    /// inconsistent.
    pub fn tick(&mut self) -> Option<Tick> {
        let candidates = self.order.len();
        for _ in 0..candidates {
            let candidate = self
                .order
                .pop_front()
                .expect("candidate count came from the scheduling queue");
            match candidate {
                Candidate::Attached(id) => {
                    let Some(runtime) = self.plans.get_mut(id.0) else {
                        continue;
                    };
                    let tick = runtime.tick();
                    self.order.push_back(candidate);
                    let progressed = tick.progressed();
                    let report = tick.into_report();
                    if progressed || !report.is_empty() {
                        return Some(Tick { plan: id, progressed, report });
                    }
                }
                Candidate::Retiring(id) => {
                    let Some(retirement) = self.retirements.get_mut(id) else {
                        continue;
                    };
                    let plan = retirement.plan;
                    let progressed = retirement.runtime.tick();
                    if retirement.runtime.is_complete() {
                        let retirement = self
                            .retirements
                            .remove(id)
                            .expect("current retirement remains attached");
                        let Ok(report) = retirement.runtime.try_finish() else {
                            panic!("completed retirement retained a revision")
                        };
                        return Some(Tick { plan, progressed, report });
                    }
                    self.order.push_back(candidate);
                    if progressed {
                        return Some(Tick {
                            plan,
                            progressed: true,
                            report: Report::default(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Immediately fences one attachment and starts asynchronous retirement.
    ///
    /// The plan identity becomes stale before this method returns. Only work
    /// already committed may subsequently run or reconcile.
    fn detach(&mut self, id: PlanId) {
        let attached = self
            .plans
            .remove(id.0)
            .expect("scoped plan remains attached");
        self.order.retain(
            |candidate| !matches!(candidate, Candidate::Attached(current) if *current == id),
        );
        let retirement = self.retirements.insert(Retirement {
            plan: id,
            runtime: attached.runtime.begin_retirement(),
        });
        self.order.push_back(Candidate::Retiring(retirement));
    }
}

impl<I> Scheduler<I, Immediate>
where
    I: Id,
{
    /// Creates a scheduler that executes work directly on its orchestration
    /// thread without accepting an executor strategy.
    #[must_use]
    pub fn inline() -> Self {
        Self {
            plans: Slab::new(),
            retirements: Slab::new(),
            order: VecDeque::new(),
            backend: Backend::inline(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> Value for Option<T> where T: Value {}

// ----------------------------------------------------------------------------

impl<T> Value for Vec<T> where T: Value {}

// ----------------------------------------------------------------------------

impl<T> Id for T where T: Value {}

// ----------------------------------------------------------------------------

impl Display for RevisionId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

// ----------------------------------------------------------------------------

impl AsRef<Slot> for PlanId {
    fn as_ref(&self) -> &Slot {
        &self.0
    }
}

// ----------------------------------------------------------------------------

impl<I, S> Default for Scheduler<I, S>
where
    I: Id,
    S: Strategy + Default,
{
    fn default() -> Self {
        Self::new(S::default())
    }
}
