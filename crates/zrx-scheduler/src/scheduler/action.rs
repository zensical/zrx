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

//! Typed actions over whole-batch transport segments.

use std::any::{TypeId, type_name};
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::slice;
use std::time::Instant;

use crate::scheduler::event::Change;
use crate::scheduler::{Id, RevisionId, Value};

pub mod control;
pub mod error;
mod inputs {
    pub trait Sealed {}
}
mod instrumentation;
mod job;
mod outcome;
pub mod replication;
mod segment;

use control::Events;
pub use error::{Error, Result};
pub use instrumentation::{
    Annotation, Instrumentation, Measurement, Record, Recorder,
};
pub(crate) use job::InputSegments;
pub use job::Job;
pub use outcome::Outcomes;
pub(crate) use outcome::{
    DefaultEvaluation, EvaluationChange, EvaluationChanges,
};
pub(crate) use segment::Segment;

macro_rules! impl_inputs_for_tuple {
    ($($input:ident),+ $(,)?) => {
        impl<$($input),+> inputs::Sealed for ($($input,)+) {}

        impl<I, $($input),+> Inputs<I> for ($($input,)+)
        where
            I: Id,
            $($input: Value),+
        {
            #[allow(unused_parens)]
            type View<'a> = ($(Input<'a, I, $input>),+)
            where
                I: 'a,
                Self: 'a;

            fn layout() -> InputLayout {
                InputLayout::Fixed(vec![$(Port::of::<I, $input>()),+])
            }

            #[allow(non_snake_case)]
            unsafe fn view<'a>(
                segments: &'a mut [Option<Segment<I>>],
            ) -> Self::View<'a>
            where
                I: 'a,
                Self: 'a,
            {
                let mut segments = segments.iter_mut();
                #[allow(unused_parens)]
                ($(
                    Input::<I, $input>::new(
                        segments
                            .next()
                            .expect("validated input count")
                            .as_mut()
                    )
                ),+)
            }
        }
    };
}

impl_inputs_for_tuple!(T1);

impl_inputs_for_tuple!(T1, T2);

impl_inputs_for_tuple!(T1, T2, T3);

impl_inputs_for_tuple!(T1, T2, T3, T4);

impl_inputs_for_tuple!(T1, T2, T3, T4, T5);

impl_inputs_for_tuple!(T1, T2, T3, T4, T5, T6);

impl_inputs_for_tuple!(T1, T2, T3, T4, T5, T6, T7);

impl_inputs_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Maps fixed tuples or homogeneous vectors to scoped segment drivers.
pub trait Inputs<I>: inputs::Sealed + Send + 'static
where
    I: Id,
{
    /// Scoped typed views supplied to one action invocation.
    type View<'a>
    where
        I: 'a,
        Self: 'a;

    /// Returns this input shape's typed port layout.
    fn layout() -> InputLayout;

    /// Binds prevalidated erased segments to typed views.
    ///
    /// # Safety
    ///
    /// Every segment must match the corresponding port returned by `layout`.
    unsafe fn view<'a>(
        segments: &'a mut [Option<Segment<I>>],
    ) -> Self::View<'a>
    where
        I: 'a,
        Self: 'a;
}

// ----------------------------------------------------------------------------

/// Typed computation over independent inputs and exactly one output type.
///
/// Revision progress is an installation property, not an action-type property.
/// Mark an installed [`Job`] with [`Job::with_progress`] when this action should
/// receive progress events.
pub trait Action<I>: Sized + Send + 'static
where
    I: Id,
{
    /// Fixed tuple or homogeneous vector of input value types.
    type Inputs: Inputs<I>;
    /// Value type emitted by this action.
    type Output: Value;

    /// Declares permitted concurrency for independent input slices.
    ///
    /// The default permits only one active invocation. Replicas must preserve
    /// action semantics and may not own independently diverging state.
    fn concurrency(&self) -> Concurrency<Self> {
        Concurrency::default()
    }

    /// Executes one scheduler-selected batch slice.
    ///
    /// Transport partitioning is not semantic: processing consecutive slices
    /// must be equivalent to processing their concatenation. Windowing,
    /// aggregation, and other cross-slice meaning belongs in action-owned
    /// state.
    fn execute(&mut self, context: Context<'_, I, Self>);
}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

enum InputStorage<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

// ----------------------------------------------------------------------------

/// Static description of an action's input lanes.
pub enum InputLayout {
    /// Exact heterogeneous input ports.
    Fixed(Vec<Port>),
    /// Any number of homogeneous input ports.
    Repeated(Port),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One input payload whose transport ownership remains hidden.
pub struct InputValue<'a, T> {
    storage: InputStorage<'a, T>,
}

// ----------------------------------------------------------------------------

/// Action-local semantic identity of one replaceable wake.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct WakeKey(u64);

// ----------------------------------------------------------------------------

/// One action-local keyed wake update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Wake {
    key: WakeKey,
    deadline: Option<Instant>,
}

// ----------------------------------------------------------------------------

/// One wake update returned by an action invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::scheduler) struct WakeRequest {
    wake: Wake,
}

// ----------------------------------------------------------------------------

/// Exact in-process identity of one typed action port.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Port {
    id: TypeId,
    name: &'static str,
}

// ----------------------------------------------------------------------------

/// Invocation-wide output and outcome builder.
pub struct Output<I, V> {
    items: Vec<Change<I, V>>,
    outcomes: Outcomes,
    evaluations: EvaluationChanges<I>,
    instrumentation: Instrumentation,
    wakes: Vec<WakeRequest>,
    capacity_hint: usize,
    connected: bool,
}

// ----------------------------------------------------------------------------

/// Output capability scoped to one action callback.
pub struct Emitter<'a, I, V> {
    items: &'a mut Vec<Change<I, V>>,
    outcomes: &'a mut Outcomes,
    evaluations: &'a mut EvaluationChanges<I>,
    instrumentation: &'a mut Instrumentation,
    wakes: &'a mut Vec<WakeRequest>,
    capacity_hint: &'a mut usize,
    connected: bool,
}

// ----------------------------------------------------------------------------

/// Scoped typed driver over one erased job input segment.
pub struct Input<'a, I, V>
where
    I: Id,
    V: Value,
{
    segment: Option<&'a mut Segment<I>>,
    marker: PhantomData<fn() -> V>,
}

// ----------------------------------------------------------------------------

/// One-pass typed views over homogeneous input lanes.
pub struct Lanes<'a, I, V>
where
    I: Id,
    V: Value,
{
    segments: slice::IterMut<'a, Option<Segment<I>>>,
    marker: PhantomData<fn() -> V>,
}

// ----------------------------------------------------------------------------

/// Permitted concurrency for independent invocations of one action.
///
/// The scheduler always chooses the actual concurrency. A bound limits that
/// choice, while adaptive concurrency imposes no action-specific limit.
pub struct Concurrency<A> {
    maximum: Option<NonZeroUsize>,
    replica: Option<fn(&A) -> A>,
}

// ----------------------------------------------------------------------------

/// Scoped typed view over one action invocation.
pub struct Context<'a, I, A>
where
    I: Id,
    A: Action<I>,
    A::Inputs: 'a,
{
    /// Scheduler revision whose homogeneous work is being executed.
    pub revision: RevisionId,
    /// Independent typed input drivers.
    pub inputs: <A::Inputs as Inputs<I>>::View<'a>,
    /// Typed output and outcome builder.
    pub output: &'a mut Output<I, A::Output>,
    /// One-pass control-plane events serialized with data invocations.
    pub events: Events<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, T> InputValue<'a, T> {
    /// Borrows the value independently of transport ownership.
    #[must_use]
    pub const fn as_ref(&self) -> &T {
        match &self.storage {
            InputStorage::Owned(value) => value,
            InputStorage::Borrowed(value) => value,
        }
    }

    /// Returns an owned payload, cloning only when transport requires it.
    #[must_use]
    pub fn into_owned(self) -> T
    where
        T: Clone,
    {
        match self.storage {
            InputStorage::Owned(value) => value,
            InputStorage::Borrowed(value) => value.clone(),
        }
    }

    const fn owned(value: T) -> Self {
        Self {
            storage: InputStorage::Owned(value),
        }
    }

    const fn borrowed(value: &'a T) -> Self {
        Self {
            storage: InputStorage::Borrowed(value),
        }
    }
}

// ----------------------------------------------------------------------------

impl WakeKey {
    /// Creates one action-local wake identity.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

// ----------------------------------------------------------------------------

impl Wake {
    /// Installs or replaces one keyed wake at a deadline.
    #[must_use]
    pub const fn at(key: WakeKey, deadline: Instant) -> Self {
        Self { key, deadline: Some(deadline) }
    }

    /// Removes the current keyed wake, if any.
    #[must_use]
    pub const fn clear(key: WakeKey) -> Self {
        Self { key, deadline: None }
    }
}

// ----------------------------------------------------------------------------

impl WakeRequest {
    pub(in crate::scheduler) const fn new(wake: Wake) -> Self {
        Self { wake }
    }

    pub(in crate::scheduler) const fn key(&self) -> WakeKey {
        self.wake.key
    }

    pub(in crate::scheduler) const fn deadline(&self) -> Option<Instant> {
        self.wake.deadline
    }

    pub(in crate::scheduler) const fn into_parts(
        self,
    ) -> (WakeKey, Option<Instant>) {
        (self.wake.key, self.wake.deadline)
    }
}

// ----------------------------------------------------------------------------

impl Port {
    /// Describes one identity and value type pair.
    #[must_use]
    pub fn of<I, V>() -> Self
    where
        I: 'static,
        V: 'static,
    {
        Self {
            id: TypeId::of::<Change<I, V>>(),
            name: type_name::<V>(),
        }
    }

    /// Returns the diagnostic value type name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.name
    }
}

// ----------------------------------------------------------------------------

impl<I, V> Output<I, V> {
    fn new(connected: bool) -> Self {
        Self { connected, ..Self::default() }
    }

    fn reserve(&mut self, additional: usize) {
        if self.connected {
            // This is deliberately a lazy hint, not Vec::reserve. Filters and
            // events frequently emit nothing and must not allocate merely
            // because they inspected an input segment.
            let target = self
                .items
                .len()
                .checked_add(additional)
                .expect("action output capacity overflowed");
            self.capacity_hint = self.capacity_hint.max(target);
        }
    }

    fn emitter(&mut self) -> Emitter<'_, I, V> {
        Emitter {
            items: &mut self.items,
            outcomes: &mut self.outcomes,
            evaluations: &mut self.evaluations,
            instrumentation: &mut self.instrumentation,
            wakes: &mut self.wakes,
            capacity_hint: &mut self.capacity_hint,
            connected: self.connected,
        }
    }

    fn seal(self) -> SealedOutput<I, V> {
        (
            self.items,
            self.outcomes,
            self.evaluations,
            self.wakes,
            self.instrumentation,
        )
    }
}

// ----------------------------------------------------------------------------

impl<I, V> Emitter<'_, I, V> {
    /// Reports one sparse historical error from this invocation.
    pub fn report(&mut self, error: Error) {
        self.outcomes.report(error);
    }

    /// Rejects the primary evaluation for `key`.
    pub fn reject(&mut self, key: I, error: Error)
    where
        I: Eq,
    {
        self.reject_at::<DefaultEvaluation>(key, error);
    }

    /// Resolves the primary evaluation for `key`.
    pub fn resolve(&mut self, key: I)
    where
        I: Eq,
    {
        self.resolve_at::<DefaultEvaluation>(key);
    }

    /// Rejects one explicitly distinguished evaluation kind for `key`.
    pub fn reject_at<D>(&mut self, key: I, error: Error)
    where
        D: 'static,
        I: Eq,
    {
        self.outcomes.report(error.clone());
        self.evaluations.reject::<D>(key, error);
    }

    /// Resolves one explicitly distinguished evaluation kind for `key`.
    pub fn resolve_at<D>(&mut self, key: I)
    where
        D: 'static,
        I: Eq,
    {
        self.evaluations.resolve::<D>(key);
    }

    /// Emits a named zero-duration annotation.
    pub fn mark(&mut self, name: impl Into<std::borrow::Cow<'static, str>>) {
        Recorder::mark(self, name.into());
    }

    /// Measures one explicitly named operation.
    ///
    /// The monotonic clock is read only when this method is called. Records
    /// emitted by the callback precede the resulting measurement.
    pub fn measure<R>(
        &mut self, name: impl Into<std::borrow::Cow<'static, str>>,
        callback: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let start = Instant::now();
        let result = callback(self);
        Recorder::measure(self, name.into(), start.elapsed());
        result
    }

    /// Appends one insertion or replacement.
    pub fn insert(&mut self, identity: I, value: V) {
        if self.prepare() {
            self.items.push(Change::Insert(identity, value));
        }
    }

    /// Appends one removal.
    pub fn remove(&mut self, identity: I) {
        if self.prepare() {
            self.items.push(Change::Remove(identity));
        }
    }

    /// Applies one action-local keyed wake update.
    pub fn wake(&mut self, wake: Wake) {
        self.wakes.push(WakeRequest::new(wake));
    }

    fn prepare(&mut self) -> bool {
        if !self.connected {
            return false;
        }
        if self.items.capacity() < *self.capacity_hint {
            let additional = self
                .capacity_hint
                .checked_sub(self.items.len())
                .expect("action output hint precedes its length");
            self.items.reserve(additional);
        }
        // One reservation satisfies the complete current input hint. Keep
        // subsequent pushes on Vec's ordinary growth path until another input
        // supplies a new hint.
        *self.capacity_hint = 0;
        true
    }
}

// ----------------------------------------------------------------------------

impl<'a, I, V> Input<'a, I, V>
where
    I: Id,
    V: Value,
{
    fn new(segment: Option<&'a mut Segment<I>>) -> Self {
        if let Some(segment) = segment {
            segment.promote_if_unique();
            return Self {
                segment: Some(segment),
                marker: PhantomData,
            };
        }
        Self { segment, marker: PhantomData }
    }

    /// Drives every available item and records its ordinary result.
    pub fn for_each<O>(
        self, output: &mut Output<I, O>,
        mut callback: impl FnMut(
            InputChange<'_, I, V>,
            &mut Emitter<'_, I, O>,
        ) -> Result,
    ) {
        let Some(segment) = self.segment else {
            return;
        };
        output.reserve(segment.len());
        loop {
            let Some(change) = segment.pop_front::<V>() else {
                break;
            };
            if let Err(error) = callback(change, &mut output.emitter()) {
                output.outcomes.report(error);
            }
        }
    }
}

// ----------------------------------------------------------------------------

impl<A> Concurrency<A> {
    /// Permits scheduler-selected concurrency without an action-specific bound.
    #[must_use]
    pub const fn adaptive_with(replica: fn(&A) -> A) -> Self {
        Self {
            maximum: None,
            replica: Some(replica),
        }
    }

    /// Permits scheduler-selected concurrency up to `maximum` instances.
    #[must_use]
    pub const fn bounded_with(
        maximum: NonZeroUsize, replica: fn(&A) -> A,
    ) -> Self {
        Self {
            maximum: Some(maximum),
            replica: Some(replica),
        }
    }
}

impl<A> Concurrency<A>
where
    A: Clone,
{
    /// Permits scheduler-selected concurrency using cloned action instances.
    #[must_use]
    pub const fn adaptive() -> Self {
        Self::adaptive_with(A::clone)
    }

    /// Permits at most `maximum` cloned action instances.
    #[must_use]
    pub const fn bounded(maximum: NonZeroUsize) -> Self {
        Self::bounded_with(maximum, A::clone)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, V> Default for Output<I, V> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            outcomes: Outcomes::default(),
            evaluations: EvaluationChanges::default(),
            instrumentation: Instrumentation::default(),
            wakes: Vec::new(),
            capacity_hint: 0,
            connected: true,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, V> Recorder for Emitter<'_, I, V> {
    fn mark(&mut self, name: std::borrow::Cow<'static, str>) {
        self.instrumentation
            .push(Record::Annotation(Annotation::new(name)));
    }

    fn measure(
        &mut self, name: std::borrow::Cow<'static, str>,
        elapsed: std::time::Duration,
    ) {
        self.instrumentation
            .push(Record::Measurement(Measurement::new(name, elapsed)));
    }
}

impl<I, V> zrx_diagnostic::sink::Sink for Emitter<'_, I, V> {
    fn emit(&mut self, diagnostic: zrx_diagnostic::Diagnostic) {
        self.instrumentation.push(Record::Diagnostic(diagnostic));
    }
}

// ----------------------------------------------------------------------------

impl<'a, I, V> Iterator for Lanes<'a, I, V>
where
    I: Id,
    V: Value,
{
    type Item = Input<'a, I, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.segments
            .next()
            .map(|segment| Input::new(segment.as_mut()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.segments.size_hint()
    }
}

impl<I, V> ExactSizeIterator for Lanes<'_, I, V>
where
    I: Id,
    V: Value,
{
}

impl<I, V> FusedIterator for Lanes<'_, I, V>
where
    I: Id,
    V: Value,
{
}

// ----------------------------------------------------------------------------

impl<T> inputs::Sealed for Vec<T> {}

impl<I, T> Inputs<I> for Vec<T>
where
    I: Id,
    T: Value,
{
    type View<'a>
        = Lanes<'a, I, T>
    where
        I: 'a,
        Self: 'a;

    fn layout() -> InputLayout {
        InputLayout::Repeated(Port::of::<I, T>())
    }

    unsafe fn view<'a>(segments: &'a mut [Option<Segment<I>>]) -> Self::View<'a>
    where
        I: 'a,
        Self: 'a,
    {
        Lanes {
            segments: segments.iter_mut(),
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<A> Default for Concurrency<A> {
    fn default() -> Self {
        Self {
            maximum: Some(NonZeroUsize::MIN),
            replica: None,
        }
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// One input change with an owned identity and transport-managed payload.
pub type InputChange<'a, I, V> = Change<I, InputValue<'a, V>>;

type SealedOutput<I, V> = (
    Vec<Change<I, V>>,
    Outcomes,
    EvaluationChanges<I>,
    Vec<WakeRequest>,
    Instrumentation,
);

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use zrx_diagnostic::sink::Sink;

    use super::{Output, Record, Wake, WakeKey};
    use crate::scheduler::{Id, Value};

    #[derive(Clone)]
    struct OpaqueValue;

    #[derive(Clone)]
    struct OpaqueId;

    impl Value for OpaqueValue {}

    impl Value for OpaqueId {}

    #[test]
    fn contracts_require_only_transport_capabilities() {
        fn value<T>()
        where
            T: Value,
        {
        }
        fn id<T>()
        where
            T: Id,
        {
        }

        value::<OpaqueValue>();
        id::<OpaqueId>();
    }

    #[test]
    fn output_capacity_hint_allocates_only_on_the_first_emission() {
        let mut output = Output::<u64, u64>::new(true);
        output.reserve(1_024);
        assert_eq!(output.items.capacity(), 0);

        output.emitter().insert(1, 1);

        assert!(output.items.capacity() >= 1_024);
    }

    #[test]
    fn disconnected_output_discards_items_but_retains_wakes() {
        let mut output = Output::<u64, u64>::new(false);
        output.reserve(1_024);
        let mut emitter = output.emitter();
        emitter.insert(1, 1);
        emitter.remove(2);
        emitter.wake(Wake::at(WakeKey::new(1), Instant::now()));
        let (items, outcomes, _, wakes, instrumentation) = output.seal();

        assert!(items.is_empty());
        assert!(outcomes.is_empty());
        assert!(instrumentation.is_empty());
        assert_eq!(wakes.len(), 1);
    }

    #[test]
    fn emitter_records_diagnostics_and_annotations_in_order() {
        let mut output = Output::<u64, u64>::new(false);
        {
            let mut first = output.emitter();
            first.emit(zrx_diagnostic::warning!("warning"));
            first.mark("checked");
        }
        output.emitter().mark(String::from("dynamic"));

        let (_, _, _, _, instrumentation) = output.seal();
        let [diagnostic, marker, dynamic] = instrumentation.records() else {
            panic!("three ordered records expected")
        };
        assert!(
            matches!(diagnostic, Record::Diagnostic(value) if value.message == "warning")
        );
        assert!(
            matches!(marker, Record::Annotation(value) if value.name() == "checked")
        );
        assert!(
            matches!(dynamic, Record::Annotation(value) if value.name() == "dynamic")
        );
    }

    #[test]
    fn explicit_measurement_preserves_inner_record_order() {
        let mut output = Output::<u64, u64>::new(false);
        output.emitter().measure("parse", |emit| {
            emit.mark("inside");
        });

        let (_, _, _, _, instrumentation) = output.seal();
        let [marker, measurement] = instrumentation.records() else {
            panic!("inner marker and completed measurement expected")
        };
        assert!(
            matches!(marker, Record::Annotation(value) if value.name() == "inside")
        );
        assert!(
            matches!(measurement, Record::Measurement(value) if value.name() == "parse")
        );
    }
}
