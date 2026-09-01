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

//! Persistent type-erased action adaptation at the batch boundary.

use std::num::NonZeroUsize;

use crate::scheduler::{Id, RevisionId, Value};

use super::control::{Event, Events};
use super::{
    Action, Concurrency, Context, EvaluationChanges, InputLayout, Inputs,
    Instrumentation, Outcomes, Output, Port, Segment, WakeRequest,
};

const MAX_INPUTS: usize = 8;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

trait Execute<I>: Send
where
    I: Id,
{
    fn replica(&self) -> Box<dyn Execute<I>>;

    fn run(
        &mut self, revision: RevisionId, inputs: InputSegments<I>,
        event: Option<Event>, connected: bool,
    ) -> (
        Option<Segment<I>>,
        Outcomes,
        EvaluationChanges<I>,
        Vec<WakeRequest>,
        Instrumentation,
    );
}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Clone)]
enum Ports {
    Fixed(Box<[Port]>),
    Repeated { port: Port, ports: Vec<Port> },
}

// ----------------------------------------------------------------------------

// The large inline variant is deliberate: ordinary actions must not allocate
// their lane-position table merely to make wide variadic actions smaller.
#[allow(clippy::large_enum_variant)]
enum InputStorage<I>
where
    I: Id,
{
    Inline {
        segments: [Option<Segment<I>>; MAX_INPUTS],
        len: usize,
    },
    Heap(Vec<Option<Segment<I>>>),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Inline ownership of one action invocation's independent input lanes.
pub(crate) struct InputSegments<I>
where
    I: Id,
{
    storage: InputStorage<I>,
}

// ----------------------------------------------------------------------------

struct Adapter<A, R> {
    action: A,
    replica: Option<R>,
}

// ----------------------------------------------------------------------------

struct Forward;

// ----------------------------------------------------------------------------

/// Persistent erased action state.
pub struct Job<I>
where
    I: Id,
{
    action: Box<dyn Execute<I>>,
    inputs: Ports,
    output: Port,
    progress: bool,
    max_parallelism: Option<NonZeroUsize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Ports {
    fn as_slice(&self) -> &[Port] {
        match self {
            Self::Fixed(ports) => ports,
            Self::Repeated { ports, .. } => ports,
        }
    }

    fn resolve(&mut self, len: usize) {
        let Self::Repeated { port, ports } = self else {
            return;
        };
        ports.resize(len, *port);
    }
}

// ----------------------------------------------------------------------------

impl<I> InputSegments<I>
where
    I: Id,
{
    pub(crate) fn collect(
        inputs: impl IntoIterator<Item = Option<Segment<I>>>, expected: usize,
    ) -> Self {
        let mut inputs = inputs.into_iter();
        let storage = if expected <= MAX_INPUTS {
            let mut segments = std::array::from_fn(|_| None);
            for segment in &mut segments[..expected] {
                *segment = inputs
                    .next()
                    .expect("plan supplied every action input position");
            }
            InputStorage::Inline { segments, len: expected }
        } else {
            let segments = inputs.by_ref().take(expected).collect::<Vec<_>>();
            assert_eq!(
                segments.len(),
                expected,
                "plan supplied every action input position"
            );
            InputStorage::Heap(segments)
        };
        debug_assert!(
            inputs.next().is_none(),
            "plan supplied too many action input positions"
        );
        Self { storage }
    }

    pub(crate) fn empty(len: usize) -> Self {
        let storage = if len <= MAX_INPUTS {
            InputStorage::Inline {
                segments: std::array::from_fn(|_| None),
                len,
            }
        } else {
            InputStorage::Heap(
                std::iter::repeat_with(|| None).take(len).collect(),
            )
        };
        Self { storage }
    }

    pub(crate) fn as_slice(&self) -> &[Option<Segment<I>>] {
        match &self.storage {
            InputStorage::Inline { segments, len } => &segments[..*len],
            InputStorage::Heap(segments) => segments,
        }
    }

    pub(crate) fn as_mut_slice(&mut self) -> &mut [Option<Segment<I>>] {
        match &mut self.storage {
            InputStorage::Inline { segments, len } => &mut segments[..*len],
            InputStorage::Heap(segments) => segments,
        }
    }

    fn into_single(mut self) -> Option<Segment<I>> {
        assert_eq!(
            self.as_slice().len(),
            1,
            "forwarding job requires exactly one input lane"
        );
        self.as_mut_slice()[0].take()
    }
}

// ----------------------------------------------------------------------------

impl<I> Job<I>
where
    I: Id,
{
    /// Creates a serial job that forwards one same-typed input segment.
    ///
    /// Forwarding preserves the segment's allocation, items, and ordering.
    /// Event-only invocations are consumed without output.
    #[must_use]
    pub fn forward<V>() -> Self
    where
        V: Value,
    {
        let port = Port::of::<I, V>();
        Self {
            action: Box::new(Forward),
            inputs: Ports::Fixed(Box::new([port])),
            output: port,
            progress: false,
            max_parallelism: Some(NonZeroUsize::MIN),
        }
    }

    /// Erases one typed action exactly once.
    #[must_use]
    pub fn new<A>(action: A) -> Self
    where
        A: Action<I>,
    {
        let Concurrency { maximum, replica } = action.concurrency();
        Self::install(action, replica, maximum)
    }

    /// Erases one typed action with an externally supplied replica factory.
    #[must_use]
    pub fn replicated<A, R>(
        action: A, maximum: Option<NonZeroUsize>, replica: R,
    ) -> Self
    where
        A: Action<I>,
        R: Fn(&A) -> A + Clone + Send + 'static,
    {
        Self::install(action, Some(replica), maximum)
    }

    fn install<A, R>(
        action: A, replica: Option<R>, max_parallelism: Option<NonZeroUsize>,
    ) -> Self
    where
        A: Action<I>,
        R: Fn(&A) -> A + Clone + Send + 'static,
    {
        let inputs = match A::Inputs::layout() {
            InputLayout::Fixed(inputs) => {
                Ports::Fixed(inputs.into_boxed_slice())
            }
            InputLayout::Repeated(port) => {
                Ports::Repeated { port, ports: Vec::new() }
            }
        };
        Self {
            action: Box::new(Adapter { action, replica }),
            inputs,
            output: Port::of::<I, A::Output>(),
            progress: false,
            max_parallelism,
        }
    }

    /// Marks this installed job as a revision-progress subscriber.
    ///
    /// Progress consumption belongs to graph construction rather than the
    /// action type: the same action can be installed with or without progress
    /// in different plans. Plan construction derives one shared overlay from
    /// every external input that can reach this job.
    #[must_use]
    pub fn with_progress(mut self) -> Self {
        self.progress = true;
        self
    }

    pub(in crate::scheduler) fn inputs(&self) -> &[Port] {
        self.inputs.as_slice()
    }

    /// Returns this job's exact output port.
    #[must_use]
    pub const fn output(&self) -> Port {
        self.output
    }

    /// Returns whether this job consumes revision progress events.
    #[must_use]
    pub const fn requires_progress(&self) -> bool {
        self.progress
    }

    pub(in crate::scheduler) fn parallelism(&self, capacity: usize) -> usize {
        assert!(capacity != 0, "job parallelism capacity must be non-zero");
        self.max_parallelism
            .map_or(capacity, |limit| limit.get().min(capacity))
    }

    pub(in crate::scheduler) fn replica(&self) -> Self {
        Self {
            action: self.action.replica(),
            inputs: self.inputs.clone(),
            output: self.output,
            progress: self.progress,
            max_parallelism: self.max_parallelism,
        }
    }

    pub(in crate::scheduler) fn run(
        &mut self, revision: RevisionId, inputs: InputSegments<I>,
        event: Option<Event>, connected: bool,
    ) -> (
        Option<Segment<I>>,
        Outcomes,
        EvaluationChanges<I>,
        Vec<WakeRequest>,
        Instrumentation,
    ) {
        debug_assert_eq!(inputs.as_slice().len(), self.inputs().len());
        self.action.run(revision, inputs, event, connected)
    }

    pub(in crate::scheduler) fn resolve_inputs(&mut self, len: usize) {
        self.inputs.resolve(len);
    }

    pub(in crate::scheduler) fn is_variadic(&self) -> bool {
        matches!(&self.inputs, Ports::Repeated { .. })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Execute<I> for Forward
where
    I: Id,
{
    fn replica(&self) -> Box<dyn Execute<I>> {
        panic!("serial forwarding job was asked to create a replica")
    }

    fn run(
        &mut self, _: RevisionId, inputs: InputSegments<I>,
        event: Option<Event>, connected: bool,
    ) -> (
        Option<Segment<I>>,
        Outcomes,
        EvaluationChanges<I>,
        Vec<WakeRequest>,
        Instrumentation,
    ) {
        let input = inputs.into_single();
        let output = (event.is_none() && connected).then_some(input).flatten();
        (
            output,
            Outcomes::default(),
            EvaluationChanges::default(),
            Vec::new(),
            Instrumentation::default(),
        )
    }
}

// ----------------------------------------------------------------------------

impl<I, A, R> Execute<I> for Adapter<A, R>
where
    I: Id,
    A: Action<I>,
    R: Fn(&A) -> A + Clone + Send + 'static,
{
    fn replica(&self) -> Box<dyn Execute<I>> {
        let replica = self
            .replica
            .as_ref()
            .expect("serial action was asked to create a replica");
        Box::new(Self {
            action: replica(&self.action),
            replica: self.replica.clone(),
        })
    }

    fn run(
        &mut self, revision: RevisionId, mut inputs: InputSegments<I>,
        event: Option<Event>, connected: bool,
    ) -> (
        Option<Segment<I>>,
        Outcomes,
        EvaluationChanges<I>,
        Vec<WakeRequest>,
        Instrumentation,
    ) {
        let mut output = Output::new(connected);
        {
            let context = Context {
                revision,
                // SAFETY: plan installation fixes every lane port and ingress
                // validates external segments before transport admission.
                inputs: unsafe { A::Inputs::view(inputs.as_mut_slice()) },
                output: &mut output,
                events: event.map_or_else(Events::empty, Events::one),
            };
            self.action.execute(context);
        }
        assert!(
            inputs
                .as_slice()
                .iter()
                .filter_map(Option::as_ref)
                .all(Segment::is_empty),
            "action returned with unread input"
        );
        let (items, outcomes, evaluations, wakes, instrumentation) =
            output.seal();
        let output = if connected && !items.is_empty() {
            Some(Segment::new(items))
        } else {
            None
        };
        (output, outcomes, evaluations, wakes, instrumentation)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Instant;

    use super::Job;
    use crate::scheduler::Change;
    use crate::scheduler::action::InputSegments;
    use crate::scheduler::action::control::Event;
    use crate::scheduler::action::{Action, Context, Segment, WakeKey};
    use crate::scheduler::{RevisionId, Value};

    struct EightInputs;

    struct VariadicInputs;

    struct MoveOnly(String);

    impl Clone for MoveOnly {
        fn clone(&self) -> Self {
            panic!("forwarded test value was cloned")
        }
    }

    struct LazyReplica(Arc<AtomicUsize>);

    impl Value for MoveOnly {}

    impl Action<u64> for LazyReplica {
        type Inputs = (u64,);
        type Output = ();

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context {
                inputs: input, output, events, ..
            } = context;
            input.for_each(output, |_, _| Ok(()));
            events.for_each(output, |_, _| Ok(()));
        }
    }

    impl Action<u64> for EightInputs {
        type Inputs = (u8, u16, u32, u64, i8, i16, i32, i64);
        type Output = u8;

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context {
                inputs: (t1, t2, t3, t4, t5, t6, t7, t8),
                output,
                events,
                ..
            } = context;
            macro_rules! drain {
                ($input:ident, $lane:literal) => {
                    $input.for_each(output, |change, emit| {
                        if let Change::Insert(key, _) = change {
                            emit.insert(key, $lane);
                        }
                        Ok(())
                    });
                };
            }
            drain!(t1, 1);
            drain!(t2, 2);
            drain!(t3, 3);
            drain!(t4, 4);
            drain!(t5, 5);
            drain!(t6, 6);
            drain!(t7, 7);
            drain!(t8, 8);
            events.for_each(output, |_, _| Ok(()));
        }
    }

    impl Action<u64> for VariadicInputs {
        type Inputs = Vec<u64>;
        type Output = usize;

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context { inputs, output, events, .. } = context;
            for (lane, input) in inputs.enumerate() {
                input.for_each(output, |change, emit| {
                    if let Change::Insert(key, _) = change {
                        emit.insert(key, lane);
                    }
                    Ok(())
                });
            }
            events.for_each(output, |_, _| Ok(()));
        }
    }

    fn input<V>(key: u64, value: V) -> Segment<u64>
    where
        V: Value,
    {
        Segment::new(vec![Change::Insert(key, value)])
    }

    #[test]
    fn maximum_input_arity_executes_every_independent_lane() {
        let mut job = Job::new(EightInputs);
        let inputs = InputSegments::collect(
            [
                Some(input(1, 1_u8)),
                Some(input(2, 2_u16)),
                Some(input(3, 3_u32)),
                Some(input(4, 4_u64)),
                Some(input(5, 5_i8)),
                Some(input(6, 6_i16)),
                Some(input(7, 7_i32)),
                Some(input(8, 8_i64)),
            ],
            8,
        );

        let (output, outcomes, _, wakes, instrumentation) =
            job.run(RevisionId::test(0), inputs, None, true);
        assert!(outcomes.is_empty());
        assert!(instrumentation.is_empty());
        assert!(wakes.is_empty());
        let mut lanes = Vec::new();
        output
            .expect("connected action emitted output")
            .drain::<u8>(|change| {
                let Change::Insert(_, lane) = change else {
                    panic!("maximum-arity action emitted a removal");
                };
                lanes.push(*lane.as_ref());
            });
        assert_eq!(lanes, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn homogeneous_inputs_spill_beyond_inline_lane_storage() {
        let mut job = Job::new(VariadicInputs);
        job.resolve_inputs(9);
        let inputs = InputSegments::collect(
            (0..9).map(|lane| Some(input(lane, lane))),
            9,
        );

        let (output, outcomes, _, wakes, instrumentation) =
            job.run(RevisionId::test(0), inputs, None, true);
        assert!(outcomes.is_empty());
        assert!(instrumentation.is_empty());
        assert!(wakes.is_empty());
        let mut lanes = Vec::new();
        output
            .expect("connected action emitted output")
            .drain::<usize>(|change| {
                let Change::Insert(_, lane) = change else {
                    panic!("variadic action emitted a removal");
                };
                lanes.push(*lane.as_ref());
            });
        assert_eq!(lanes, (0..9).collect::<Vec<_>>());
    }

    #[test]
    fn external_concurrency_is_installed_without_action_ownership() {
        let replicas = Arc::new(AtomicUsize::new(0));
        let job = Job::<u64>::replicated(
            LazyReplica(Arc::clone(&replicas)),
            None,
            |action: &LazyReplica| {
                action.0.fetch_add(1, Ordering::Relaxed);
                LazyReplica(Arc::clone(&action.0))
            },
        )
        .with_progress();

        assert_eq!(replicas.load(Ordering::Relaxed), 0);
        assert_eq!(job.parallelism(4), 4);
        assert!(job.requires_progress());

        let replica = job.replica();
        assert_eq!(replicas.load(Ordering::Relaxed), 1);
        assert!(replica.requires_progress());
        let serial = Job::new(EightInputs);
        assert_eq!(serial.parallelism(4), 1);
        assert!(!serial.requires_progress());
    }

    #[test]
    fn forwarding_reuses_one_segment_across_repeated_data_invocations() {
        let mut job = Job::<u64>::forward::<MoveOnly>();
        assert_eq!(job.parallelism(8), 1);

        for key in [1, 2] {
            let inputs = InputSegments::collect(
                [Some(Segment::new(vec![
                    Change::Insert(key, MoveOnly(key.to_string())),
                    Change::Remove(key + 10),
                ]))],
                1,
            );

            let (output, outcomes, _, wakes, instrumentation) = job.run(
                RevisionId::test(usize::try_from(key).unwrap()),
                inputs,
                None,
                true,
            );
            assert!(outcomes.is_empty());
            assert!(wakes.is_empty());
            assert!(instrumentation.is_empty());

            let mut changes = Vec::new();
            output
                .expect("forwarded output")
                .drain::<MoveOnly>(|change| match change {
                    Change::Insert(key, value) => {
                        changes.push((key, Some(value.as_ref().0.clone())));
                    }
                    Change::Remove(key) => changes.push((key, None)),
                });
            assert_eq!(
                changes,
                [(key, Some(key.to_string())), (key + 10, None),]
            );
        }
    }

    #[test]
    fn forwarding_drops_disconnected_data_and_event_only_invocations() {
        let mut job = Job::<u64>::forward::<u64>();
        let inputs = InputSegments::collect([Some(input(1, 1_u64))], 1);
        let (output, outcomes, _, wakes, instrumentation) =
            job.run(RevisionId::test(0), inputs, None, false);
        assert!(output.is_none());
        assert!(outcomes.is_empty());
        assert!(wakes.is_empty());
        assert!(instrumentation.is_empty());

        let event = Event::Wake {
            key: WakeKey::new(1),
            deadline: Instant::now(),
        };
        let (output, outcomes, _, wakes, instrumentation) = job.run(
            RevisionId::test(0),
            InputSegments::empty(1),
            Some(event),
            true,
        );
        assert!(output.is_none());
        assert!(outcomes.is_empty());
        assert!(wakes.is_empty());
        assert!(instrumentation.is_empty());
    }
}
