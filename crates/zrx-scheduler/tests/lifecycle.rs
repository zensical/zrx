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

//! End-to-end scheduler lifecycle conformance.

use anyhow::anyhow;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use zrx_scheduler::Change;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context, Job, Wake, WakeKey};
use zrx_scheduler::plan::{
    InputBinding, InputId, OutputBinding, OutputId, Plan, Route,
};
use zrx_scheduler::{RevisionId, Settlement};

#[path = "support/runtime.rs"]
mod support;

use support::{Batch, Runtime};

const INPUT: InputId = InputId::new(1);
const OUTPUT: OutputId = OutputId::new(1);

struct Pass;

impl Action<u64> for Pass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs, output, .. } = context;
        inputs.for_each(output, |change, emit| {
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

struct Scale {
    factor: u64,
    fail: u64,
}

impl Action<u64> for Scale {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context { inputs, output, .. } = context;
        inputs.for_each(output, |change, emit| match change {
            Change::Insert(_, value) if *value.as_ref() == self.fail => {
                Err(anyhow!("deliberate branch failure").into())
            }
            Change::Insert(key, value) => {
                emit.insert(key, *value.as_ref() * self.factor);
                Ok(())
            }
            Change::Remove(key) => {
                emit.remove(key);
                Ok(())
            }
        });
    }
}

#[derive(Default)]
struct Seen {
    events: BTreeSet<Observation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Observation {
    Begin,
    Left,
    Right,
    Wake,
    End,
    Abort,
}

struct Converge {
    active: HashMap<RevisionId, Seen>,
    observations: Arc<Mutex<Vec<(RevisionId, Observation)>>>,
}

impl Converge {
    fn record(&self, revision: RevisionId, observation: Observation) {
        self.observations
            .lock()
            .unwrap()
            .push((revision, observation));
    }
}

impl Action<u64> for Converge {
    type Inputs = (u64, u64);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            revision,
            inputs: (left, right),
            output,
            events,
        } = context;
        left.for_each(output, |change, emit| {
            self.active
                .entry(revision)
                .or_default()
                .events
                .insert(Observation::Left);
            self.record(revision, Observation::Left);
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key * 10, value.into_owned());
                    emit.wake(Wake::at(WakeKey::new(key), Instant::now()));
                }
                Change::Remove(key) => emit.remove(key * 10),
            }
            Ok(())
        });
        right.for_each(output, |change, emit| {
            self.active
                .entry(revision)
                .or_default()
                .events
                .insert(Observation::Right);
            self.record(revision, Observation::Right);
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key * 10 + 1, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key * 10 + 1),
            }
            Ok(())
        });
        events.for_each(output, |event, _| {
            let observation = match event {
                Event::Progress(ProgressEvent::Begin) => {
                    let seen = self.active.entry(revision).or_default();
                    assert!(
                        seen.events.insert(Observation::Begin),
                        "revision began more than once"
                    );
                    Observation::Begin
                }
                Event::Wake { .. } => {
                    let seen = self.active.entry(revision).or_default();
                    assert!(
                        seen.events.contains(&Observation::Left),
                        "wake overtook the data that scheduled it"
                    );
                    seen.events.insert(Observation::Wake);
                    Observation::Wake
                }
                Event::Progress(ProgressEvent::End) => {
                    let seen =
                        self.active.remove(&revision).unwrap_or_default();
                    assert!(
                        seen.events.contains(&Observation::Begin),
                        "end arrived without begin"
                    );
                    assert_eq!(
                        seen.events.contains(&Observation::Left),
                        seen.events.contains(&Observation::Wake),
                        "wake was not closed"
                    );
                    Observation::End
                }
                Event::Progress(ProgressEvent::Abort) => {
                    self.active.remove(&revision);
                    Observation::Abort
                }
            };
            self.record(revision, observation);
            Ok(())
        });
    }
}

fn plan(observations: Arc<Mutex<Vec<(RevisionId, Observation)>>>) -> Plan<u64> {
    Plan::builder(
        vec![
            Job::new(Pass),
            Job::new(Scale { factor: 2, fail: u64::MAX }),
            Job::new(Scale { factor: 3, fail: 99 }),
            Job::new(Converge {
                active: HashMap::new(),
                observations,
            })
            .with_progress(),
        ],
        vec![
            vec![Route::new(1, 0), Route::new(2, 0)],
            vec![Route::new(3, 0)],
            vec![Route::new(3, 1)],
            vec![],
        ],
    )
    .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
    .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT, 3)])
    .build()
    .unwrap()
}

fn take(runtime: &mut Runtime<u64>) -> Vec<(u64, u64)> {
    let Some(egress) = runtime.egress() else {
        return Vec::new();
    };
    assert_eq!(egress.output(), OUTPUT);
    egress
        .into_changes::<u64>()
        .filter_map(|change| match change {
            Change::Insert(key, value) => Some((key, value)),
            Change::Remove(_) => None,
        })
        .collect()
}

fn failures(report: &zrx_scheduler::Report) -> usize {
    report
        .invocations()
        .iter()
        .map(|invocation| invocation.outcomes.failures().len())
        .sum()
}

#[test]
fn complete_lifecycle_closes_every_owned_boundary_once() {
    let observations = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = Runtime::new(plan(Arc::clone(&observations)));

    let revision = runtime.begin(INPUT).unwrap();
    runtime
        .ingress(revision, Batch::new(vec![Change::Insert(7_u64, 5_u64)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    // Internal work, progress, and the wake may drain, but physical
    // settlement remains held by both unaccepted external output batches.
    let report = runtime.run_until_idle();
    assert!(report.settlements().is_empty());
    assert_eq!(failures(&report), 0);
    let first = take(&mut runtime);
    assert!(runtime.run_until_idle().settlements().is_empty());
    let second = take(&mut runtime);
    let report = runtime.run_until_idle();
    let [Settlement::Complete(settled)] = report.settlements() else {
        panic!("revision did not complete exactly once")
    };
    let mut output = [first, second].concat();
    output.sort_unstable();
    assert_eq!(output, [(70, 10), (71, 15)]);

    let events: Vec<_> = observations
        .lock()
        .unwrap()
        .iter()
        .filter_map(|(revision, event)| (revision == settled).then_some(*event))
        .collect();
    assert_eq!(events.last(), Some(&Observation::End));
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Observation::Begin)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Observation::Left)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Observation::Right)
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| **event == Observation::Wake)
            .count(),
        1
    );
    let observed = observations.lock().unwrap().len();
    assert!(runtime.run_until_idle().is_empty());
    assert!(runtime.egress().is_none());
    assert_eq!(observations.lock().unwrap().len(), observed);

    // Empty revisions still deliver and close progress without fabricating
    // data, wakes, or duplicate settlement.
    let empty = runtime.begin(INPUT).unwrap();
    runtime.seal(empty).unwrap();
    let report = runtime.run_until_idle();
    let [Settlement::Complete(empty)] = report.settlements() else {
        panic!("empty revision did not complete exactly once")
    };
    let events: Vec<_> = observations
        .lock()
        .unwrap()
        .iter()
        .filter_map(|(revision, event)| (revision == empty).then_some(*event))
        .collect();
    assert_eq!(events, [Observation::Begin, Observation::End]);

    // A branch failure remains diagnostic, suppresses only that branch's
    // output, and still closes progress and settlement after accepted egress.
    let failed = runtime.begin(INPUT).unwrap();
    runtime
        .ingress(failed, Batch::new(vec![Change::Insert(9_u64, 99_u64)]))
        .unwrap();
    runtime.seal(failed).unwrap();
    let report = runtime.run_until_idle();
    assert_eq!(failures(&report), 1);
    assert!(report.settlements().is_empty());
    assert_eq!(take(&mut runtime), [(90, 198)]);
    let report = runtime.run_until_idle();
    assert!(matches!(report.settlements(), [Settlement::Complete(_)]));

    // Abort is independently terminal and cannot leave queued callbacks,
    // egress, or a second settlement behind.
    let aborted = runtime.begin(INPUT).unwrap();
    runtime.abort(aborted).unwrap();
    let report = runtime.run_until_idle();
    assert!(matches!(report.settlements(), [Settlement::Aborted(_)]));
    assert!(runtime.egress().is_none());
    let observed = observations.lock().unwrap().len();
    assert!(runtime.run_until_idle().is_empty());
    assert_eq!(observations.lock().unwrap().len(), observed);
}
