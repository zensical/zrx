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

//! Dynamic stream barrier tests.

use zrx_executor::strategy::Immediate;
use zrx_scheduler::Value;
use zrx_stream::{Change, Key, Run, Workflow};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone)]
struct Rule {
    maximum: u64,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for Rule {}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type Snapshot = Vec<(Key<u64>, String)>;

// ----------------------------------------------------------------------------

type OutputChange = (u64, Option<Snapshot>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<OutputChange> {
    run.output::<Snapshot>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => {
                (*key.try_as_id().unwrap(), Some(value))
            }
            Change::Remove(key) => (*key.try_as_id().unwrap(), None),
        })
        .collect()
}

fn failures(run: &Run<u64>) -> usize {
    run.report()
        .invocations()
        .iter()
        .map(|invocation| invocation.outcomes.error_count())
        .sum()
}

#[test]
fn fuzzy_barrier_removes_a_previously_required_unresolved_discovery() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    let failed = Arc::new(AtomicBool::new(false));
    let matcher_failed = Arc::clone(&failed);
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, move |_: &Rule| {
                let failed = Arc::clone(&matcher_failed);
                move |_: &Key<u64>| {
                    assert!(
                        !failed.load(Ordering::Relaxed),
                        "invalid discovery"
                    );
                    true
                }
            });
        workflow.output(&selected);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input::<()>().unwrap();
    let configuration = runner.input::<Rule>().unwrap();

    let mut revision = configuration.begin().unwrap();
    revision.insert(Key::from(10), Rule { maximum: 1 }).unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(10, Some(vec![]))]);

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(1), ()).unwrap();
    let discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(10, None)]);

    failed.store(true, Ordering::Relaxed);
    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(1), ()).unwrap();
    let discovered = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());
    assert_eq!(runner.errors().len(), 1);

    let mut revision = discovered.begin().unwrap();
    revision.remove(Key::from(1)).unwrap();
    let discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(10, Some(vec![]))]);
    assert!(runner.errors().is_empty());
    drop((discovered, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_filters_requirements_and_reopens_on_removal() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, |rule: &Rule| {
                let maximum = rule.maximum;
                move |key: &Key<u64>| {
                    key.try_as_id()
                        .is_ok_and(|identifier| *identifier <= maximum)
                }
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let completed = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 10 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = discovered.begin().unwrap();
    for key in 1..=10_000 {
        revision.insert(Key::from(key), ()).unwrap();
    }
    let discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    let mut revision = completed.begin().unwrap();
    for key in 1..=10_000 {
        revision.insert(Key::from(key), key.to_string()).unwrap();
    }
    let mut completed = revision.seal().unwrap();
    let changes = values(&mut runner.settle().unwrap());
    let [(1, Some(snapshot))] = changes.as_slice() else {
        panic!("expected one terminal snapshot");
    };
    assert_eq!(snapshot.len(), 10);
    assert_eq!(snapshot.first().unwrap().0, Key::from(1_u64));
    assert_eq!(snapshot.last().unwrap().0, Key::from(10_u64));

    let mut revision = completed.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    completed = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    drop((discovered, completed, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_never_publishes_provisional_revision_state() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, |_: &Rule| {
                |_: &Key<u64>| true
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let completed = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 1 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(1_u64), ()).unwrap();
    let discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    let mut revision = completed.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("transient"))
        .unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    let completed = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    drop((discovered, completed, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_reopens_for_late_discovery() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, |rule: &Rule| {
                let maximum = rule.maximum;
                move |key: &Key<u64>| {
                    key.try_as_id()
                        .is_ok_and(|identifier| *identifier <= maximum)
                }
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let completed = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 2 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(1_u64), ()).unwrap();
    let mut discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    let mut revision = completed.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("one"))
        .unwrap();
    let mut completed = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(vec![(Key::from(1_u64), String::from("one"))]))]
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(2_u64), ()).unwrap();
    discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    let mut revision = completed.begin().unwrap();
    revision
        .insert(Key::from(2_u64), String::from("two"))
        .unwrap();
    completed = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            1,
            Some(vec![
                (Key::from(1_u64), String::from("one")),
                (Key::from(2_u64), String::from("two")),
            ])
        )]
    );

    drop((discovered, completed, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_replays_discovery_for_replaced_matchers() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, |rule: &Rule| {
                let maximum = rule.maximum;
                move |key: &Key<u64>| {
                    key.try_as_id()
                        .is_ok_and(|identifier| *identifier <= maximum)
                }
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let completed = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = discovered.begin().unwrap();
    for key in 1..=3 {
        revision.insert(Key::from(key), ()).unwrap();
    }
    let discovered = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = completed.begin().unwrap();
    for key in 1..=2 {
        revision.insert(Key::from(key), key.to_string()).unwrap();
    }
    let mut completed = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 2 })
        .unwrap();
    let mut configuration = revision.seal().unwrap();
    let changes = values(&mut runner.settle().unwrap());
    let [(1, Some(snapshot))] = changes.as_slice() else {
        panic!("expected the discovered keys to be replayed");
    };
    assert_eq!(snapshot.len(), 2);

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 3 })
        .unwrap();
    configuration = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    let mut revision = completed.begin().unwrap();
    revision
        .insert(Key::from(3_u64), String::from("three"))
        .unwrap();
    completed = revision.seal().unwrap();
    let changes = values(&mut runner.settle().unwrap());
    let [(1, Some(snapshot))] = changes.as_slice() else {
        panic!("expected the replacement matcher to become fulfilled");
    };
    assert_eq!(snapshot.len(), 3);

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 1 })
        .unwrap();
    configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(vec![(Key::from(1_u64), String::from("1"))]))]
    );

    drop((discovered, completed, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_does_not_publish_aborted_pending_work() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, |rule: &Rule| {
                let maximum = rule.maximum;
                move |key: &Key<u64>| {
                    key.try_as_id()
                        .is_ok_and(|identifier| *identifier <= maximum)
                }
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let completed = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 2_000 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = discovered.begin().unwrap();
    for key in 1..=1_024 {
        revision.insert(Key::from(key), ()).unwrap();
    }
    let discovered = revision.abort().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    drop((discovered, completed, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_does_not_treat_failed_work_as_completion() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let source = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let completed = source.map(|value: &String| {
            anyhow::ensure!(value != "bad", "completion failed");
            Ok::<_, anyhow::Error>(value.clone())
        });
        let selected =
            discovered.barrier(&completed, &configuration, |rule: &Rule| {
                let maximum = rule.maximum;
                move |key: &Key<u64>| {
                    key.try_as_id()
                        .is_ok_and(|identifier| *identifier <= maximum)
                }
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let source = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 1 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(1_u64), ()).unwrap();
    let discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(1, None)]);

    let mut revision = source.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("bad"))
        .unwrap();
    let mut source = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = source.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("good"))
        .unwrap();
    source = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(vec![(Key::from(1_u64), String::from("good"))]))]
    );

    drop((discovered, source, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn fuzzy_barrier_does_not_publish_while_a_match_is_unresolved() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected =
            discovered.barrier(&completed, &configuration, |rule: &Rule| {
                let maximum = rule.maximum;
                move |key: &Key<u64>| {
                    let identifier = *key.try_as_id().unwrap();
                    assert!(identifier != 2, "invalid discovery");
                    identifier <= maximum
                }
            });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let discovered = runner.input_at::<()>(inputs[0]).unwrap();
    let completed = runner.input_at::<String>(inputs[1]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[2]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 3 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(10, Some(Vec::new()))]
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(1_u64), ()).unwrap();
    let mut discovered = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(10, None)]);

    let mut revision = completed.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("one"))
        .unwrap();
    let mut completed = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(10, Some(vec![(Key::from(1_u64), String::from("one"))]))]
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(2_u64), ()).unwrap();
    discovered = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(
        runner.errors()[0].key(),
        &[10_u64, 2].into_iter().collect::<Key<_>>()
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(Key::from(3_u64), ()).unwrap();
    discovered = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());
    assert_eq!(runner.errors().len(), 1);

    let mut revision = completed.begin().unwrap();
    revision
        .insert(Key::from(3_u64), String::from("three"))
        .unwrap();
    completed = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());
    assert_eq!(runner.errors().len(), 1);

    let mut revision = discovered.begin().unwrap();
    revision.remove(Key::from(2_u64)).unwrap();
    discovered = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            10,
            Some(vec![
                (Key::from(1_u64), String::from("one")),
                (Key::from(3_u64), String::from("three")),
            ]),
        )]
    );
    assert!(runner.errors().is_empty());

    drop((discovered, completed, configuration));
}
