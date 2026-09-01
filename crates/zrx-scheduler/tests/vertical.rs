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

//! Vertical integration tests.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use zrx_executor::Strategy;
use zrx_executor::strategy::WorkSharing;

use zrx_scheduler::Change;
use zrx_scheduler::Settlement;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{
    Action, Concurrency, Context, Emitter, InputChange, Job, Wake, WakeKey,
};
use zrx_scheduler::plan::{
    InputBinding, InputId, OutputBinding, OutputId, Plan, Route,
};

#[path = "support/runtime.rs"]
mod support;
use support::{Batch, Revision, Runtime};

const LEFT: InputId = InputId::new(1);
const RIGHT: InputId = InputId::new(2);
const OUTPUT: OutputId = OutputId::new(1);

type State = Arc<Mutex<BTreeMap<u64, u64>>>;
type Progress = Arc<Mutex<Vec<&'static str>>>;

fn insert(_cause: u64, key: u64, value: u64) -> Change<u64, u64> {
    Change::Insert(key, value)
}

fn remove(_cause: u64, key: u64) -> Change<u64, u64> {
    Change::Remove(key)
}

#[derive(Clone)]
struct Scale(u64);

impl Action<u64> for Scale {
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
                    emit.insert(key, value.as_ref() * self.0);
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

struct LatestProduct {
    left: BTreeMap<u64, u64>,
    right: BTreeMap<u64, u64>,
    emitted: BTreeSet<u64>,
    pending: BTreeMap<WakeKey, u64>,
    progress: Progress,
}

impl LatestProduct {
    fn new(progress: Progress) -> Self {
        Self {
            left: BTreeMap::new(),
            right: BTreeMap::new(),
            emitted: BTreeSet::new(),
            pending: BTreeMap::new(),
            progress,
        }
    }

    fn update(
        &mut self, left: bool, change: InputChange<'_, u64, u64>,
        emit: &mut Emitter<'_, u64, u64>,
    ) {
        let (state, other) = if left {
            (&mut self.left, &self.right)
        } else {
            (&mut self.right, &self.left)
        };
        match change {
            Change::Insert(key, value) => {
                state.insert(key, *value.as_ref());
                if other.contains_key(&key) {
                    let wake = WakeKey::new(key);
                    self.pending.insert(wake, key);
                    emit.wake(Wake::at(wake, Instant::now()));
                }
            }
            Change::Remove(key) => {
                state.remove(&key);
                let wake = WakeKey::new(key);
                self.pending.remove(&wake);
                emit.wake(Wake::clear(wake));
                if self.emitted.remove(&key) {
                    emit.remove(key);
                }
            }
        }
    }

    fn event(&mut self, event: Event, emit: &mut Emitter<'_, u64, u64>) {
        match event {
            Event::Wake { key: wake, .. } => {
                let key = self
                    .pending
                    .remove(&wake)
                    .expect("current wake has pending join state");
                let Some((&left, &right)) =
                    self.left.get(&key).zip(self.right.get(&key))
                else {
                    return;
                };
                self.emitted.insert(key);
                emit.insert(key, left * right);
            }
            Event::Progress(progress) => {
                let event = match progress {
                    ProgressEvent::Begin => "begin",
                    ProgressEvent::End => "end",
                    ProgressEvent::Abort => "abort",
                };
                self.progress.lock().unwrap().push(event);
            }
        }
    }
}

impl Action<u64> for LatestProduct {
    type Inputs = (u64, u64);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: (left, right),
            output,
            events,
            ..
        } = context;
        left.for_each(output, |change, emit| {
            self.update(true, change, emit);
            Ok(())
        });
        right.for_each(output, |change, emit| {
            self.update(false, change, emit);
            Ok(())
        });
        events.for_each(output, |event, emit| {
            self.event(event, emit);
            Ok(())
        });
    }
}

fn program(factor: u64, progress: Progress) -> Plan<u64> {
    Plan::builder(
        vec![
            Job::new::<Scale>(Scale(factor)),
            Job::new(LatestProduct::new(progress)).with_progress(),
        ],
        vec![vec![Route::new(1, 0)], vec![]],
    )
    .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT, 1)])
    .inputs(vec![
        InputBinding::new::<u64, u64>(LEFT, Route::new(0, 0)),
        InputBinding::new::<u64, u64>(RIGHT, Route::new(1, 1)),
    ])
    .build()
    .unwrap()
}

fn drain<S>(runtime: &mut Runtime<u64, S>, state: &State)
where
    S: Strategy,
{
    while let Some(egress) = runtime.egress() {
        assert_eq!(egress.output(), OUTPUT);
        egress.for_each::<u64>(|change| match change {
            Change::Insert(key, value) => {
                state.lock().unwrap().insert(key, *value.as_ref());
            }
            Change::Remove(key) => {
                state.lock().unwrap().remove(&key);
            }
        });
    }
}

fn assert_settled(report: &[Settlement], revisions: &[Revision]) {
    assert_eq!(report.len(), revisions.len());
    assert!(
        report
            .iter()
            .all(|settlement| matches!(settlement, Settlement::Complete(_)))
    );
}

#[test]
fn static_generation_runs_the_complete_kernel_path() {
    let state = Arc::new(Mutex::new(BTreeMap::new()));
    let progress = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = Runtime::with_strategy(
        program(10, Arc::clone(&progress)),
        WorkSharing::new(2),
    );

    let left = runtime.begin(LEFT).unwrap();
    let right = runtime.begin(RIGHT).unwrap();
    let left_segment = Batch::new(vec![insert(1, 1, 1), insert(2, 2, 2)]);
    let right_segment = Batch::new(vec![insert(3, 1, 3), insert(4, 2, 4)]);

    runtime.ingress(left, left_segment).unwrap();
    runtime.ingress(right, right_segment).unwrap();
    runtime.seal(left).unwrap();
    runtime.seal(right).unwrap();
    let mut report = runtime.run_until_idle();
    drain(&mut runtime, &state);
    report.append(runtime.run_until_idle());
    assert_eq!(*state.lock().unwrap(), BTreeMap::from([(1, 30), (2, 80)]));
    assert_settled(report.settlements(), &[left, right]);

    let events = progress.lock().unwrap();
    assert_eq!(events.iter().filter(|&&event| event == "begin").count(), 2);
    assert_eq!(events.iter().filter(|&&event| event == "end").count(), 2);
    drop(events);

    let next = runtime.begin(LEFT).unwrap();
    let segment = Batch::new(vec![remove(5, 1)]);
    runtime.ingress(next, segment).unwrap();
    runtime.seal(next).unwrap();
    let mut report = runtime.run_until_idle();
    drain(&mut runtime, &state);
    report.append(runtime.run_until_idle());

    assert_eq!(*state.lock().unwrap(), BTreeMap::from([(2, 80)]));
    assert_settled(report.settlements(), &[next]);
}

#[test]
fn fresh_generation_rebuilds_state_after_the_old_generation_settles() {
    let old_state = Arc::new(Mutex::new(BTreeMap::new()));
    let mut old = Runtime::new(program(2, Arc::new(Mutex::new(Vec::new()))));
    let old_left = old.begin(LEFT).unwrap();
    let old_right = old.begin(RIGHT).unwrap();
    let _ = old.run_until_idle();

    old.ingress(old_left, Batch::new(vec![insert(1, 1, 1)]))
        .unwrap();

    old.ingress(old_right, Batch::new(vec![insert(2, 1, 3)]))
        .unwrap();
    let _ = old.run_until_idle();
    drain(&mut old, &old_state);
    old.seal(old_left).unwrap();
    old.seal(old_right).unwrap();
    let report = old.run_until_idle();
    assert_settled(report.settlements(), &[old_left, old_right]);
    assert_eq!(*old_state.lock().unwrap(), BTreeMap::from([(1, 6)]));
    drop(old);

    let new_state = Arc::new(Mutex::new(BTreeMap::new()));
    let mut new = Runtime::new(program(10, Arc::new(Mutex::new(Vec::new()))));
    let new_left = new.begin(LEFT).unwrap();
    let new_right = new.begin(RIGHT).unwrap();
    let _ = new.run_until_idle();

    new.ingress(new_left, Batch::new(vec![insert(3, 1, 1)]))
        .unwrap();

    new.ingress(new_right, Batch::new(vec![insert(4, 1, 3)]))
        .unwrap();
    let _ = new.run_until_idle();
    drain(&mut new, &new_state);
    new.seal(new_left).unwrap();
    new.seal(new_right).unwrap();
    let report = new.run_until_idle();

    assert_settled(report.settlements(), &[new_left, new_right]);
    assert_eq!(*new_state.lock().unwrap(), BTreeMap::from([(1, 30)]));
    assert_eq!(*old_state.lock().unwrap(), BTreeMap::from([(1, 6)]));
}
