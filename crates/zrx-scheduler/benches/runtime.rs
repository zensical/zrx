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

//! Scheduler runtime benchmarks.

use criterion::{
    BenchmarkId, Criterion, Throughput, black_box, criterion_group,
    criterion_main,
};
use crossbeam::channel::Select;
use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering};

use zrx_executor::Strategy;
use zrx_executor::strategy::{Immediate, WorkSharing};
use zrx_scheduler::Change;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Concurrency, Context, Job};
use zrx_scheduler::plan::{
    InputBinding, InputId, OutputBinding, OutputId, Plan, Route,
};
use zrx_scheduler::{
    Egress, Error, PlanId, Report, Scheduler, Session, Writer,
};

const BATCHES: [usize; 4] = [1, 8, 64, 1_024];
const INPUT_A: InputId = InputId::new(1);
const INPUT_B: InputId = InputId::new(2);
const INPUT_C: InputId = InputId::new(3);
const OUTPUT: OutputId = OutputId::new(1);

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

static LIVE: AtomicIsize = AtomicIsize::new(0);

static PEAK: AtomicIsize = AtomicIsize::new(0);

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

criterion_group!(benches, unary, placement, topology, progress);

criterion_main!(benches);

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Batch {
    base: u64,
    count: usize,
    value: u64,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
struct Revision(u64);

// ----------------------------------------------------------------------------

struct Runtime<S = Immediate>
where
    S: Strategy,
{
    scheduler: Scheduler<u64, S>,
    plan: PlanId,
    sessions: HashMap<InputId, Session<u64, u64>>,
    writers: HashMap<Revision, Writer<u64, u64>>,
    inputs: HashMap<Revision, InputId>,
    next_revision: u64,
    inline: bool,
}

// ----------------------------------------------------------------------------

struct CountingAllocator;

// ----------------------------------------------------------------------------

struct AllocationProfile {
    allocations: usize,
    bytes: usize,
    peak: usize,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Map;

// ----------------------------------------------------------------------------

struct FilterNone;

// ----------------------------------------------------------------------------

struct Expand;

// ----------------------------------------------------------------------------

struct DropAll;

// ----------------------------------------------------------------------------

#[derive(Default)]
struct ObserveProgress {
    changes: usize,
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct Join3 {
    values: HashMap<u64, [Option<u64>; 3]>,
}

// ----------------------------------------------------------------------------

struct Unary<S>
where
    S: Strategy,
{
    runtime: Runtime<S>,
    batch: usize,
    next: u64,
}

// ----------------------------------------------------------------------------

struct FanOut {
    runtime: Runtime,
    batch: usize,
    next: u64,
}

// ----------------------------------------------------------------------------

struct Join {
    runtime: Runtime,
    batch: usize,
    next: u64,
}

// ----------------------------------------------------------------------------

struct ProgressOverlay {
    runtime: Runtime,
    batch: usize,
    next: u64,
    subscribers: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Batch {
    fn changes(self) -> impl ExactSizeIterator<Item = Change<u64, u64>> {
        (0..self.count).map(move |offset| {
            let offset = u64::try_from(offset).expect("batch offset fits");
            Change::Insert(self.base.wrapping_add(offset), self.value)
        })
    }
}

// ----------------------------------------------------------------------------

impl Runtime<Immediate> {
    fn new(plan: Plan<u64>) -> Self {
        let mut scheduler = Scheduler::inline();
        let plan = scheduler.attach(plan);
        Self {
            scheduler,
            plan,
            sessions: HashMap::new(),
            writers: HashMap::new(),
            inputs: HashMap::new(),
            next_revision: 0,
            inline: true,
        }
    }
}

impl<S> Runtime<S>
where
    S: Strategy,
{
    fn with_strategy(plan: Plan<u64>, strategy: S) -> Self {
        let mut scheduler = Scheduler::new(strategy);
        let plan = scheduler.attach(plan);
        Self {
            scheduler,
            plan,
            sessions: HashMap::new(),
            writers: HashMap::new(),
            inputs: HashMap::new(),
            next_revision: 0,
            inline: false,
        }
    }

    fn begin(&mut self, input: InputId) -> Result<Revision, Error> {
        let session = match self.sessions.remove(&input) {
            Some(session) => session,
            None => self.scheduler.attachment(self.plan)?.session(input)?,
        };
        let writer = session.begin()?;
        let revision = Revision(self.next_revision);
        self.next_revision = self
            .next_revision
            .checked_add(1)
            .expect("benchmark revision identity exhausted");
        assert!(self.writers.insert(revision, writer).is_none());
        assert!(self.inputs.insert(revision, input).is_none());
        Ok(revision)
    }

    fn ingress(
        &mut self, revision: Revision, batch: Batch,
    ) -> Result<(), Error> {
        let writer = self
            .writers
            .get_mut(&revision)
            .expect("benchmark revision remains open");
        writer.emit_batch(batch.changes())?;
        Ok(())
    }

    fn ingress_incremental(
        &mut self, revision: Revision, batch: Batch,
    ) -> Result<(), Error> {
        let writer = self
            .writers
            .get_mut(&revision)
            .expect("benchmark revision remains open");
        for change in batch.changes() {
            writer.emit(change)?;
        }
        writer.flush()?;
        Ok(())
    }

    fn seal(&mut self, revision: Revision) -> Result<(), Error> {
        let writer = self
            .writers
            .remove(&revision)
            .expect("benchmark revision remains open");
        let session = writer.seal()?;
        let input = self
            .inputs
            .remove(&revision)
            .expect("benchmark revision remains open");
        assert!(self.sessions.insert(input, session).is_none());
        Ok(())
    }

    fn egress(&mut self) -> Option<Egress<u64>> {
        self.scheduler.attachment(self.plan).unwrap().egress()
    }

    fn tick(&mut self) -> Report {
        self.scheduler
            .tick()
            .map_or_else(Report::default, zrx_scheduler::Tick::into_report)
    }

    fn run_until_idle(&mut self) -> Report {
        let mut report = Report::default();
        loop {
            while let Some(tick) = self.scheduler.tick() {
                report.append(tick.into_report());
            }
            if self.inline {
                break;
            }
            let waiting = {
                let mut select = Select::new();
                let readiness = self.scheduler.register(&mut select);
                if readiness.pending() {
                    let operation = select.ready();
                    assert!(readiness.contains(operation));
                    true
                } else {
                    false
                }
            };
            if !waiting {
                break;
            }
        }
        report
    }
}

// ----------------------------------------------------------------------------

impl Join3 {
    fn insert(&mut self, lane: usize, key: u64, value: u64) -> Option<u64> {
        let values = self.values.entry(key).or_default();
        values[lane] = Some(value);
        let [Some(first), Some(second), Some(third)] = *values else {
            return None;
        };
        self.values.remove(&key);
        Some(first.wrapping_add(second).wrapping_add(third))
    }
}

// ----------------------------------------------------------------------------

impl<S> Unary<S>
where
    S: Strategy,
{
    fn run(&mut self) -> u64 {
        self.run_with(false)
    }

    fn run_incremental(&mut self) -> u64 {
        self.run_with(true)
    }

    fn run_with(&mut self, incremental: bool) -> u64 {
        let revision = self.runtime.begin(INPUT_A).unwrap();
        let base = self.next;
        self.next = self
            .next
            .wrapping_add(u64::try_from(self.batch).expect("batch fits"));

        let batch = items(base, self.batch, 1);
        if incremental {
            self.runtime.ingress_incremental(revision, batch).unwrap();
        } else {
            self.runtime.ingress(revision, batch).unwrap();
        }
        self.runtime.seal(revision).unwrap();
        let report = self.runtime.run_until_idle();
        let mut settlements = report.settlements().len();
        let mut checksum = 0_u64;
        while let Some(egress) = self.runtime.egress() {
            egress.for_each::<u64>(|change| {
                if let Change::Insert(key, value) = change {
                    checksum = checksum.wrapping_add(key ^ *value.as_ref());
                }
            });
        }
        settlements += self.runtime.tick().settlements().len();
        debug_assert_eq!(settlements, 1);
        checksum
    }
}

// ----------------------------------------------------------------------------

impl FanOut {
    fn new(subscribers: usize, batch: usize) -> Self {
        let mut jobs = vec![map_job()];
        jobs.extend((0..subscribers).map(|_| Job::new(DropAll)));
        let routes = std::iter::once(
            (0..subscribers)
                .map(|subscriber| Route::new(subscriber + 1, 0))
                .collect(),
        )
        .chain((0..subscribers).map(|_| Vec::new()))
        .collect();
        let program = Plan::builder(jobs, routes)
            .inputs(vec![InputBinding::new::<u64, u64>(
                INPUT_A,
                Route::new(0, 0),
            )])
            .build()
            .unwrap();
        Self {
            runtime: Runtime::new(program),
            batch,
            next: 1,
        }
    }

    fn run(&mut self) -> u64 {
        let revision = self.runtime.begin(INPUT_A).unwrap();
        let base = self.next;
        self.next = self
            .next
            .wrapping_add(u64::try_from(self.batch).expect("batch fits"));

        self.runtime
            .ingress(revision, items(base, self.batch, 1))
            .unwrap();
        self.runtime.seal(revision).unwrap();
        let report = self.runtime.run_until_idle();
        debug_assert_eq!(report.settlements().len(), 1);
        base
    }
}

// ----------------------------------------------------------------------------

impl ProgressOverlay {
    fn new(subscribers: usize, batch: usize) -> Self {
        let mut jobs = vec![Job::new(FilterNone)];
        jobs.extend(
            (0..subscribers)
                .map(|_| Job::new(ObserveProgress::default()).with_progress()),
        );
        let routes = std::iter::once(
            (0..subscribers)
                .map(|subscriber| Route::new(subscriber + 1, 0))
                .collect(),
        )
        .chain((0..subscribers).map(|_| Vec::new()))
        .collect();
        let outputs = (0..subscribers)
            .map(|subscriber| {
                OutputBinding::new::<u64, u64>(
                    OutputId::new(
                        u64::try_from(subscriber + 1).expect("subscriber fits"),
                    ),
                    subscriber + 1,
                )
            })
            .collect();
        let program = Plan::builder(jobs, routes)
            .inputs(vec![InputBinding::new::<u64, u64>(
                INPUT_A,
                Route::new(0, 0),
            )])
            .outputs(outputs)
            .build()
            .unwrap();
        Self {
            runtime: Runtime::new(program),
            batch,
            next: 1,
            subscribers,
        }
    }

    fn run(&mut self) -> u64 {
        let revision = self.runtime.begin(INPUT_A).unwrap();
        let base = self.next;
        self.next = self
            .next
            .wrapping_add(u64::try_from(self.batch).expect("batch fits"));
        self.runtime
            .ingress(revision, items(base, self.batch, 1))
            .unwrap();
        self.runtime.seal(revision).unwrap();
        let report = self.runtime.run_until_idle();
        debug_assert!(report.settlements().is_empty());
        let mut checksum = 0_u64;
        let mut outputs = 0;
        while let Some(egress) = self.runtime.egress() {
            egress.for_each::<u64>(|change| {
                if let Change::Insert(_, value) = change {
                    checksum = checksum.wrapping_add(*value.as_ref());
                }
            });
            outputs += 1;
        }
        debug_assert_eq!(outputs, self.subscribers);
        let report = self.runtime.tick();
        debug_assert_eq!(report.settlements().len(), 1);
        checksum
    }
}

// ----------------------------------------------------------------------------

impl Join {
    fn new(batch: usize) -> Self {
        let program =
            Plan::builder(vec![Job::new(Join3::default())], vec![vec![]])
                .inputs(vec![
                    InputBinding::new::<u64, u64>(INPUT_A, Route::new(0, 0)),
                    InputBinding::new::<u64, u64>(INPUT_B, Route::new(0, 1)),
                    InputBinding::new::<u64, u64>(INPUT_C, Route::new(0, 2)),
                ])
                .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT, 0)])
                .build()
                .unwrap();
        Self {
            runtime: Runtime::new(program),
            batch,
            next: 1,
        }
    }

    fn run(&mut self) -> u64 {
        let revisions = [
            self.runtime.begin(INPUT_A).unwrap(),
            self.runtime.begin(INPUT_B).unwrap(),
            self.runtime.begin(INPUT_C).unwrap(),
        ];
        let base = self.next;
        self.next = self
            .next
            .wrapping_add(u64::try_from(self.batch).expect("batch fits"));
        for (revision, value) in revisions.into_iter().zip(1_u64..) {
            self.runtime
                .ingress(revision, items(base, self.batch, value))
                .unwrap();
            self.runtime.seal(revision).unwrap();
        }
        let report = self.runtime.run_until_idle();
        debug_assert_eq!(report.settlements().len(), 2);
        let mut checksum = 0_u64;
        self.runtime
            .egress()
            .expect("join output is ready")
            .for_each::<u64>(|change| {
                if let Change::Insert(key, value) = change {
                    checksum = checksum.wrapping_add(key ^ *value.as_ref());
                }
            });
        let report = self.runtime.tick();
        debug_assert_eq!(report.settlements().len(), 1);
        checksum
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

// SAFETY: every operation delegates the original pointer and layout unchanged
// to the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller supplies a valid allocation layout.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
            update_live(isize::try_from(layout.size()).expect("size fits"));
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        update_live(-isize::try_from(layout.size()).expect("size fits"));
        // SAFETY: the pointer and layout came from the delegated allocator.
        unsafe { System.dealloc(pointer, layout) };
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for AllocationProfile {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} allocs, {} bytes, {} peak",
            self.allocations, self.bytes, self.peak
        )
    }
}

// ----------------------------------------------------------------------------

impl Action<u64> for Map {
    type Inputs = (u64,);
    type Output = u64;

    fn concurrency(&self) -> Concurrency<Self> {
        Concurrency::adaptive()
    }

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned().wrapping_add(1));
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

// ----------------------------------------------------------------------------

impl Action<u64> for FilterNone {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |_, _| Ok(()));
    }
}

// ----------------------------------------------------------------------------

impl Action<u64> for Expand {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    let value = value.into_owned();
                    emit.insert(key.wrapping_mul(2), value);
                    emit.insert(
                        key.wrapping_mul(2).wrapping_add(1),
                        value.wrapping_add(1),
                    );
                }
                Change::Remove(key) => {
                    emit.remove(key.wrapping_mul(2));
                    emit.remove(key.wrapping_mul(2).wrapping_add(1));
                }
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

// ----------------------------------------------------------------------------

impl Action<u64> for DropAll {
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

// ----------------------------------------------------------------------------

impl Action<u64> for ObserveProgress {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| {
            self.changes = self.changes.wrapping_add(1);
            Ok(())
        });
        events.for_each(output, |event, emit| {
            match event {
                Event::Progress(
                    ProgressEvent::Begin | ProgressEvent::Abort,
                ) => {
                    self.changes = 0;
                }
                Event::Progress(ProgressEvent::End) => {
                    emit.insert(
                        0,
                        u64::try_from(self.changes).expect("count fits"),
                    );
                    self.changes = 0;
                }
                Event::Wake { .. } => {}
            }
            Ok(())
        });
    }
}

// ----------------------------------------------------------------------------

impl Action<u64> for Join3 {
    type Inputs = (u64, u64, u64);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: (first, second, third),
            output,
            events,
            ..
        } = context;
        first.for_each(output, |change, emit| {
            if let Change::Insert(key, value) = change
                && let Some(value) = self.insert(0, key, value.into_owned())
            {
                emit.insert(key, value);
            }
            Ok(())
        });
        second.for_each(output, |change, emit| {
            if let Change::Insert(key, value) = change
                && let Some(value) = self.insert(1, key, value.into_owned())
            {
                emit.insert(key, value);
            }
            Ok(())
        });
        third.for_each(output, |change, emit| {
            if let Change::Insert(key, value) = change
                && let Some(value) = self.insert(2, key, value.into_owned())
            {
                emit.insert(key, value);
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn update_live(delta: isize) {
    let live = LIVE.fetch_add(delta, Ordering::Relaxed) + delta;
    PEAK.fetch_max(live, Ordering::Relaxed);
}

fn profile(run: impl FnOnce() -> u64) -> AllocationProfile {
    let baseline = LIVE.load(Ordering::Relaxed);
    ALLOCATIONS.store(0, Ordering::Relaxed);
    ALLOCATED.store(0, Ordering::Relaxed);
    PEAK.store(baseline, Ordering::Relaxed);
    black_box(run());
    AllocationProfile {
        allocations: ALLOCATIONS.load(Ordering::Relaxed),
        bytes: ALLOCATED.load(Ordering::Relaxed),
        peak: usize::try_from(PEAK.load(Ordering::Relaxed) - baseline)
            .expect("profile peak is non-negative"),
    }
}

fn map_job() -> Job<u64> {
    Job::new::<Map>(Map)
}

const fn items(base: u64, count: usize, value: u64) -> Batch {
    Batch { base, count, value }
}

fn unary_program(stages: usize, terminal: Job<u64>) -> Plan<u64> {
    let mut jobs: Vec<_> =
        (0..stages.saturating_sub(1)).map(|_| map_job()).collect();
    jobs.push(terminal);
    let routes = (0..jobs.len())
        .map(|node| {
            if node + 1 < jobs.len() {
                vec![Route::new(node + 1, 0)]
            } else {
                Vec::new()
            }
        })
        .collect();
    Plan::builder(jobs, routes)
        .inputs(vec![InputBinding::new::<u64, u64>(
            INPUT_A,
            Route::new(0, 0),
        )])
        .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT, stages - 1)])
        .build()
        .unwrap()
}

fn unary_immediate(
    stages: usize, terminal: Job<u64>, batch: usize,
) -> Unary<Immediate> {
    Unary {
        runtime: Runtime::new(unary_program(stages, terminal)),
        batch,
        next: 1,
    }
}

fn unary_worker(batch: usize, shards: usize) -> Unary<WorkSharing> {
    Unary {
        runtime: Runtime::with_strategy(
            unary_program(1, map_job()),
            WorkSharing::new(shards),
        ),
        batch,
        next: 1,
    }
}

fn print_profiles() {
    for batch in BATCHES {
        let mut forward = unary_immediate(1, Job::forward::<u64>(), batch);
        let mut one = unary_immediate(1, map_job(), batch);
        let mut five = unary_immediate(5, map_job(), batch);
        let mut filter = unary_immediate(1, Job::new(FilterNone), batch);
        let mut expand = unary_immediate(1, Job::new(Expand), batch);
        let mut fanout = FanOut::new(8, batch);
        let mut join = Join::new(batch);
        let mut progress = ProgressOverlay::new(1, batch);
        let mut progress_many = ProgressOverlay::new(8, batch);
        black_box(forward.run());
        black_box(one.run());
        black_box(five.run());
        black_box(filter.run());
        black_box(expand.run());
        black_box(fanout.run());
        black_box(join.run());
        black_box(progress.run());
        black_box(progress_many.run());
        eprintln!(
            "alloc/runtime/{batch}: forward={} one={} five={} filter-zero={} expand={} fanout-8={} join-3={} progress-1={} progress-8={}",
            profile(|| forward.run()),
            profile(|| one.run()),
            profile(|| five.run()),
            profile(|| filter.run()),
            profile(|| expand.run()),
            profile(|| fanout.run()),
            profile(|| join.run()),
            profile(|| progress.run()),
            profile(|| progress_many.run()),
        );
    }
}

fn unary(c: &mut Criterion) {
    print_profiles();
    let mut group = c.benchmark_group("scheduler/unary");
    for batch in BATCHES {
        group.throughput(Throughput::Elements(
            u64::try_from(batch).expect("batch fits"),
        ));
        let mut forward = unary_immediate(1, Job::forward::<u64>(), batch);
        group.bench_with_input(
            BenchmarkId::new("forward", batch),
            &batch,
            |bencher, _| bencher.iter(|| black_box(forward.run())),
        );
        for stages in [1, 5] {
            let mut runtime = unary_immediate(stages, map_job(), batch);
            group.bench_with_input(
                BenchmarkId::new(format!("immediate-{stages}"), batch),
                &batch,
                |bencher, _| bencher.iter(|| black_box(runtime.run())),
            );
        }
        let mut incremental = unary_immediate(1, map_job(), batch);
        group.bench_with_input(
            BenchmarkId::new("incremental-1", batch),
            &batch,
            |bencher, _| {
                bencher.iter(|| black_box(incremental.run_incremental()));
            },
        );
        let mut filter = unary_immediate(1, Job::new(FilterNone), batch);
        group.bench_with_input(
            BenchmarkId::new("filter-zero", batch),
            &batch,
            |bencher, _| bencher.iter(|| black_box(filter.run())),
        );
        let mut expand = unary_immediate(1, Job::new(Expand), batch);
        group.bench_with_input(
            BenchmarkId::new("expand", batch),
            &batch,
            |bencher, _| bencher.iter(|| black_box(expand.run())),
        );
    }
    group.finish();
}

fn placement(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler/placement");
    for batch in BATCHES {
        group.throughput(Throughput::Elements(
            u64::try_from(batch).expect("batch fits"),
        ));
        let mut immediate = unary_immediate(1, map_job(), batch);
        group.bench_with_input(
            BenchmarkId::new("immediate", batch),
            &batch,
            |bencher, _| bencher.iter(|| black_box(immediate.run())),
        );
        for shards in [1, 2, 4] {
            let mut worker = unary_worker(batch, shards);
            group.bench_with_input(
                BenchmarkId::new(format!("worker-{shards}"), batch),
                &batch,
                |bencher, _| bencher.iter(|| black_box(worker.run())),
            );
        }
    }
    group.finish();
}

fn topology(c: &mut Criterion) {
    let mut fanout = c.benchmark_group("scheduler/fanout");
    for batch in BATCHES {
        fanout.throughput(Throughput::Elements(
            u64::try_from(batch).expect("batch fits"),
        ));
        for subscribers in [1, 2, 8] {
            let mut runtime = FanOut::new(subscribers, batch);
            fanout.bench_with_input(
                BenchmarkId::new(format!("subscribers-{subscribers}"), batch),
                &batch,
                |bencher, _| bencher.iter(|| black_box(runtime.run())),
            );
        }
    }
    fanout.finish();

    let mut join = c.benchmark_group("scheduler/join-3");
    for batch in BATCHES {
        join.throughput(Throughput::Elements(
            u64::try_from(batch).expect("batch fits"),
        ));
        let mut runtime = Join::new(batch);
        join.bench_with_input(
            BenchmarkId::from_parameter(batch),
            &batch,
            |bencher, _| bencher.iter(|| black_box(runtime.run())),
        );
    }
    join.finish();
}

fn progress(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler/progress");
    for batch in BATCHES {
        group.throughput(Throughput::Elements(
            u64::try_from(batch).expect("batch fits"),
        ));
        for subscribers in [1, 8] {
            let mut runtime = ProgressOverlay::new(subscribers, batch);
            group.bench_with_input(
                BenchmarkId::new(format!("subscribers-{subscribers}"), batch),
                &batch,
                |bencher, _| bencher.iter(|| black_box(runtime.run())),
            );
        }
    }
    group.finish();
}
