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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::Duration;

use zrx_executor::strategy::{Immediate, WorkSharing};

use zrx_scheduler::Change;
use zrx_scheduler::action::control::Event;
use zrx_scheduler::action::{Action, Context, Job};
use zrx_scheduler::plan::{InputBinding, InputId, Plan, Route};
use zrx_scheduler::{Error as SchedulerError, Scheduler, Settlement};

const INPUT: InputId = InputId::new(1);

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
    let _session = writer.seal().unwrap();

    assert!(scheduler.tick().is_some());
    assert!(scheduler.tick().is_some());
    scheduler.attachment(id).unwrap().detach();

    let mut aborted = false;
    while let Some(tick) = scheduler.tick() {
        aborted |= tick
            .into_report()
            .settlements()
            .iter()
            .any(|settlement| matches!(settlement, Settlement::Aborted(_)));
    }
    assert!(aborted);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
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
    scheduler.attachment(id).unwrap().detach();
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
