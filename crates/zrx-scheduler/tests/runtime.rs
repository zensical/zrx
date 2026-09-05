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

//! Scheduler runtime integration tests.

use crossbeam::channel::Select;
use std::collections::VecDeque;
use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use zrx_diagnostic::sink::Sink;
use zrx_executor::strategy::{Immediate, WorkSharing};
use zrx_executor::task::Task;
use zrx_executor::{Error as ExecutorError, Strategy};

use zrx_scheduler::Change;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{
    Action, Concurrency, Context, Job, Record, Wake, WakeKey,
};
use zrx_scheduler::plan::{
    InputBinding, InputId, Plan, PlanError, ProgressError, Route, RouteError,
};
use zrx_scheduler::{
    Error as RuntimeError, Report, RevisionId, SessionError, Settlement,
};

#[path = "support/runtime.rs"]
mod support;

use support::{Batch, Runtime};

const INPUT_A: InputId = InputId::new(1);
const INPUT_B: InputId = InputId::new(2);
const PROGRESS_REVISIONS: usize = 8;
const PROGRESS_ITEMS_PER_REVISION: usize = 4;

static BLOCKING_WORKERS: Mutex<()> = Mutex::new(());

fn assert_complete(settlements: &[Settlement]) {
    assert!(matches!(settlements, [Settlement::Complete(_)]));
}

fn assert_aborted(settlements: &[Settlement]) {
    assert!(matches!(settlements, [Settlement::Aborted(_)]));
}

fn drive_until_received<S, T>(
    runtime: &mut Runtime<u64, S>, waiting: &mpsc::Receiver<T>, count: usize,
) -> Vec<T>
where
    S: zrx_executor::Strategy,
{
    let deadline = Instant::now() + Duration::from_secs(1);
    let mut values = Vec::new();
    while values.len() != count {
        let _ = runtime.tick();
        values.extend(waiting.try_iter());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    values
}

fn runtime(program: Plan<u64>) -> Runtime<u64> {
    Runtime::new(program)
}

fn item(_cause: u64, key: u64, value: u64) -> Change<u64, u64> {
    Change::Insert(key, value)
}

#[derive(Debug, Default)]
struct RejectOnce {
    rejected: AtomicBool,
}

impl Strategy for RejectOnce {
    fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
        if !self.rejected.swap(true, Ordering::Relaxed) {
            return Err(ExecutorError::Submit(task));
        }
        drop(task.execute());
        Ok(())
    }

    fn num_workers(&self) -> usize {
        1
    }

    fn num_tasks_running(&self) -> usize {
        0
    }

    fn num_tasks_pending(&self) -> usize {
        0
    }

    fn capacity(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug, Default)]
struct Queued {
    tasks: Arc<Mutex<VecDeque<Box<dyn Task>>>>,
}

impl Queued {
    fn len(&self) -> usize {
        self.tasks.lock().unwrap().len()
    }

    fn execute(&self, index: usize) {
        let task = self.tasks.lock().unwrap().remove(index).unwrap();
        task.execute().execute();
    }

    fn execute_next(&self) -> bool {
        let Some(task) = self.tasks.lock().unwrap().pop_front() else {
            return false;
        };
        task.execute().execute();
        true
    }
}

impl Strategy for Queued {
    fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
        self.tasks.lock().unwrap().push_back(task);
        Ok(())
    }

    fn num_workers(&self) -> usize {
        2
    }

    fn num_tasks_running(&self) -> usize {
        0
    }

    fn num_tasks_pending(&self) -> usize {
        self.len()
    }

    fn capacity(&self) -> usize {
        2
    }
}

#[derive(Debug)]
struct SharedStrategy<S>(Arc<S>);

impl<S> Clone for SharedStrategy<S> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<S> SharedStrategy<S> {
    fn new(strategy: S) -> Self {
        Self(Arc::new(strategy))
    }
}

impl<S> Strategy for SharedStrategy<S>
where
    S: Strategy,
{
    fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
        self.0.submit(task)
    }

    fn num_workers(&self) -> usize {
        self.0.num_workers()
    }

    fn num_tasks_running(&self) -> usize {
        self.0.num_tasks_running()
    }

    fn num_tasks_pending(&self) -> usize {
        self.0.num_tasks_pending()
    }

    fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

fn drain_queued(runtime: &mut Runtime<u64, Queued>, queue: &Queued) -> Report {
    let mut report = Report::default();
    loop {
        while queue.execute_next() {}
        let tick = runtime.tick();
        let progressed = tick.progressed();
        report.append(tick.into_report());

        let pending = {
            let mut select = Select::new();
            runtime.register(&mut select).pending()
        };
        if !progressed && !pending && queue.len() == 0 {
            return report;
        }
        assert!(progressed || queue.len() != 0, "queued runtime stalled");
    }
}

struct Pass;

impl Action<u64> for Pass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
    }
}

type ProgressRecord = (RevisionId, &'static str);

struct RecordProgress(Arc<Mutex<Vec<ProgressRecord>>>);

impl Action<u64> for RecordProgress {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            revision,
            inputs: input,
            output,
            events,
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, _| {
            let name = match event {
                Event::Progress(ProgressEvent::Begin) => "begin",
                Event::Progress(ProgressEvent::End) => "end",
                Event::Progress(ProgressEvent::Abort) => "abort",
                Event::Wake { .. } => {
                    unreachable!(
                        "progress-only test action received another event"
                    )
                }
            };
            self.0.lock().unwrap().push((revision, name));
            Ok(())
        });
    }
}

struct RecordConvergedProgress {
    events: Arc<Mutex<Vec<ProgressRecord>>>,
    started: Option<mpsc::Sender<()>>,
    release: Option<mpsc::Receiver<()>>,
}

impl Action<u64> for RecordConvergedProgress {
    type Inputs = (u64, u64);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            revision,
            inputs: (left, right),
            output,
            events,
        } = context;
        left.for_each(output, |_, _| Ok(()));
        right.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, _| {
            let name = match event {
                Event::Progress(ProgressEvent::Begin) => {
                    if let Some(started) = self.started.take() {
                        started.send(()).unwrap();
                        self.release
                            .take()
                            .expect("blocked progress has a release")
                            .recv()
                            .unwrap();
                    }
                    "begin"
                }
                Event::Progress(ProgressEvent::End) => "end",
                Event::Progress(ProgressEvent::Abort) => "abort",
                Event::Wake { .. } => {
                    unreachable!(
                        "progress-only test action received another event"
                    )
                }
            };
            self.events.lock().unwrap().push((revision, name));
            Ok(())
        });
    }
}

struct DiscardAndSignal {
    lane: usize,
    discarded: mpsc::Sender<usize>,
}

impl Action<u64> for DiscardAndSignal {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |_, _| {
            self.discarded.send(self.lane).unwrap();
            Ok(())
        });
    }
}

fn converged_progress_program(
    progress: RecordConvergedProgress, discarded: mpsc::Sender<usize>,
) -> Plan<u64> {
    Plan::builder(
        vec![
            Job::new(Pass),
            Job::new(DiscardAndSignal {
                lane: 0,
                discarded: discarded.clone(),
            }),
            Job::new(DiscardAndSignal { lane: 1, discarded }),
            Job::new(progress).with_progress(),
        ],
        vec![
            vec![Route::new(1, 0), Route::new(2, 0)],
            vec![Route::new(3, 0)],
            vec![Route::new(3, 1)],
            vec![],
        ],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap()
}

fn assert_ordered_progress(events: &Arc<Mutex<Vec<ProgressRecord>>>) {
    let events = events.lock().unwrap();
    let frames = 2;
    assert_eq!(events.len(), PROGRESS_REVISIONS * frames);
    let mut branches = Vec::new();
    for revision in 0..PROGRESS_REVISIONS {
        let offset = revision * frames;
        let branch = events[offset].0;
        assert!(!branches.contains(&branch));
        branches.push(branch);
        assert_eq!(events[offset], (branch, "begin"));
        assert_eq!(events[offset + 1], (branch, "end"));
    }
}

struct PanicOnData;

impl Action<u64> for PanicOnData {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |_, _| panic!("action bug"));
    }
}

struct InstrumentedFailure;

impl Action<u64> for InstrumentedFailure {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |_, emit| {
            emit.emit(zrx_diagnostic::warning!("invalid value"));
            emit.mark("validated");
            Err(anyhow::anyhow!("rejected value").into())
        });
    }
}

struct CurrentFailure;

impl Action<u64> for CurrentFailure {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) if *value.as_ref() <= 1 => {
                    let value = *value.as_ref();
                    emit.reject(
                        key,
                        anyhow::anyhow!("rejected value {value}").into(),
                    );
                }
                Change::Insert(key, _) | Change::Remove(key) => {
                    emit.resolve(key);
                }
            }
            Ok(())
        });
    }
}

#[derive(Clone)]
struct ReplicableCurrentFailure;

impl Action<u64> for ReplicableCurrentFailure {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            if let Change::Insert(key, value) = change {
                if *value.as_ref() == 0 {
                    emit.reject(key, anyhow::anyhow!("rejected").into());
                } else {
                    emit.resolve(key);
                }
            }
            Ok(())
        });
    }
}

struct BlockingPass {
    started: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

struct BlockingFailure {
    started: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Action<u64> for BlockingFailure {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |_, _| {
            self.started.send(()).unwrap();
            self.release.recv().unwrap();
            Err(anyhow::anyhow!("rejected after progress arrived").into())
        });
    }
}

#[derive(Clone, Copy)]
enum FailProgressAt {
    Begin,
    Abort,
}

struct FailProgress {
    at: FailProgressAt,
    failed: bool,
}

impl Action<u64> for FailProgress {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            let selected = matches!(
                (&self.at, &event),
                (FailProgressAt::Begin, Event::Progress(ProgressEvent::Begin),)
                    | (
                        FailProgressAt::Abort,
                        Event::Progress(ProgressEvent::Abort),
                    )
            );
            if selected && !self.failed {
                self.failed = true;
                return Err(anyhow::anyhow!("rejected progress event").into());
            }
            Ok(())
        });
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ProgressObservation {
    Begin,
    End,
    Abort,
}

struct RecordProgressStatus(Arc<Mutex<Vec<ProgressObservation>>>);

impl Action<u64> for RecordProgressStatus {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, _| {
            let observation = match event {
                Event::Progress(ProgressEvent::Begin) => {
                    ProgressObservation::Begin
                }
                Event::Progress(ProgressEvent::End) => ProgressObservation::End,
                Event::Progress(ProgressEvent::Abort) => {
                    ProgressObservation::Abort
                }
                Event::Wake { .. } => {
                    unreachable!("progress-only test action received a wake")
                }
            };
            self.0.lock().unwrap().push(observation);
            Ok(())
        });
    }
}

fn failing_progress_program(
    at: FailProgressAt, observations: Arc<Mutex<Vec<ProgressObservation>>>,
) -> Plan<u64> {
    Plan::builder(
        vec![
            Job::new(FailProgress { at, failed: false }).with_progress(),
            Job::new(RecordProgressStatus(observations)).with_progress(),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap()
}

struct VariadicPass;

impl Action<u64> for VariadicPass {
    type Inputs = Vec<u64>;
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs, output, events, .. } = context;
        for input in inputs {
            input.for_each(output, |change, emit| {
                match change {
                    Change::Insert(key, value) => {
                        emit.insert(key, value.into_owned());
                    }
                    Change::Remove(key) => emit.remove(key),
                }
                Ok(())
            });
        }
        events.for_each(output, |_, _| Ok(()));
    }
}

fn progress_lane_program(
    lane: usize, subscriber: usize,
    observations: Arc<Mutex<Vec<ProgressObservation>>>,
) -> Result<Plan<u64>, PlanError> {
    let progress = |job: Job<u64>, node| {
        if node == subscriber {
            job.with_progress()
        } else {
            job
        }
    };
    Plan::builder(
        vec![
            progress(Job::new(Pass), 0),
            progress(Job::new(VariadicPass), 1),
            progress(Job::new(RecordProgressStatus(observations)), 2),
        ],
        vec![
            vec![Route::new(1, 0), Route::new(1, lane)],
            vec![Route::new(2, 0)],
            vec![],
        ],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
}

struct BlockingFirstPass {
    blocked: bool,
    started: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Action<u64> for BlockingFirstPass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        if !self.blocked {
            self.blocked = true;
            self.started.send(()).unwrap();
            self.release.recv().unwrap();
        }
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
    }
}

struct CountPass {
    calls: Arc<Mutex<usize>>,
}

#[derive(Clone)]
struct ReplicableCountPass {
    calls: Arc<Mutex<usize>>,
}

struct InvocationOrder {
    tag: u64,
    order: Arc<Mutex<Vec<u64>>>,
}

#[derive(Clone)]
struct ReplicableGatePass {
    started: mpsc::Sender<u64>,
    gate: Arc<(Mutex<bool>, Condvar)>,
    blocked: Option<u64>,
    wake: Option<Instant>,
    events: Arc<Mutex<Vec<&'static str>>>,
}

#[derive(Clone)]
struct ReplicableWakeOrder {
    started: mpsc::Sender<u64>,
    gate: Arc<(Mutex<bool>, Condvar)>,
}

impl Action<u64> for ReplicableWakeOrder {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            if let Change::Insert(key, _) = change {
                self.started.send(key).unwrap();
                if key == 1 {
                    let (lock, ready) = &*self.gate;
                    let guard = lock.lock().unwrap();
                    drop(
                        ready.wait_while(guard, |released| !*released).unwrap(),
                    );
                    emit.wake(Wake::at(
                        WakeKey::new(1),
                        Instant::now() + Duration::from_secs(60),
                    ));
                } else {
                    emit.wake(Wake::clear(WakeKey::new(1)));
                }
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

impl Action<u64> for ReplicableGatePass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    if key == 0 {
                        emit.wake(Wake::at(
                            WakeKey::new(1),
                            self.wake.expect("wake deadline is configured"),
                        ));
                    } else {
                        self.started.send(key).unwrap();
                    }
                    if key != 0
                        && self.blocked.is_none_or(|blocked| blocked == key)
                    {
                        let (lock, ready) = &*self.gate;
                        let guard = lock.lock().unwrap();
                        drop(
                            ready
                                .wait_while(guard, |released| !*released)
                                .unwrap(),
                        );
                    }
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            assert!(matches!(event, Event::Wake { .. }));
            self.events.lock().unwrap().push("wake");
            Ok(())
        });
    }
}

fn release(gate: &Arc<(Mutex<bool>, Condvar)>) {
    let (lock, ready) = &**gate;
    *lock.lock().unwrap() = true;
    ready.notify_all();
}

impl Action<u64> for InvocationOrder {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        self.order.lock().unwrap().push(self.tag);
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |_, _| Ok(()));
    }
}

impl Action<u64> for CountPass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        *self.calls.lock().unwrap() += 1;
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
    }
}

impl Action<u64> for ReplicableCountPass {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        *self.calls.lock().unwrap() += 1;
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |_, _| Ok(()));
    }
}

#[derive(Clone, Copy)]
enum Parallelism {
    Adaptive,
    Bounded(NonZeroUsize),
}

#[derive(Clone)]
struct Concurrent<A> {
    action: A,
    parallelism: Parallelism,
}

impl<A> Action<u64> for Concurrent<A>
where
    A: Action<u64> + Clone,
{
    type Inputs = A::Inputs;
    type Output = A::Output;

    fn concurrency(&self) -> Concurrency<Self> {
        match self.parallelism {
            Parallelism::Adaptive => Concurrency::adaptive(),
            Parallelism::Bounded(maximum) => Concurrency::bounded(maximum),
        }
    }

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            revision,
            inputs,
            output,
            events,
        } = context;
        self.action.execute(Context {
            revision,
            inputs,
            output,
            events,
        });
    }
}

fn concurrent<A>(action: A, parallelism: Parallelism) -> Job<u64>
where
    A: Action<u64> + Clone,
{
    Job::new(Concurrent { action, parallelism })
}

impl Action<u64> for BlockingPass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        self.started.send(()).unwrap();
        self.release.recv().unwrap();
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
    }
}

struct CollectU64(Arc<Mutex<Vec<(u64, u64)>>>);

impl Action<u64> for CollectU64 {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, _| {
            if let Change::Insert(key, value) = change {
                self.0.lock().unwrap().push((key, *value.as_ref()));
            }
            Ok(())
        });
    }
}

struct LaneOrder(Arc<Mutex<Vec<u64>>>);

impl Action<u64> for LaneOrder {
    type Inputs = (u64, u64);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: (left, right), output, ..
        } = context;
        left.for_each(output, |change, _| {
            if let Change::Insert(_, value) = change {
                self.0.lock().unwrap().push(*value.as_ref());
            }
            Ok(())
        });
        right.for_each(output, |change, _| {
            if let Change::Insert(_, value) = change {
                self.0.lock().unwrap().push(*value.as_ref());
            }
            Ok(())
        });
    }
}

struct DataWakeOrder(Arc<Mutex<Vec<&'static str>>>);

impl Action<u64> for DataWakeOrder {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            self.0.lock().unwrap().push("data");
            if let Change::Insert(key, _) = change {
                emit.wake(Wake::at(WakeKey::new(key), Instant::now()));
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            assert!(matches!(event, Event::Wake { .. }));
            self.0.lock().unwrap().push("wake");
            Ok(())
        });
    }
}

struct FutureWakePass {
    deadline: Instant,
}

impl Action<u64> for FutureWakePass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                    emit.wake(Wake::at(WakeKey::new(key), self.deadline));
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

struct ObserveWakeDeadline {
    deadline: Instant,
    observed: Arc<Mutex<Option<Instant>>>,
}

impl Action<u64> for ObserveWakeDeadline {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            if let Change::Insert(key, _) = change {
                emit.wake(Wake::at(WakeKey::new(key), self.deadline));
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            let Event::Wake { deadline, .. } = event else {
                panic!("unexpected event")
            };
            *self.observed.lock().unwrap() = Some(deadline);
            Ok(())
        });
    }
}

#[test]
fn external_input_routes_data_and_settles() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(Pass), Job::new(CollectU64(Arc::clone(&collected)))],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 7, 9)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();
    assert_eq!(*collected.lock().unwrap(), [(7, 9)]);
    assert_complete(report.settlements());
    let tick = runtime.tick();
    assert!(!tick.progressed());
    assert!(tick.into_report().is_empty());
}

#[test]
fn immediate_runtime_imports_submitted_completion_on_the_next_tick() {
    let calls = Arc::new(Mutex::new(0));
    let program = Plan::builder(
        vec![Job::new(CountPass { calls: Arc::clone(&calls) })],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, Immediate::new());
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10)]))
        .unwrap();

    let tick = runtime.tick();
    assert!(tick.progressed());
    assert_eq!(*calls.lock().unwrap(), 1);
    assert!(tick.into_report().is_empty());
    assert!(runtime.tick().progressed());
    assert!(!runtime.tick().progressed());

    runtime.seal(revision).unwrap();
    assert_complete(runtime.tick().into_report().settlements());
}

#[test]
fn executor_rejection_retains_the_complete_dispatch_until_retry() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(Pass), Job::new(CollectU64(Arc::clone(&collected)))],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, RejectOnce::default());
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 7, 9)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(*collected.lock().unwrap(), [(7, 9)]);
    assert_complete(report.settlements());
}

#[test]
#[should_panic(expected = "action bug")]
fn worker_action_panic_resumes_on_the_scheduler_thread() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let program = Plan::builder(vec![Job::new(PanicOnData)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(
            INPUT_A,
            Route::new(0, 0),
        )])
        .build()
        .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(1));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 1)]))
        .unwrap();
    let _ = runtime.run_until_idle();
}

#[test]
fn bounded_session_retains_later_root_batches_until_capacity_returns() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(CollectU64(Arc::clone(&collected)))],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::new(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10)]))
        .unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let _ = runtime.run_until_idle();
    assert_eq!(*collected.lock().unwrap(), [(1, 10), (2, 20)]);
}

#[test]
fn one_full_fanout_branch_prevents_action_execution_and_partial_delivery() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let (release, wait_for_release) = mpsc::channel();
    let calls = Arc::new(Mutex::new(0));
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![
            Job::new(CountPass { calls: Arc::clone(&calls) }),
            Job::new(CollectU64(Arc::clone(&collected))),
            Job::new(BlockingFirstPass {
                blocked: false,
                started,
                release: wait_for_release,
            }),
        ],
        vec![vec![Route::new(1, 0), Route::new(2, 0)], vec![], vec![]],
    )
    .inputs(vec![
        InputBinding::new::<u64, u64>(INPUT_A, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(INPUT_B, Route::new(2, 0)),
    ])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(2));

    let pressure = runtime.begin(INPUT_B).unwrap();

    runtime
        .ingress(pressure, Batch::new(vec![item(1, 1, 10)]))
        .unwrap();
    assert!(runtime.tick().progressed());
    waiting.recv().unwrap();

    runtime
        .ingress(pressure, Batch::new(vec![item(2, 2, 20)]))
        .unwrap();

    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(3, 3, 30)]))
        .unwrap();
    let _ = runtime.tick();
    assert_eq!(*calls.lock().unwrap(), 0);
    assert!(collected.lock().unwrap().is_empty());

    release.send(()).unwrap();
    runtime.seal(pressure).unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();
    assert_eq!(*calls.lock().unwrap(), 1);
    assert_eq!(*collected.lock().unwrap(), [(3, 30)]);
    assert_eq!(report.settlements().len(), 2);
}

#[test]
fn due_wakes_remain_schedulable_without_busy_deadlines() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(DataWakeOrder(Arc::clone(&order)))],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::new(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10), item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    assert!(runtime.tick().progressed());
    assert!(runtime.tick().progressed());
    assert!(runtime.tick().progressed());
    {
        let mut select = Select::new();
        assert_eq!(runtime.register(&mut select).deadline(), None);
    }
    let report = runtime.run_until_idle();
    assert_eq!(*order.lock().unwrap(), ["data", "data", "wake", "wake"]);
    assert_complete(report.settlements());
}

#[test]
fn due_wake_exposes_the_requested_deadline() {
    let deadline = Instant::now();
    let observed = Arc::new(Mutex::new(None));
    let program = Plan::builder(
        vec![Job::new(ObserveWakeDeadline {
            deadline,
            observed: Arc::clone(&observed),
        })],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 1)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(*observed.lock().unwrap(), Some(deadline));
    assert_complete(report.settlements());
}

#[test]
fn one_source_has_at_most_one_open_revision() {
    let program = Plan::builder(vec![Job::new(Pass)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(
            INPUT_A,
            Route::new(0, 0),
        )])
        .build()
        .unwrap();
    let mut runtime = runtime(program);
    let first = runtime.begin(INPUT_A).unwrap();
    assert!(matches!(
        runtime.begin(INPUT_A),
        Err(RuntimeError::Session(SessionError::Installed(INPUT_A)))
    ));
    runtime.seal(first).unwrap();
    let second = runtime.begin(INPUT_A).unwrap();
    runtime.seal(second).unwrap();
}

#[test]
fn invocation_report_preserves_failure_and_instrumentation_attribution() {
    let program =
        Plan::builder(vec![Job::new(InstrumentedFailure)], vec![vec![]])
            .inputs(vec![InputBinding::new::<u64, u64>(
                INPUT_A,
                Route::new(0, 0),
            )])
            .build()
            .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(9, 4, 40)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let report = runtime.run_until_idle();
    let [invocation] = report.invocations() else {
        panic!("one nonempty invocation report expected")
    };
    assert_eq!(invocation.node, 0);
    assert_eq!(invocation.outcomes.failures().len(), 1);
    let [diagnostic, marker] = invocation.instrumentation.records() else {
        panic!("diagnostic and marker expected")
    };
    assert!(
        matches!(diagnostic, Record::Diagnostic(value) if value.message == "invalid value")
    );
    assert!(
        matches!(marker, Record::Annotation(value) if value.name() == "validated")
    );
}

#[test]
fn current_error_is_upserted_until_the_same_evaluation_resolves() {
    let program = Plan::builder(vec![Job::new(CurrentFailure)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(
            INPUT_A,
            Route::new(0, 0),
        )])
        .build()
        .unwrap();
    let mut runtime = runtime(program);

    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![item(1, 4, 0)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());
    assert_eq!(runtime.errors().len(), 1);
    assert_eq!(runtime.errors()[0].node(), 0);
    assert_eq!(runtime.errors()[0].key(), &4);
    assert_eq!(runtime.errors()[0].error().to_string(), "rejected value 0");

    let revision = runtime.begin(INPUT_A).unwrap();
    runtime.seal(revision).unwrap();
    assert_complete(runtime.run_until_idle().settlements());
    assert_eq!(runtime.errors().len(), 1);

    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![item(2, 4, 1)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    assert_complete(runtime.run_until_idle().settlements());
    assert_eq!(runtime.errors().len(), 1);
    assert_eq!(runtime.errors()[0].error().to_string(), "rejected value 1");

    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![item(3, 4, 2)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    assert_complete(runtime.run_until_idle().settlements());
    assert!(runtime.errors().is_empty());

    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![item(4, 4, 0)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    assert_complete(runtime.run_until_idle().settlements());
    assert_eq!(runtime.errors().len(), 1);

    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![Change::<u64, u64>::Remove(4)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    assert_complete(runtime.run_until_idle().settlements());
    assert!(runtime.errors().is_empty());
}

#[test]
fn shared_progress_abort_identifies_the_revision() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(RecordProgress(Arc::clone(&events))).with_progress()],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(7, 3, 30)]))
        .unwrap();
    let _ = runtime.run_until_idle();
    runtime.abort(revision).unwrap();
    let report = runtime.run_until_idle();

    let events = events.lock().unwrap();
    let branch = events[0].0;
    assert_eq!(*events, [(branch, "begin"), (branch, "abort")]);
    assert_aborted(report.settlements());
}

#[test]
fn abort_can_be_the_only_delivered_progress_event() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(RecordProgress(Arc::clone(&events))).with_progress()],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime.abort(revision).unwrap();
    let report = runtime.run_until_idle();

    let events = events.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].1, "abort");
    assert_aborted(report.settlements());
}

#[test]
fn immediate_converged_progress_preserves_source_order() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let (discarded, waiting) = mpsc::channel();
    let program = converged_progress_program(
        RecordConvergedProgress {
            events: Arc::clone(&events),
            started: None,
            release: None,
        },
        discarded,
    );
    let mut runtime = runtime(program);
    for revision in 0..PROGRESS_REVISIONS {
        let current = runtime.begin(INPUT_A).unwrap();
        for cause in 0..PROGRESS_ITEMS_PER_REVISION {
            let index = (revision * PROGRESS_ITEMS_PER_REVISION + cause) as u64;
            runtime
                .ingress(current, Batch::new(vec![item(index, index, index)]))
                .unwrap();
        }
        runtime.seal(current).unwrap();
    }
    let report = runtime.run_until_idle();

    let discarded: Vec<_> = waiting.try_iter().collect();
    assert_eq!(
        discarded.len(),
        PROGRESS_REVISIONS * PROGRESS_ITEMS_PER_REVISION * 2
    );
    assert_ordered_progress(&events);
    assert_eq!(report.settlements().len(), PROGRESS_REVISIONS);
    assert!(
        report
            .settlements()
            .iter()
            .all(|settlement| matches!(settlement, Settlement::Complete(_)))
    );
}

#[test]
fn busy_subscriber_observes_converged_progress_in_source_order() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let events = Arc::new(Mutex::new(Vec::new()));
    let (started, waiting_for_start) = mpsc::channel();
    let (release, waiting_for_release) = mpsc::channel();
    let (discarded, waiting_for_discard) = mpsc::channel();
    let program = converged_progress_program(
        RecordConvergedProgress {
            events: Arc::clone(&events),
            started: Some(started),
            release: Some(waiting_for_release),
        },
        discarded,
    );
    let strategy = SharedStrategy::new(WorkSharing::new(2));
    let observer = strategy.clone();
    let mut runtime = Runtime::with_strategy(program, strategy);
    let mut current = runtime.begin(INPUT_A).unwrap();

    let started = drive_until_received(&mut runtime, &waiting_for_start, 1);
    assert_eq!(started.len(), 1);
    for revision in 0..PROGRESS_REVISIONS {
        for cause in 0..PROGRESS_ITEMS_PER_REVISION {
            let index = (revision * PROGRESS_ITEMS_PER_REVISION + cause) as u64;
            runtime
                .ingress(current, Batch::new(vec![item(index, index, index)]))
                .unwrap();
        }
        runtime.seal(current).unwrap();
        if revision + 1 != PROGRESS_REVISIONS {
            current = runtime.begin(INPUT_A).unwrap();
        }
    }

    let discarded = drive_until_received(
        &mut runtime,
        &waiting_for_discard,
        PROGRESS_REVISIONS * PROGRESS_ITEMS_PER_REVISION * 2,
    );
    assert_eq!(
        discarded.len(),
        PROGRESS_REVISIONS * PROGRESS_ITEMS_PER_REVISION * 2
    );
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let progressed = runtime.tick().progressed();
        if !progressed
            && observer.num_tasks_running() == 1
            && observer.num_tasks_pending() == 1
        {
            break;
        }
        thread::yield_now();
        assert!(Instant::now() < deadline);
    }
    assert!(events.lock().unwrap().is_empty());

    release.send(()).unwrap();
    let report = runtime.run_until_idle();

    assert_ordered_progress(&events);
    assert_eq!(report.settlements().len(), PROGRESS_REVISIONS);
    assert!(
        report
            .settlements()
            .iter()
            .all(|settlement| matches!(settlement, Settlement::Complete(_)))
    );
}

#[test]
fn progress_end_waits_for_preceding_failure_reconciliation() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let (started, waiting_for_start) = mpsc::channel();
    let (release, waiting_for_release) = mpsc::channel();
    let program = Plan::builder(
        vec![
            Job::new(BlockingFailure {
                started,
                release: waiting_for_release,
            }),
            Job::new(RecordProgressStatus(Arc::clone(&observations)))
                .with_progress(),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let strategy = SharedStrategy::new(WorkSharing::new(2));
    let observer = strategy.clone();
    let mut runtime = Runtime::with_strategy(program, strategy);
    let revision = runtime.begin(INPUT_A).unwrap();
    let begin = runtime.run_until_idle();
    assert!(begin.is_empty());

    runtime
        .ingress(revision, Batch::new(vec![item(0, 1, 10)]))
        .unwrap();
    let started = drive_until_received(&mut runtime, &waiting_for_start, 1);
    assert_eq!(started.len(), 1);
    runtime.seal(revision).unwrap();

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let progressed = runtime.tick().progressed();
        if !progressed
            && observer.num_tasks_running() == 1
            && observer.num_tasks_pending() == 1
        {
            break;
        }
        thread::yield_now();
        assert!(Instant::now() < deadline);
    }
    assert_eq!(*observations.lock().unwrap(), [ProgressObservation::Begin]);

    release.send(()).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(
        *observations.lock().unwrap(),
        [ProgressObservation::Begin, ProgressObservation::End]
    );
    assert_eq!(report.invocations().len(), 1);
    assert_complete(report.settlements());
}

#[test]
fn progress_begin_failure_is_branch_persistent() {
    let observations = Arc::new(Mutex::new(Vec::new()));
    let program = failing_progress_program(
        FailProgressAt::Begin,
        Arc::clone(&observations),
    );
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(0, 1, 10)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(
        *observations.lock().unwrap(),
        [ProgressObservation::Begin, ProgressObservation::End,]
    );
    assert_eq!(report.invocations().len(), 1);
    assert_eq!(report.invocations()[0].node, 0);
    assert_complete(report.settlements());
}

#[test]
fn progress_abort_failure_remains_terminal_and_diagnostic() {
    let observations = Arc::new(Mutex::new(Vec::new()));
    let program = failing_progress_program(
        FailProgressAt::Abort,
        Arc::clone(&observations),
    );
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();
    assert!(runtime.run_until_idle().is_empty());

    runtime.abort(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(
        *observations.lock().unwrap(),
        [ProgressObservation::Begin, ProgressObservation::Abort,]
    );
    assert_eq!(report.invocations().len(), 1);
    assert_eq!(report.invocations()[0].node, 0);
    assert_aborted(report.settlements());
}

#[test]
fn maximum_progress_lane_width_executes() {
    let observations = Arc::new(Mutex::new(Vec::new()));
    let program = progress_lane_program(63, 2, Arc::clone(&observations))
        .expect("64 progress lane positions are supported");
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(0, 1, 10)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(
        *observations.lock().unwrap(),
        [ProgressObservation::Begin, ProgressObservation::End,]
    );
    assert_complete(report.settlements());
}

#[test]
fn single_progress_lane_above_convergence_mask_executes() {
    let program = Plan::builder(
        vec![Job::new(VariadicPass).with_progress()],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 64),
    )])
    .build()
    .expect("a direct progress lane does not use convergence bits");
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(0, 1, 10)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_complete(report.settlements());
}

#[test]
fn construction_rejects_unsupported_subscriber_progress_lane() {
    let program = progress_lane_program(64, 1, Arc::default());

    assert!(matches!(
        program,
        Err(PlanError::Progress(ProgressError::Width {
            input: INPUT_A,
            node: 1,
            lane: 64,
        }))
    ));
}

#[test]
fn construction_rejects_unsupported_transparent_progress_lane() {
    let program = progress_lane_program(64, 2, Arc::default());

    assert!(matches!(
        program,
        Err(PlanError::Progress(ProgressError::Width {
            input: INPUT_A,
            node: 1,
            lane: 64,
        }))
    ));
}

#[test]
fn abort_preserves_output_from_an_already_dispatched_action() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![
            Job::new(BlockingPass { started, release: blocked }),
            Job::new(CollectU64(Arc::clone(&collected))),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(1));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 7, 9)]))
        .unwrap();
    assert!(runtime.tick().progressed());
    waiting.recv().unwrap();

    runtime.abort(revision).unwrap();
    release.send(()).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(*collected.lock().unwrap(), [(7, 9)]);
    assert_aborted(report.settlements());
}

#[test]
fn readiness_selection_does_not_consume_worker_completion() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let program = Plan::builder(
        vec![Job::new(BlockingPass { started, release: blocked })],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(1));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 7, 9)]))
        .unwrap();
    assert!(runtime.tick().progressed());
    waiting.recv().unwrap();
    release.send(()).unwrap();

    {
        let mut select = Select::new();
        let readiness = runtime.register(&mut select);
        let operation = select
            .ready_timeout(Duration::from_secs(1))
            .expect("worker completion became selectable");
        assert!(readiness.contains(operation));
        assert_eq!(readiness.deadline(), None);
    }

    assert!(runtime.tick().progressed());
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());
}

#[test]
fn readiness_captures_the_next_wake_deadline() {
    let deadline = Instant::now() + Duration::from_secs(60);
    let program = Plan::builder(
        vec![Job::new(FutureWakePass { deadline })],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 1)]))
        .unwrap();
    assert!(runtime.tick().progressed());

    let mut select = Select::new();
    let readiness = runtime.register(&mut select);
    assert_eq!(readiness.deadline(), Some(deadline));
}

#[test]
fn data_lanes_are_reconsidered_round_robin() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(LaneOrder(Arc::clone(&order)))],
        vec![vec![]],
    )
    .inputs(vec![
        InputBinding::new::<u64, u64>(INPUT_A, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(INPUT_B, Route::new(0, 1)),
    ])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let left = runtime.begin(INPUT_A).unwrap();
    let right = runtime.begin(INPUT_B).unwrap();

    runtime
        .ingress(left, Batch::new(vec![item(1, 1, 10)]))
        .unwrap();

    runtime
        .ingress(left, Batch::new(vec![item(2, 2, 20)]))
        .unwrap();

    runtime
        .ingress(right, Batch::new(vec![item(3, 3, 30)]))
        .unwrap();
    runtime.seal(left).unwrap();
    runtime.seal(right).unwrap();

    let _ = runtime.run_until_idle();
    assert_eq!(*order.lock().unwrap(), [10, 30, 20]);
}

#[test]
fn segment_slices_reenter_the_ready_queue_for_fair_scheduling() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![
            Job::new(InvocationOrder {
                tag: 1,
                order: Arc::clone(&order),
            }),
            Job::new(InvocationOrder {
                tag: 2,
                order: Arc::clone(&order),
            }),
        ],
        vec![vec![], vec![]],
    )
    .inputs(vec![
        InputBinding::new::<u64, u64>(INPUT_A, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(INPUT_B, Route::new(1, 0)),
    ])
    .build()
    .unwrap();
    let mut runtime =
        Runtime::with_strategy(program, WorkSharing::with_capacity(1, 2));
    let left = runtime.begin(INPUT_A).unwrap();
    let right = runtime.begin(INPUT_B).unwrap();

    runtime
        .ingress(
            left,
            Batch::new(
                (1..=1_025).map(|value| item(value, value, value)).collect(),
            ),
        )
        .unwrap();

    runtime
        .ingress(right, Batch::new(vec![item(4, 4, 40)]))
        .unwrap();
    runtime.seal(left).unwrap();
    runtime.seal(right).unwrap();

    let report = runtime.run_until_idle();
    assert_eq!(*order.lock().unwrap(), [1, 2, 1]);
    assert_eq!(report.settlements().len(), 2);
}

#[test]
fn slicing_preserves_values_and_revision_settlement() {
    let calls = Arc::new(Mutex::new(0));
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![
            Job::new(CountPass { calls: Arc::clone(&calls) }),
            Job::new(CollectU64(Arc::clone(&collected))),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::new(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(
            revision,
            Batch::new(
                (1..=1_025).map(|value| item(value, value, value)).collect(),
            ),
        )
        .unwrap();
    runtime.seal(revision).unwrap();

    let report = runtime.run_until_idle();
    assert_eq!(*calls.lock().unwrap(), 2);
    let collected = collected.lock().unwrap();
    assert_eq!(collected.len(), 1_025);
    assert_eq!(collected.first(), Some(&(1, 1)));
    assert_eq!(collected.last(), Some(&(1_025, 1_025)));
    assert_complete(report.settlements());
}

#[test]
fn replicable_actions_run_bounded_slices_concurrently() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let events = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![concurrent(
            ReplicableGatePass {
                started,
                gate: Arc::clone(&gate),
                blocked: None,
                wake: None,
                events,
            },
            Parallelism::Adaptive,
        )],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(2));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10), item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let mut keys = drive_until_received(&mut runtime, &waiting, 2);
    keys.sort_unstable();
    assert_eq!(keys, [1, 2]);

    release(&gate);
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());
}

#[test]
fn one_segment_keeps_its_initial_shard_quantum() {
    let calls = Arc::new(Mutex::new(0));
    let program = Plan::builder(
        vec![concurrent(
            ReplicableCountPass { calls: Arc::clone(&calls) },
            Parallelism::Adaptive,
        )],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(4));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(
            revision,
            Batch::new(
                (1..=256).map(|value| item(value, value, value)).collect(),
            ),
        )
        .unwrap();
    runtime.seal(revision).unwrap();

    let report = runtime.run_until_idle();
    assert_eq!(*calls.lock().unwrap(), 4);
    assert_complete(report.settlements());
}

#[test]
fn bounded_concurrency_caps_action_instances() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let program = Plan::builder(
        vec![concurrent(
            ReplicableGatePass {
                started,
                gate: Arc::clone(&gate),
                blocked: None,
                wake: None,
                events: Arc::new(Mutex::new(Vec::new())),
            },
            Parallelism::Bounded(NonZeroUsize::new(2).unwrap()),
        )],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(4));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(
            revision,
            Batch::new((1..=4).map(|key| item(key, key, key)).collect()),
        )
        .unwrap();
    runtime.seal(revision).unwrap();

    let mut keys = drive_until_received(&mut runtime, &waiting, 2);
    for _ in 0..32 {
        let _ = runtime.tick();
        thread::yield_now();
    }
    assert!(waiting.try_recv().is_err());

    release(&gate);
    let report = runtime.run_until_idle();
    keys.extend(waiting.try_iter());
    keys.sort_unstable();
    assert_eq!(keys, [1, 2, 3, 4]);
    assert_complete(report.settlements());
}

#[test]
fn nonreplicable_actions_keep_one_resident_execution() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let program = Plan::builder(
        vec![Job::new(BlockingPass { started, release: blocked })],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(2));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10), item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let _ = runtime.tick();
        if waiting.try_recv().is_ok() {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert!(!runtime.tick().progressed());
    assert!(waiting.try_recv().is_err());

    release.send(()).unwrap();
    release.send(()).unwrap();
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());
}

#[test]
fn replicated_results_reconcile_in_dispatch_order() {
    let (started, waiting) = mpsc::channel();
    let gate = Arc::new((Mutex::new(true), Condvar::new()));
    let events = Arc::new(Mutex::new(Vec::new()));
    let collected = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![
            concurrent(
                ReplicableGatePass {
                    started,
                    gate: Arc::clone(&gate),
                    blocked: Some(0),
                    wake: None,
                    events,
                },
                Parallelism::Adaptive,
            ),
            Job::new(CollectU64(Arc::clone(&collected))),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let queue = Queued::default();
    let mut runtime = Runtime::with_strategy(program, queue.clone());
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(
            revision,
            Batch::new(vec![item(1, 1, 10), item(2, 2, 20), item(3, 3, 30)]),
        )
        .unwrap();
    runtime.seal(revision).unwrap();

    while queue.len() < 2 {
        assert!(runtime.tick().progressed());
    }
    queue.execute(1);
    assert_eq!(waiting.recv().unwrap(), 3);
    assert!(runtime.tick().progressed());
    assert!(collected.lock().unwrap().is_empty());

    queue.execute(0);
    assert_eq!(waiting.recv().unwrap(), 1);
    let report = drain_queued(&mut runtime, &queue);
    assert_eq!(*collected.lock().unwrap(), [(1, 10), (2, 20), (3, 30)]);
    assert_complete(report.settlements());
}

#[test]
fn replicated_error_changes_reconcile_in_dispatch_order() {
    let program = Plan::builder(
        vec![concurrent(ReplicableCurrentFailure, Parallelism::Adaptive)],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let queue = Queued::default();
    let mut runtime = Runtime::with_strategy(program, queue.clone());
    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(
            revision,
            Batch::new(vec![item(1, 4, 0), item(2, 8, 8), item(3, 4, 2)]),
        )
        .unwrap();
    runtime.seal(revision).unwrap();

    while queue.len() < 2 {
        assert!(runtime.tick().progressed());
    }
    queue.execute(1);
    assert!(runtime.tick().progressed());
    assert!(runtime.errors().is_empty());

    queue.execute(0);
    let report = drain_queued(&mut runtime, &queue);
    assert_complete(report.settlements());
    assert_eq!(
        report
            .invocations()
            .iter()
            .map(|invocation| invocation.outcomes.error_count())
            .sum::<usize>(),
        1
    );
    assert!(runtime.errors().is_empty());
}

#[test]
fn replicated_wake_requests_reconcile_in_dispatch_order() {
    let (started, waiting) = mpsc::channel();
    let gate = Arc::new((Mutex::new(true), Condvar::new()));
    let program = Plan::builder(
        vec![concurrent(
            ReplicableWakeOrder {
                started,
                gate: Arc::clone(&gate),
            },
            Parallelism::Adaptive,
        )],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let queue = Queued::default();
    let mut runtime = Runtime::with_strategy(program, queue.clone());
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10), item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    while queue.len() < 2 {
        assert!(runtime.tick().progressed());
    }
    queue.execute(1);
    assert_eq!(waiting.recv().unwrap(), 2);
    assert!(runtime.tick().progressed());
    {
        let mut select = Select::new();
        assert_eq!(runtime.register(&mut select).deadline(), None);
    }

    queue.execute(0);
    assert_eq!(waiting.recv().unwrap(), 1);
    let report = drain_queued(&mut runtime, &queue);
    {
        let mut select = Select::new();
        assert_eq!(runtime.register(&mut select).deadline(), None);
    }
    assert_complete(report.settlements());
}

#[test]
fn events_wait_for_all_replicated_data_shards() {
    let _workers = BLOCKING_WORKERS.lock().unwrap();
    let (started, waiting) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let events = Arc::new(Mutex::new(Vec::new()));
    let deadline = Instant::now() + Duration::from_millis(20);
    let program = Plan::builder(
        vec![concurrent(
            ReplicableGatePass {
                started,
                gate: Arc::clone(&gate),
                blocked: None,
                wake: Some(deadline),
                events: Arc::clone(&events),
            },
            Parallelism::Adaptive,
        )],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(program, WorkSharing::new(2));
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(0, 0, 0)]))
        .unwrap();
    assert!(runtime.tick().progressed());
    {
        let mut select = Select::new();
        let readiness = runtime.register(&mut select);
        let operation = select
            .ready_timeout(Duration::from_secs(1))
            .expect("wake-producing invocation completed");
        assert!(readiness.contains(operation));
    }
    assert!(runtime.tick().progressed());

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10), item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let _ = drive_until_received(&mut runtime, &waiting, 2);
    thread::sleep(
        deadline.saturating_duration_since(Instant::now())
            + Duration::from_millis(1),
    );
    assert!(runtime.tick().progressed());
    assert!(!runtime.tick().progressed());
    assert!(events.lock().unwrap().is_empty());

    release(&gate);
    let report = runtime.run_until_idle();
    assert_eq!(*events.lock().unwrap(), ["wake"]);
    assert_complete(report.settlements());
}

#[test]
fn wake_and_data_classes_are_reconsidered_round_robin() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(DataWakeOrder(Arc::clone(&order)))],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 10)]))
        .unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(2, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let _ = runtime.run_until_idle();
    assert_eq!(*order.lock().unwrap(), ["data", "wake", "data", "wake"]);
}

#[test]
fn replacing_a_wake_releases_the_prior_revision_authority() {
    let deadline = Instant::now() + Duration::from_secs(60);
    let program = Plan::builder(
        vec![Job::new(FutureWakePass { deadline })],
        vec![vec![]],
    )
    .inputs(vec![
        InputBinding::new::<u64, u64>(INPUT_A, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(INPUT_B, Route::new(0, 0)),
    ])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let first = runtime.begin(INPUT_A).unwrap();
    let second = runtime.begin(INPUT_B).unwrap();

    runtime
        .ingress(first, Batch::new(vec![item(1, 1, 10)]))
        .unwrap();
    runtime.seal(first).unwrap();
    assert!(runtime.run_until_idle().settlements().is_empty());

    runtime
        .ingress(second, Batch::new(vec![item(2, 1, 20)]))
        .unwrap();
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());

    runtime.abort(second).unwrap();
    let report = runtime.run_until_idle();
    assert_aborted(report.settlements());
}

struct WakeBeforeEnd(Arc<Mutex<bool>>);

struct ConvergedWake {
    count: usize,
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Action<u64> for ConvergedWake {
    type Inputs = (u64, u64);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: (left, right),
            output,
            events,
            ..
        } = context;
        left.for_each(output, |_, _| Ok(()));
        right.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, emit| {
            let name = match event {
                Event::Progress(ProgressEvent::Begin) => {
                    emit.wake(Wake::at(WakeKey::new(1), Instant::now()));
                    "begin"
                }
                Event::Wake { .. } => {
                    self.count += 1;
                    if self.count == 1 {
                        emit.wake(Wake::at(WakeKey::new(1), Instant::now()));
                        "wake1"
                    } else {
                        "wake2"
                    }
                }
                Event::Progress(ProgressEvent::End) => "end",
                Event::Progress(ProgressEvent::Abort) => "abort",
            };
            self.events.lock().unwrap().push(name);
            Ok(())
        });
    }
}

#[test]
fn converged_end_rechecks_wakes_created_by_an_in_flight_begin() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let queue = Queued::default();
    let plan = Plan::builder(
        vec![
            Job::<u64>::forward::<u64>(),
            Job::new(ConvergedWake {
                count: 0,
                events: Arc::clone(&events),
            })
            .with_progress(),
        ],
        vec![vec![Route::new(1, 0), Route::new(1, 1)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = Runtime::with_strategy(plan, queue.clone());
    let revision = runtime.begin(INPUT_A).unwrap();
    runtime.seal(revision).unwrap();
    let report = drain_delayed(&mut runtime, &queue);
    assert_complete(report.settlements());
    assert_eq!(*events.lock().unwrap(), ["begin", "wake1", "wake2", "end"]);
}

struct RearmingWake {
    count: usize,
    events: Arc<Mutex<Vec<&'static str>>>,
}

impl Action<u64> for RearmingWake {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs, output, events, .. } = context;
        inputs.for_each(output, |_, emit| {
            self.events.lock().unwrap().push("input");
            emit.wake(Wake::at(WakeKey::new(1), Instant::now()));
            Ok(())
        });
        events.for_each(output, |event, emit| {
            assert!(matches!(event, Event::Wake { .. }));
            self.count += 1;
            if self.count == 1 {
                self.events.lock().unwrap().push("wake1");
                emit.wake(Wake::at(WakeKey::new(1), Instant::now()));
            } else {
                self.events.lock().unwrap().push("wake2");
                emit.insert(1, 42);
            }
            Ok(())
        });
    }
}

struct ObserveWakeEnd(Arc<Mutex<Vec<&'static str>>>);

impl Action<u64> for ObserveWakeEnd {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs, output, events, .. } = context;
        inputs.for_each(output, |_, _| {
            self.0.lock().unwrap().push("output");
            Ok(())
        });
        events.for_each(output, |event, _| {
            match event {
                Event::Progress(ProgressEvent::End) => {
                    self.0.lock().unwrap().push("end");
                }
                Event::Progress(ProgressEvent::Abort) => {
                    self.0.lock().unwrap().push("abort");
                }
                _ => {}
            }
            Ok(())
        });
    }
}

fn drain_delayed(runtime: &mut Runtime<u64, Queued>, queue: &Queued) -> Report {
    let mut report = Report::default();
    for _ in 0..100 {
        loop {
            let tick = runtime.tick();
            let progressed = tick.progressed();
            report.append(tick.into_report());
            if !progressed {
                break;
            }
        }
        if !queue.execute_next() {
            return report;
        }
    }
    panic!("delayed execution did not drain");
}

fn rearming_plan(events: &Arc<Mutex<Vec<&'static str>>>) -> Plan<u64> {
    Plan::builder(
        vec![
            Job::new(RearmingWake {
                count: 0,
                events: Arc::clone(events),
            }),
            Job::new(ObserveWakeEnd(Arc::clone(events))).with_progress(),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap()
}

#[test]
fn end_waits_for_reconciliation_and_rearmed_wakes() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let queue = Queued::default();
    let mut runtime =
        Runtime::with_strategy(rearming_plan(&events), queue.clone());
    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 1)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = drain_delayed(&mut runtime, &queue);
    assert_complete(report.settlements());
    assert_eq!(
        *events.lock().unwrap(),
        ["input", "wake1", "wake2", "output", "end"]
    );
}

#[test]
fn aborting_a_dispatched_wake_discards_its_rearm_and_settles() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let queue = Queued::default();
    let mut runtime =
        Runtime::with_strategy(rearming_plan(&events), queue.clone());
    let revision = runtime.begin(INPUT_A).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 1)]))
        .unwrap();
    // Run the input callback, then let orchestration dispatch its wake without
    // executing the queued wake callback.
    for _ in 0..20 {
        while runtime.tick().progressed() {}
        if !events.lock().unwrap().is_empty() {
            break;
        }
        assert!(queue.execute_next());
    }
    assert_eq!(*events.lock().unwrap(), ["input"]);
    assert_ne!(queue.len(), 0);
    runtime.abort(revision).unwrap();
    let report = drain_delayed(&mut runtime, &queue);
    assert_aborted(report.settlements());
    assert_eq!(*events.lock().unwrap(), ["input", "wake1", "abort"]);
    let mut select = Select::new();
    let readiness = runtime.register(&mut select);
    assert!(!readiness.pending());
    assert_eq!(readiness.deadline(), None);
}

impl Action<u64> for WakeBeforeEnd {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            if let Change::Insert(key, value) = change {
                emit.insert(key, value.into_owned());
                emit.wake(Wake::at(WakeKey::new(1), Instant::now()));
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            assert!(matches!(event, Event::Wake { .. }));
            *self.0.lock().unwrap() = true;
            Ok(())
        });
    }
}

struct WakeOnBegin(Arc<Mutex<Vec<&'static str>>>);

impl Action<u64> for WakeOnBegin {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, emit| {
            let name = match event {
                Event::Progress(ProgressEvent::Begin) => {
                    emit.wake(Wake::at(WakeKey::new(1), Instant::now()));
                    "begin"
                }
                Event::Wake { .. } => "wake",
                Event::Progress(ProgressEvent::End) => "end",
                Event::Progress(ProgressEvent::Abort) => "abort",
            };
            self.0.lock().unwrap().push(name);
            Ok(())
        });
    }
}

struct ScheduleThenClearWake(Arc<Mutex<Vec<&'static str>>>);

impl Action<u64> for ScheduleThenClearWake {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            let Change::Insert(key, _) = change else {
                return Ok(());
            };
            if key == 1 {
                emit.wake(Wake::at(
                    WakeKey::new(1),
                    Instant::now() + Duration::from_secs(60),
                ));
            } else {
                emit.wake(Wake::clear(WakeKey::new(1)));
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            let name = match event {
                Event::Progress(ProgressEvent::Begin) => "begin",
                Event::Progress(ProgressEvent::End) => "end",
                Event::Progress(ProgressEvent::Abort) => "abort",
                Event::Wake { .. } => "wake",
            };
            self.0.lock().unwrap().push(name);
            Ok(())
        });
    }
}

type BranchEvent = (RevisionId, &'static str);

struct RevisionScopedWake {
    deadline: Instant,
    events: Arc<Mutex<Vec<BranchEvent>>>,
}

impl Action<u64> for RevisionScopedWake {
    type Inputs = (u64, u64);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            revision,
            inputs: (left, right),
            output,
            events,
        } = context;
        left.for_each(output, |_, emit| {
            emit.wake(Wake::at(WakeKey::new(1), self.deadline));
            Ok(())
        });
        right.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, _| {
            let name = match event {
                Event::Progress(ProgressEvent::Begin) => "begin",
                Event::Progress(ProgressEvent::End) => "end",
                Event::Progress(ProgressEvent::Abort) => "abort",
                Event::Wake { .. } => "wake",
            };
            self.events.lock().unwrap().push((revision, name));
            Ok(())
        });
    }
}

struct AssertEnd(Arc<Mutex<bool>>);

impl Action<u64> for AssertEnd {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, _| {
            if matches!(event, Event::Progress(ProgressEvent::End)) {
                assert!(*self.0.lock().unwrap(), "end overtook relevant wake");
            }
            Ok(())
        });
    }
}

#[test]
fn end_does_not_overtake_a_wake_scheduled_by_preceding_path_work() {
    let fired = Arc::new(Mutex::new(false));
    let program = Plan::builder(
        vec![
            Job::new(WakeBeforeEnd(Arc::clone(&fired))),
            Job::new(AssertEnd(Arc::clone(&fired))).with_progress(),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1, 1, 1)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();
    assert!(*fired.lock().unwrap());
    assert_complete(report.settlements());
}

#[test]
fn end_waits_for_a_wake_scheduled_by_begin() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![Job::new(WakeOnBegin(Arc::clone(&events))).with_progress()],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(*events.lock().unwrap(), ["begin", "wake", "end"]);
    assert_complete(report.settlements());
}

#[test]
fn clearing_a_wake_releases_end_without_firing_it() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let program = Plan::builder(
        vec![
            Job::new(ScheduleThenClearWake(Arc::clone(&events)))
                .with_progress(),
        ],
        vec![vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(
        INPUT_A,
        Route::new(0, 0),
    )])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let revision = runtime.begin(INPUT_A).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(0, 1, 10)]))
        .unwrap();
    assert!(runtime.run_until_idle().is_empty());
    runtime
        .ingress(revision, Batch::new(vec![item(1, 2, 20)]))
        .unwrap();
    runtime.seal(revision).unwrap();
    let report = runtime.run_until_idle();

    assert_eq!(*events.lock().unwrap(), ["begin", "end"]);
    assert_complete(report.settlements());
}

#[test]
fn a_wake_holds_only_its_own_revision_end() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let deadline = Instant::now() + Duration::from_millis(20);
    let program = Plan::builder(
        vec![
            Job::new(RevisionScopedWake {
                deadline,
                events: Arc::clone(&events),
            })
            .with_progress(),
        ],
        vec![vec![]],
    )
    .inputs(vec![
        InputBinding::new::<u64, u64>(INPUT_A, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(INPUT_B, Route::new(0, 1)),
    ])
    .build()
    .unwrap();
    let mut runtime = runtime(program);
    let first = runtime.begin(INPUT_A).unwrap();
    let second = runtime.begin(INPUT_B).unwrap();
    assert!(runtime.run_until_idle().is_empty());
    let branches: Vec<_> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|(cause, name)| (*name == "begin").then_some(*cause))
        .collect();
    assert_eq!(branches.len(), 2);

    runtime
        .ingress(first, Batch::new(vec![item(0, 1, 10)]))
        .unwrap();
    assert!(runtime.run_until_idle().is_empty());
    runtime.seal(first).unwrap();
    assert!(runtime.run_until_idle().settlements().is_empty());

    runtime.seal(second).unwrap();
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());
    let ends: Vec<_> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|(cause, name)| (*name == "end").then_some(*cause))
        .collect();
    assert_eq!(ends, [branches[1]]);

    thread::sleep(
        deadline.saturating_duration_since(Instant::now())
            + Duration::from_millis(1),
    );
    let report = runtime.run_until_idle();
    assert_complete(report.settlements());
    let ends: Vec<_> = events
        .lock()
        .unwrap()
        .iter()
        .filter_map(|(cause, name)| (*name == "end").then_some(*cause))
        .collect();
    assert_eq!(ends, [branches[1], branches[0]]);
}

#[test]
fn construction_rejects_duplicate_data_routes() {
    let program = Plan::builder(
        vec![Job::new(Pass), Job::new(Pass)],
        vec![vec![Route::new(1, 0), Route::new(1, 0)], vec![]],
    )
    .build();
    assert!(matches!(
        program,
        Err(PlanError::Route(RouteError::Duplicate {
            from: 0,
            target: 1,
            lane: 0,
        }))
    ));
}
