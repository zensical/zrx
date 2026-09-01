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

//! Session integration tests.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use zrx_executor::Strategy;
use zrx_executor::strategy::Immediate;
use zrx_scheduler::Change;
use zrx_scheduler::action::{Action, Context, Job};
use zrx_scheduler::plan::{InputBinding, InputId, Plan, Route};
use zrx_scheduler::{Scheduler, Settlement};

const INPUT: InputId = InputId::new(1);

fn plan<A>(action: A) -> Plan<u64>
where
    A: Action<u64, Inputs = (u64,), Output = ()>,
{
    Plan::builder(vec![Job::new(action)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .build()
        .unwrap()
}

fn run_until_settled<S>(
    scheduler: &mut Scheduler<u64, S>, terminal: fn(&Settlement) -> bool,
) where
    S: Strategy,
{
    loop {
        while let Some(tick) = scheduler.tick() {
            if tick.into_report().settlements().iter().any(terminal) {
                return;
            }
        }
        let mut select = crossbeam::channel::Select::new();
        let readiness = scheduler.register(&mut select);
        let operation = select.ready_timeout(Duration::from_secs(1)).unwrap();
        assert!(readiness.contains(operation));
    }
}

struct Record {
    invocations: Arc<AtomicUsize>,
    values: Arc<Mutex<Vec<(u64, u64)>>>,
}

impl Action<u64> for Record {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        self.invocations.fetch_add(1, Ordering::Relaxed);
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, _| {
            if let Change::Insert(key, value) = change {
                self.values.lock().unwrap().push((key, *value.as_ref()));
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

#[test]
fn transferable_session_batches_individual_changes() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::clone(&invocations),
        values: Arc::clone(&values),
    }));
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();

    let provider = thread::spawn(move || {
        let mut writer = session.begin().unwrap();
        writer.insert(1, 10).unwrap();
        writer.insert(2, 20).unwrap();
        writer.insert(3, 30).unwrap();
        let _session = writer.seal().unwrap();
    });

    run_until_settled(&mut scheduler, |settlement| {
        matches!(settlement, Settlement::Complete(_))
    });
    provider.join().unwrap();

    assert_eq!(invocations.load(Ordering::Relaxed), 1);
    assert_eq!(*values.lock().unwrap(), [(1, 10), (2, 20), (3, 30)]);
}

#[test]
fn transferable_session_emits_a_batch_without_staging() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::clone(&invocations),
        values: Arc::clone(&values),
    }));
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();

    let provider = thread::spawn(move || {
        let mut writer = session.begin().unwrap();
        writer
            .emit_batch((1_usize..1_026).map(|key| {
                let key = u64::try_from(key).unwrap();
                Change::Insert(key, key * 10)
            }))
            .unwrap();
        let _session = writer.seal().unwrap();
    });

    run_until_settled(&mut scheduler, |settlement| {
        matches!(settlement, Settlement::Complete(_))
    });
    provider.join().unwrap();

    assert_eq!(invocations.load(Ordering::Relaxed), 2);
    let values = values.lock().unwrap();
    assert_eq!(values.len(), 1_025);
    assert_eq!(values.first(), Some(&(1, 10)));
    assert_eq!(values.last(), Some(&(1_025, 10_250)));
}

#[test]
fn incremental_emission_consumes_one_native_batch_per_call() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::clone(&invocations),
        values: Arc::clone(&values),
    }));
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();

    let provider = thread::spawn(move || {
        let mut writer = session.begin().unwrap();
        let mut changes =
            (1_u64..=1_025).map(|key| Change::Insert(key, key * 10));
        assert_eq!(writer.emit_from(&mut changes).unwrap(), 1_024);
        assert_eq!(writer.emit_from(&mut changes).unwrap(), 1);
        assert_eq!(writer.emit_from(&mut changes).unwrap(), 0);
        let _session = writer.seal().unwrap();
    });

    run_until_settled(&mut scheduler, |settlement| {
        matches!(settlement, Settlement::Complete(_))
    });
    provider.join().unwrap();

    assert_eq!(invocations.load(Ordering::Relaxed), 2);
    let values = values.lock().unwrap();
    assert_eq!(values.len(), 1_025);
    assert_eq!(values.first(), Some(&(1, 10)));
    assert_eq!(values.last(), Some(&(1_025, 10_250)));
}

#[test]
fn bounded_session_channel_propagates_scheduler_progress() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::clone(&invocations),
        values: Arc::clone(&values),
    }));
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();

    let provider = thread::spawn(move || {
        let mut writer = session.begin().unwrap();
        writer.insert(1, 10).unwrap();
        writer.insert(2, 20).unwrap();
        let _session = writer.seal().unwrap();
    });

    run_until_settled(&mut scheduler, |settlement| {
        matches!(settlement, Settlement::Complete(_))
    });
    provider.join().unwrap();

    assert_eq!(invocations.load(Ordering::Relaxed), 1);
    assert_eq!(*values.lock().unwrap(), [(1, 10), (2, 20)]);
}

#[test]
fn session_rejects_a_mismatched_value_type() {
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::new(AtomicUsize::new(0)),
        values: Arc::new(Mutex::new(Vec::new())),
    }));

    assert!(
        scheduler
            .attachment(plan)
            .unwrap()
            .session::<String>(INPUT)
            .is_err()
    );
}

#[test]
fn one_input_issues_exactly_one_authoritative_session() {
    let mut scheduler = Scheduler::new(Immediate::new());
    let attached = scheduler.attach(plan(Record {
        invocations: Arc::new(AtomicUsize::new(0)),
        values: Arc::new(Mutex::new(Vec::new())),
    }));
    let _session = scheduler
        .attachment(attached)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();
    assert!(
        scheduler
            .attachment(attached)
            .unwrap()
            .session::<u64>(INPUT)
            .is_err()
    );
}

#[test]
fn empty_revision_ingress_does_not_starve_its_settlement() {
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::new(AtomicUsize::new(0)),
        values: Arc::new(Mutex::new(Vec::new())),
    }));
    let mut session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();
    for _ in 0..9 {
        session = session.begin().unwrap().seal().unwrap();
    }

    for _ in 0..16 {
        assert!(scheduler.tick().unwrap().into_report().is_empty());
    }
    assert!(
        scheduler
            .tick()
            .unwrap()
            .into_report()
            .settlements()
            .iter()
            .any(|settlement| matches!(settlement, Settlement::Complete(_)))
    );
}

#[test]
fn dropped_idle_session_becomes_terminal_without_false_readiness() {
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::new(AtomicUsize::new(0)),
        values: Arc::new(Mutex::new(Vec::new())),
    }));
    drop(
        scheduler
            .attachment(plan)
            .unwrap()
            .session::<u64>(INPUT)
            .unwrap(),
    );

    let terminal = scheduler.tick().unwrap();
    assert!(terminal.progressed());
    assert!(terminal.into_report().is_empty());
    assert!(scheduler.tick().is_none());

    let mut select = crossbeam::channel::Select::new();
    let readiness = scheduler.register(&mut select);
    assert!(!readiness.pending());
    assert_eq!(readiness.deadline(), None);
    assert!(select.ready_timeout(Duration::from_millis(10)).is_err());
}

#[test]
fn sealed_writer_returns_the_session_for_another_revision() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::clone(&invocations),
        values: Arc::clone(&values),
    }));
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();

    let provider = thread::spawn(move || {
        let mut first = session.begin().unwrap();
        first.insert(1, 10).unwrap();
        let session = first.seal().unwrap();
        let mut second = session.begin().unwrap();
        second.insert(2, 20).unwrap();
        let _session = second.seal().unwrap();
    });

    let mut settlements = 0;
    while settlements != 2 {
        while let Some(tick) = scheduler.tick() {
            settlements += tick
                .into_report()
                .settlements()
                .iter()
                .filter(|settlement| {
                    matches!(settlement, Settlement::Complete(_))
                })
                .count();
        }
        if settlements != 2 {
            let mut select = crossbeam::channel::Select::new();
            let readiness = scheduler.register(&mut select);
            let operation =
                select.ready_timeout(Duration::from_secs(1)).unwrap();
            assert!(readiness.contains(operation));
        }
    }
    provider.join().unwrap();

    assert_eq!(*values.lock().unwrap(), [(1, 10), (2, 20)]);
}

#[test]
fn dropping_an_open_writer_aborts_its_queued_revision() {
    let invocations = Arc::new(AtomicUsize::new(0));
    let values = Arc::new(Mutex::new(Vec::new()));
    let mut scheduler = Scheduler::new(Immediate::new());
    let plan = scheduler.attach(plan(Record {
        invocations: Arc::clone(&invocations),
        values: Arc::clone(&values),
    }));
    let session = scheduler
        .attachment(plan)
        .unwrap()
        .session::<u64>(INPUT)
        .unwrap();

    thread::spawn(move || {
        let mut writer = session.begin().unwrap();
        writer.insert(1, 10).unwrap();
        writer.insert(2, 20).unwrap();
        drop(writer);
    })
    .join()
    .unwrap();

    run_until_settled(&mut scheduler, |settlement| {
        matches!(settlement, Settlement::Aborted(_))
    });
    assert_eq!(invocations.load(Ordering::Relaxed), 0);
    assert!(values.lock().unwrap().is_empty());
}
