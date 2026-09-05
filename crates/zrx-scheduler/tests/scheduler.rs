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

//! Scheduler integration tests.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use zrx_executor::Strategy;
use zrx_executor::strategy::{Immediate, WorkSharing};
use zrx_executor::task::Task;

use zrx_scheduler::Change;
use zrx_scheduler::action::control::Event;
use zrx_scheduler::action::{Action, Context, Job};
use zrx_scheduler::plan::{InputBinding, InputId, Plan, Route};
use zrx_scheduler::{Error as SchedulerError, Scheduler, Settlement};

const INPUT: InputId = InputId::new(1);

#[derive(Clone, Copy)]
enum Terminal {
    Open,
    Sealed,
    Aborted,
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
struct ObservedShutdown {
    strategy: WorkSharing,
    submissions: Arc<AtomicUsize>,
    panics: Arc<AtomicUsize>,
    waiting: mpsc::Sender<()>,
}

impl Strategy for ObservedShutdown {
    fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
        let panics = Arc::clone(&self.panics);
        self.strategy.submit(Box::new(move || {
            if std::panic::catch_unwind(|| task.execute().execute()).is_err() {
                panics.fetch_add(1, Ordering::Relaxed);
            }
        }))?;
        self.submissions.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn num_workers(&self) -> usize {
        self.strategy.num_workers()
    }

    fn num_tasks_running(&self) -> usize {
        self.strategy.num_tasks_running()
    }

    fn num_tasks_pending(&self) -> usize {
        // The scheduler inspects placement locally while ticking. A strategy
        // task-count inspection here identifies executor shutdown waiting.
        let _ = self.waiting.send(());
        self.strategy.num_tasks_pending()
    }

    fn capacity(&self) -> usize {
        self.strategy.capacity()
    }
}

struct Join;

impl Action<u64> for Join {
    type Inputs = (u64, u64);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: (left, right),
            output,
            events,
            ..
        } = context;
        left.for_each(output, |_, _| Ok(()));
        right.for_each(output, |_, _| Ok(()));
        events.for_each(output, |_, _| Ok(()));
    }
}

#[test]
fn rejected_only_work_retries_after_executor_capacity_returns() {
    use crossbeam::channel::Select;
    use std::time::Instant;
    use zrx_executor::{Error, Strategy, task::Task};

    for retire in [false, true] {
        let strategy = WorkSharing::with_capacity(1, 0);
        let (release, waiting) = mpsc::channel();
        let mut task: Box<dyn Task> = Box::new(move || {
            let _ = waiting.recv();
        });
        let limit = Instant::now() + Duration::from_secs(2);
        loop {
            match strategy.submit(task) {
                Ok(()) => break,
                Err(Error::Submit(returned)) => {
                    task = returned;
                    assert!(Instant::now() < limit, "worker did not start");
                    thread::yield_now();
                }
                Err(error) => panic!("{error}"),
            }
        }
        let order = Arc::new(Mutex::new(Vec::new()));
        let mut scheduler = Scheduler::new(strategy);
        let plan = scheduler.attach(plan(Record {
            marker: 7,
            order: Arc::clone(&order),
        }));
        let session = scheduler
            .attachment(plan)
            .unwrap()
            .session::<u64>(INPUT)
            .unwrap();
        let mut writer = session.begin().unwrap();
        writer.insert(1, 42).unwrap();
        let session = writer.seal().unwrap();
        while let Some(tick) = scheduler.tick() {
            assert!(tick.into_report().settlements().is_empty());
        }
        if retire {
            scheduler.attachment(plan).unwrap().detach();
        }

        // No callback belongs to this runtime yet, so only its retry deadline
        // can wake orchestration. Keep the session connected during the test.
        assert_readiness(&scheduler);
        let first = {
            let mut select = Select::new();
            let readiness = scheduler.register(&mut select);
            assert!(readiness.pending());
            let deadline = readiness
                .deadline()
                .expect("retained task has retry readiness");
            assert!(
                select
                    .ready_timeout(
                        deadline.saturating_duration_since(Instant::now())
                    )
                    .is_err()
            );
            deadline
        };
        assert!(scheduler.tick().is_none());
        assert_readiness(&scheduler);
        {
            let mut select = Select::new();
            let readiness = scheduler.register(&mut select);
            assert!(readiness.deadline().unwrap() > first);
        }
        release.send(()).unwrap();
        let mut settlements = Vec::new();
        while settlements.is_empty() {
            while let Some(tick) = scheduler.tick() {
                settlements.extend_from_slice(tick.into_report().settlements());
            }
            if !settlements.is_empty() {
                break;
            }
            assert!(Instant::now() < limit, "retained task did not recover");
            let mut select = Select::new();
            let readiness = scheduler.register(&mut select);
            let deadline = readiness.deadline().unwrap_or(limit).min(limit);
            if let Ok(operation) = select.ready_timeout(
                deadline.saturating_duration_since(Instant::now()),
            ) {
                assert!(readiness.contains(operation));
            }
        }
        assert_eq!(settlements.len(), 1);
        assert_eq!(matches!(settlements[0], Settlement::Aborted(_)), retire);
        assert_eq!(*order.lock().unwrap(), [7]);
        assert_readiness(&scheduler);
        let mut select = Select::new();
        let readiness = scheduler.register(&mut select);
        assert!(!readiness.pending());
        assert!(readiness.deadline().is_none());
        drop(session);
    }
}

fn plan<A>(action: A) -> Plan<u64>
where
    A: Action<u64, Inputs = (u64,), Output = u64>,
{
    Plan::builder(vec![Job::new(action)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .build()
        .unwrap()
}

fn submit(
    scheduler: &mut Scheduler<u64, impl zrx_executor::Strategy>,
    plan: zrx_scheduler::PlanId, key: u64, value: u64,
) {
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();
    let mut writer = session.begin().unwrap();
    writer.insert(key, value).unwrap();
    let _ = writer.seal().unwrap();
}

struct Record {
    marker: u64,
    order: Arc<Mutex<Vec<u64>>>,
}

impl Action<u64> for Record {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            self.order.lock().unwrap().push(self.marker);
            if let Change::Insert(key, value) = change {
                emit.insert(key, value.into_owned());
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

impl Action<u64> for DropProbe {
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

struct Blocking {
    started: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Action<u64> for Blocking {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        self.started.send(()).unwrap();
        self.release.recv().unwrap();
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            if let Change::Insert(key, value) = change {
                emit.insert(key, value.into_owned());
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

struct Count(Arc<AtomicUsize>);

impl Action<u64> for Count {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

struct CountProgress(Arc<AtomicUsize>);

impl Action<u64> for CountProgress {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, _| {
            if matches!(event, Event::Progress(_)) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
            Ok(())
        });
    }
}

#[test]
fn default_scheduler_selects_the_default_work_sharing_type() {
    fn assert_default<T: Default>() {}
    assert_default::<Scheduler<u64>>();
}

#[test]
fn ticks_visit_ready_plans_round_robin() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let left = scheduler.attach(plan(Record {
        marker: 1,
        order: Arc::clone(&order),
    }));
    let right = scheduler.attach(plan(Record {
        marker: 2,
        order: Arc::clone(&order),
    }));

    for (id, cause) in [(left, 1), (right, 2)] {
        submit(&mut scheduler, id, cause, cause);
    }

    let mut actions = Vec::new();
    while actions.len() != 2 {
        let before = order.lock().unwrap().len();
        let tick = scheduler.tick().unwrap();
        if order.lock().unwrap().len() != before {
            actions.push(tick.plan());
        }
    }
    assert_eq!(actions, [left, right]);
    assert_eq!(*order.lock().unwrap(), [1, 2]);
}

#[test]
fn detach_drops_state_and_invalidates_a_reused_slot() {
    let dropped = Arc::new(AtomicUsize::new(0));
    let mut scheduler = Scheduler::new(Immediate::new());
    let old = scheduler.attach(plan(DropProbe(Arc::clone(&dropped))));

    scheduler.attachment(old).unwrap().detach();
    let retired = scheduler.tick().unwrap();
    assert_eq!(retired.plan(), old);
    assert!(retired.into_report().is_empty());
    assert_eq!(dropped.load(Ordering::Relaxed), 1);
    assert!(matches!(
        scheduler.attachment(old),
        Err(SchedulerError::Plan(id)) if id == old
    ));

    let current = scheduler.attach(plan(DropProbe(Arc::clone(&dropped))));
    assert_ne!(current, old);
    assert!(scheduler.attachment(current).is_ok());
    assert!(matches!(
        scheduler.attachment(old),
        Err(SchedulerError::Plan(id)) if id == old
    ));
    scheduler.attachment(current).unwrap().detach();
    let _ = scheduler.tick().unwrap();
    assert_eq!(dropped.load(Ordering::Relaxed), 2);
}

#[test]
fn detach_does_not_dispatch_queued_progress() {
    // Open, sealed and explicitly aborted revisions all lose queued progress.
    for terminal in [Terminal::Open, Terminal::Sealed, Terminal::Aborted] {
        let calls = Arc::new(AtomicUsize::new(0));
        let program = Plan::builder(
            vec![Job::new(CountProgress(Arc::clone(&calls))).with_progress()],
            vec![vec![]],
        )
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .build()
        .unwrap();
        let mut scheduler = Scheduler::new(Immediate::new());
        let id = scheduler.attach(program);
        let session = scheduler
            .attachment(id)
            .unwrap()
            .session::<u64>(INPUT)
            .unwrap();
        let writer = session.begin().unwrap();
        let (_writer, _session) = match terminal {
            Terminal::Open => (Some(writer), None),
            Terminal::Sealed => (None, Some(writer.seal().unwrap())),
            Terminal::Aborted => (None, Some(writer.abort().unwrap())),
        };

        assert!(scheduler.tick().is_some());
        if !matches!(terminal, Terminal::Open) {
            assert!(scheduler.tick().is_some());
        }
        scheduler.attachment(id).unwrap().detach();

        let mut settlements = Vec::new();
        while let Some(tick) = scheduler.tick() {
            settlements.extend_from_slice(tick.into_report().settlements());
        }
        assert!(matches!(settlements.as_slice(), [Settlement::Aborted(_)]));
        assert!(!scheduler.readiness().pending());
        assert!(scheduler.readiness().deadline().is_none());
        assert_eq!(calls.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn detach_drains_in_background_while_other_plans_keep_running() {
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let successors = Arc::new(AtomicUsize::new(0));
    let blocked_plan = Plan::builder(
        vec![
            Job::new(Blocking {
                started: started_tx,
                release: release_rx,
            }),
            Job::new(Count(Arc::clone(&successors))),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
    .build()
    .unwrap();
    let mut scheduler = Scheduler::new(WorkSharing::new(2));
    let id = scheduler.attach(blocked_plan);
    submit(&mut scheduler, id, 1, 1);
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while started_rx.try_recv().is_err() {
        let _ = scheduler.tick();
        assert!(std::time::Instant::now() < deadline);
        thread::yield_now();
    }
    assert_readiness(&scheduler);
    assert!(scheduler.readiness().pending());
    scheduler.attachment(id).unwrap().detach();
    assert_readiness(&scheduler);
    assert!(scheduler.readiness().pending());
    assert!(matches!(
        scheduler.attachment(id),
        Err(SchedulerError::Plan(current)) if current == id
    ));

    let order = Arc::new(Mutex::new(Vec::new()));
    let live = scheduler.attach(plan(Record {
        marker: 2,
        order: Arc::clone(&order),
    }));
    submit(&mut scheduler, live, 2, 2);

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    while order.lock().unwrap().is_empty() {
        let _ = scheduler.tick();
        assert!(std::time::Instant::now() < deadline);
        thread::yield_now();
    }
    assert_eq!(*order.lock().unwrap(), [2]);
    release_tx.send(()).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let retired = loop {
        if let Some(tick) = scheduler.tick() {
            let plan = tick.plan();
            let report = tick.into_report();
            if plan == id && !report.is_empty() {
                break report;
            }
        }
        assert!(std::time::Instant::now() < deadline);
        thread::yield_now();
    };

    assert_eq!(successors.load(Ordering::Relaxed), 0);
    assert!(matches!(retired.settlements(), [Settlement::Aborted(_)]));
    assert!(scheduler.attachment(live).is_ok());
}

fn assert_readiness<S: zrx_executor::Strategy>(scheduler: &Scheduler<u64, S>) {
    let readiness = scheduler.readiness();
    let mut select = crossbeam::channel::Select::new();
    let registered = scheduler.register(&mut select);
    assert_eq!(readiness.pending(), registered.pending());
    assert_eq!(readiness.deadline(), registered.deadline());
    assert!(!readiness.contains(0));
}

#[test]
fn readiness_tracks_the_earliest_attached_wake() {
    use std::time::Instant;
    use zrx_scheduler::action::{Wake, WakeKey};

    struct ArmWake(Instant);

    impl Action<u64> for ArmWake {
        type Inputs = (u64,);
        type Output = u64;

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context { inputs: input, output, .. } = context;
            input.for_each(output, |_, emit| {
                emit.wake(Wake::at(WakeKey::new(0), self.0));
                Ok(())
            });
        }
    }

    let mut scheduler = Scheduler::new(Immediate::new());
    assert_readiness(&scheduler);
    assert!(!scheduler.readiness().pending());
    assert!(scheduler.readiness().deadline().is_none());
    let earlier = Instant::now() + Duration::from_secs(60);
    let later = earlier + Duration::from_secs(60);
    let mut plans = Vec::new();
    for deadline in [later, earlier] {
        let id = scheduler.attach(plan(ArmWake(deadline)));
        let mut writer = scheduler
            .attachment(id)
            .unwrap()
            .session::<u64>(INPUT)
            .unwrap()
            .begin()
            .unwrap();
        writer.insert(1, 1).unwrap();
        let session = writer.seal().unwrap();
        while scheduler.tick().is_some() {}
        assert_readiness(&scheduler);
        assert!(!scheduler.readiness().pending());
        assert_eq!(scheduler.readiness().deadline(), Some(deadline));
        plans.push((id, session));
    }
    scheduler.attachment(plans[1].0).unwrap().detach();
    while scheduler.tick().is_some() {}
    assert_readiness(&scheduler);
    assert_eq!(scheduler.readiness().deadline(), Some(later));
    scheduler.attachment(plans[0].0).unwrap().detach();
    while scheduler.tick().is_some() {}
    assert_readiness(&scheduler);
    assert!(scheduler.readiness().deadline().is_none());
}

#[test]
fn aborted_partial_convergence_retires_after_committed_return() {
    // Both executor-owned work and an unimported return retain the continuation.
    for returned in [false, true] {
        let program = Plan::builder(
            vec![
                Job::forward::<u64>(),
                Job::new(CountProgress(Arc::new(AtomicUsize::new(0))))
                    .with_progress(),
                Job::new(Join).with_progress(),
            ],
            vec![
                vec![Route::new(1, 0), Route::new(2, 0)],
                vec![Route::new(2, 1)],
                vec![],
            ],
        )
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .build()
        .unwrap();
        let queue = Queued::default();
        let mut scheduler = Scheduler::new(queue.clone());
        let id = scheduler.attach(program);
        let session = scheduler
            .attachment(id)
            .unwrap()
            .session::<u64>(INPUT)
            .unwrap();
        let _session = session.begin().unwrap().abort().unwrap();
        for _ in 0..20 {
            if scheduler.tick().is_none() {
                break;
            }
        }
        assert_eq!(
            queue.len(),
            1,
            "one abort callback is committed; other arrival is retained at join"
        );
        if returned {
            queue.execute(0);
        }
        scheduler.attachment(id).unwrap().detach();
        if !returned {
            queue.execute(0);
        }
        let mut settlements = Vec::new();
        for _ in 0..20 {
            if let Some(tick) = scheduler.tick() {
                settlements.extend_from_slice(tick.into_report().settlements());
            } else {
                break;
            }
        }
        assert!(!scheduler.readiness().pending());
        assert!(scheduler.readiness().deadline().is_none());
        assert!(matches!(settlements.as_slice(), [Settlement::Aborted(_)]));
        assert_eq!(queue.len(), 0);
        assert!(scheduler.tick().is_none());
    }
}

#[test]
fn retirement_closes_output_reservations_behind_a_return_gap() {
    let other = InputId::new(2);
    let calls = Arc::new(AtomicUsize::new(0));
    let program = Plan::builder(
        vec![
            Job::forward::<u64>(),
            Job::forward::<u64>(),
            Job::new(Count(Arc::clone(&calls))),
        ],
        vec![vec![Route::new(2, 0)], vec![Route::new(2, 0)], vec![]],
    )
    .inputs(vec![
        InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(other, Route::new(1, 0)),
    ])
    .build()
    .unwrap();
    let queue = Queued::default();
    let mut scheduler = Scheduler::new(queue.clone());
    let id = scheduler.attach(program);
    let mut sessions = Vec::new();
    for input in [INPUT, other] {
        let session = scheduler
            .attachment(id)
            .unwrap()
            .session::<u64>(input)
            .unwrap();
        let mut writer = session.begin().unwrap();
        writer.insert(1, 1).unwrap();
        sessions.push(writer.seal().unwrap());
    }
    // Reserve two positions at the same destination without running either task.
    for _ in 0..20 {
        if scheduler.tick().is_none() {
            break;
        }
    }
    assert_eq!(queue.len(), 2);
    // The second producer returns first, leaving its output behind the first
    // producer's reserved position. Retirement must prune it without losing
    // gap repair.
    queue.execute(1);
    assert!(
        scheduler
            .tick()
            .unwrap()
            .into_report()
            .settlements()
            .is_empty()
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    scheduler.attachment(id).unwrap().detach();
    queue.execute(0);
    let mut settlements = Vec::new();
    while let Some(tick) = scheduler.tick() {
        settlements.extend_from_slice(tick.into_report().settlements());
    }
    assert!(
        matches!(settlements.as_slice(), [Settlement::Aborted(first), Settlement::Aborted(second)] if first != second)
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert_eq!(queue.len(), 0);
    assert!(!scheduler.readiness().pending());
    assert!(scheduler.readiness().deadline().is_none());
}

#[test]
fn idle_retirement_does_not_wait_for_another_plan() {
    use std::sync::atomic::AtomicBool;
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut scheduler = Scheduler::new(WorkSharing::new(1));
    let busy = scheduler.attach(plan(Blocking {
        started: started_tx,
        release: release_rx,
    }));
    let idle = scheduler.attach(plan(Count(Arc::new(AtomicUsize::new(0)))));
    submit(&mut scheduler, busy, 1, 1);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        assert!(std::time::Instant::now() < deadline, "worker did not start");
        let _ = scheduler.tick();
        if started_rx.try_recv().is_ok() {
            break;
        }
        thread::yield_now();
    }
    scheduler.attachment(idle).unwrap().detach();
    let (done_tx, done_rx) = mpsc::channel();
    let watchdog_fired = Arc::new(AtomicBool::new(false));
    let fired = Arc::clone(&watchdog_fired);
    let watchdog = thread::spawn(move || {
        if done_rx.recv_timeout(Duration::from_secs(1)).is_err() {
            fired.store(true, Ordering::SeqCst);
        }
        release_tx.send(()).unwrap();
    });
    let tick = scheduler.tick().expect("idle retirement must finish");
    assert_eq!(tick.plan(), idle);
    let _ = done_tx.send(());
    watchdog.join().unwrap();
    assert!(
        !watchdog_fired.load(Ordering::SeqCst),
        "idle retirement waited for unrelated worker completion"
    );
}

#[test]
fn scheduler_drop_keeps_return_ports_alive_for_running_and_queued_work() {
    for retire in [false, true] {
        let submissions = Arc::new(AtomicUsize::new(0));
        let panics = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let (waiting_tx, waiting_rx) = mpsc::channel();
        let strategy = ObservedShutdown {
            strategy: WorkSharing::with_capacity(1, 2),
            submissions: Arc::clone(&submissions),
            panics: Arc::clone(&panics),
            waiting: waiting_tx,
        };
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let mut scheduler = Scheduler::new(strategy);
        let running = scheduler.attach(plan(Blocking {
            started: started_tx,
            release: release_rx,
        }));
        let queued = scheduler.attach(plan(Count(Arc::clone(&calls))));
        submit(&mut scheduler, running, 1, 1);
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            let _ = scheduler.tick();
            if started_rx.try_recv().is_ok() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "worker did not start"
            );
            thread::yield_now();
        }
        submit(&mut scheduler, queued, 2, 2);
        while submissions.load(Ordering::Relaxed) != 2 {
            assert!(scheduler.tick().is_some());
        }
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        if retire {
            scheduler.attachment(running).unwrap().detach();
            scheduler.attachment(queued).unwrap().detach();
        }
        // Release the running task only when shutdown starts waiting. Without
        // the scheduler's wait-before-field-drop contract, both return sends panic.
        let release = thread::spawn(move || {
            let observed = waiting_rx.recv_timeout(Duration::from_secs(2));
            let _ = release_tx.send(());
            observed.expect("shutdown did not wait for accepted work");
        });
        drop(scheduler);
        release.join().unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            panics.load(Ordering::Relaxed),
            0,
            "shutdown disconnected a return port before accepted work completed"
        );
    }
}

#[test]
fn scheduler_drop_discards_rejected_work_without_retrying() {
    #[derive(Debug)]
    struct Reject(Arc<AtomicUsize>);

    impl Strategy for Reject {
        fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err(zrx_executor::Error::Submit(task))
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

    for retire in [false, true] {
        let attempts = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicUsize::new(0));
        let mut scheduler = Scheduler::new(Reject(Arc::clone(&attempts)));
        let id = scheduler.attach(plan(DropProbe(Arc::clone(&dropped))));
        submit(&mut scheduler, id, 1, 1);
        while attempts.load(Ordering::Relaxed) == 0 {
            assert!(scheduler.tick().is_some());
        }
        assert!(scheduler.readiness().pending());
        assert!(scheduler.readiness().deadline().is_some());
        assert_eq!(dropped.load(Ordering::Relaxed), 0);
        if retire {
            scheduler.attachment(id).unwrap().detach();
        }
        drop(scheduler);
        assert_eq!(attempts.load(Ordering::Relaxed), 1);
        assert_eq!(dropped.load(Ordering::Relaxed), 1);
    }
}
