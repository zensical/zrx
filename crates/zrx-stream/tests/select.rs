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

//! Dynamic selection tests.

use zrx_executor::strategy::Immediate;
use zrx_scheduler::Value;
use zrx_stream::{Change, Key, Membership, Run, Workflow};

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

type MembershipChange = (Vec<u64>, Option<(u64, String)>);

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
fn selector_replacement_refreshes_only_unresolved_retained_members() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<bool>();
        let factory = |allow: &bool| {
            let allow = *allow;
            move |value: &String| {
                anyhow::ensure!(allow || value != "new", "invalid candidate");
                Ok::<_, anyhow::Error>(true)
            }
        };
        workflow.output(&pages.select(&configuration, factory));
        workflow.output(&pages.select_by(&configuration, factory));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let pages = runner.input::<String>().unwrap();
    let configuration = runner.input::<bool>().unwrap();

    let mut revision = pages.begin().unwrap();
    revision.insert(Key::from(1), String::from("old")).unwrap();
    revision
        .insert(Key::from(2), String::from("stable"))
        .unwrap();
    let pages = revision.seal().unwrap();
    drop(runner.settle().unwrap());

    let mut revision = configuration.begin().unwrap();
    revision.insert(Key::from(10), false).unwrap();
    let configuration = revision.seal().unwrap();
    let mut initial = runner.settle().unwrap();
    assert_eq!(memberships(&mut initial).len(), 2);

    let mut revision = pages.begin().unwrap();
    revision.insert(Key::from(1), String::from("new")).unwrap();
    let pages = revision.seal().unwrap();
    let mut failed = runner.settle().unwrap();
    assert!(values(&mut failed).is_empty());
    assert!(memberships(&mut failed).is_empty());
    assert_eq!(runner.errors().len(), 2);

    // A failed selector replacement must retain the unresolved member too.
    let mut revision = configuration.begin().unwrap();
    revision.insert(Key::from(10), false).unwrap();
    let configuration = revision.seal().unwrap();
    let mut failed = runner.settle().unwrap();
    assert!(values(&mut failed).is_empty());
    assert!(memberships(&mut failed).is_empty());

    let mut revision = configuration.begin().unwrap();
    revision.insert(Key::from(10), true).unwrap();
    let configuration = revision.seal().unwrap();
    let mut recovered = runner.settle().unwrap();
    assert_eq!(
        values(&mut recovered),
        [(
            10,
            Some(vec![
                (Key::from(1), String::from("new")),
                (Key::from(2), String::from("stable")),
            ])
        )]
    );
    assert_eq!(
        memberships(&mut recovered),
        [(vec![10, 1], Some((1, String::from("new")))),]
    );
    assert!(runner.errors().is_empty());

    let mut revision = configuration.begin().unwrap();
    revision.insert(Key::from(10), true).unwrap();
    let configuration = revision.seal().unwrap();
    assert!(memberships(&mut runner.settle().unwrap()).is_empty());
    drop((pages, configuration));
}

fn memberships(run: &mut Run<u64>) -> Vec<MembershipChange> {
    run.output::<Membership<u64, String>>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, membership) => (
                key.iter().copied().collect(),
                Some((
                    *membership.candidate().try_as_id().unwrap(),
                    membership.value().clone(),
                )),
            ),
            Change::Remove(key) => (key.iter().copied().collect(), None),
        })
        .collect()
}

// ----------------------------------------------------------------------------

#[test]
fn select_by_emits_differential_memberships() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select_by(&configuration, |rule: &Rule| {
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
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 2 })
        .unwrap();
    let mut configuration = revision.seal().unwrap();
    assert!(memberships(&mut runner.settle().unwrap()).is_empty());

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("one"))
        .unwrap();
    revision
        .insert(Key::from(3_u64), String::from("three"))
        .unwrap();
    let pages = revision.seal().unwrap();
    assert_eq!(
        memberships(&mut runner.settle().unwrap()),
        [(vec![10, 1], Some((1, String::from("one"))))]
    );

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 3 })
        .unwrap();
    configuration = revision.seal().unwrap();
    assert_eq!(
        memberships(&mut runner.settle().unwrap()),
        [(vec![10, 3], Some((3, String::from("three"))))]
    );

    let mut revision = configuration.begin().unwrap();
    revision.remove(Key::from(10_u64)).unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        memberships(&mut runner.settle().unwrap()),
        [(vec![10, 1], None), (vec![10, 3], None)]
    );

    drop((pages, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn select_publishes_one_revision_complete_snapshot() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select(&configuration, |rule: &Rule| {
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
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 2_000 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = pages.begin().unwrap();
    for key in 1..=1_025 {
        revision.insert(Key::from(key), key.to_string()).unwrap();
    }
    revision.remove(Key::from(1_u64)).unwrap();
    let pages = revision.seal().unwrap();
    let changes = values(&mut runner.settle().unwrap());
    let [(_, Some(snapshot))] = changes.as_slice() else {
        panic!("expected one revision-complete snapshot");
    };
    assert_eq!(snapshot.len(), 1_024);
    assert_eq!(snapshot.first().unwrap().0, Key::from(2_u64));
    assert_eq!(snapshot.last().unwrap().0, Key::from(1_025_u64));
    drop((pages, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn select_does_not_publish_an_aborted_pending_batch() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select(&configuration, |rule: &Rule| {
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
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(1_u64), Rule { maximum: 2_000 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, Some(Vec::new()))]
    );

    let mut revision = pages.begin().unwrap();
    for key in 1..=1_024 {
        revision.insert(Key::from(key), key.to_string()).unwrap();
    }
    let pages = revision.abort().unwrap();

    assert!(values(&mut runner.settle().unwrap()).is_empty());
    drop((pages, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn select_rebuilds_revision_complete_dynamic_snapshots() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select(&configuration, |rule: &Rule| {
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
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("one"))
        .unwrap();
    revision
        .insert(Key::from(2_u64), String::from("two"))
        .unwrap();
    revision
        .insert(Key::from(3_u64), String::from("three"))
        .unwrap();
    let mut pages = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 2 })
        .unwrap();
    let mut configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            10,
            Some(vec![
                (Key::from(1_u64), String::from("one")),
                (Key::from(2_u64), String::from("two")),
            ]),
        )]
    );

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(2_u64), String::from("TWO"))
        .unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    pages = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(10, Some(vec![(Key::from(2_u64), String::from("TWO"))]),)]
    );

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 3 })
        .unwrap();
    configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            10,
            Some(vec![
                (Key::from(2_u64), String::from("TWO")),
                (Key::from(3_u64), String::from("three")),
            ]),
        )]
    );

    let mut revision = configuration.begin().unwrap();
    revision.remove(Key::from(10_u64)).unwrap();
    configuration = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(10, None)]);

    drop((pages, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn select_publishes_an_empty_configured_snapshot() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select(&configuration, |rule: &Rule| {
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
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 0 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(10, Some(Vec::new()))]
    );

    drop((pages, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn select_does_not_publish_an_empty_snapshot_for_a_failed_factory() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select(&configuration, |rule: &Rule| {
            assert!(rule.maximum != u64::MAX, "invalid selector");
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
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: u64::MAX })
        .unwrap();
    let mut configuration = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].key(), &Key::from(10_u64));

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 0 })
        .unwrap();
    configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(10, Some(Vec::new()))]
    );
    assert!(runner.errors().is_empty());

    drop((pages, configuration));
}

// ----------------------------------------------------------------------------

#[test]
fn select_keeps_the_accepted_value_when_another_pair_publishes() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let configuration = workflow.input::<Rule>();
        let selected = pages.select(&configuration, |rule: &Rule| {
            let maximum = rule.maximum;
            move |key: &Key<u64>, value: &String| {
                anyhow::ensure!(value != "bad", "invalid candidate");
                Ok::<_, anyhow::Error>(
                    key.try_as_id()
                        .is_ok_and(|identifier| *identifier <= maximum),
                )
            }
        });
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let configuration = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("one"))
        .unwrap();
    revision
        .insert(Key::from(2_u64), String::from("two"))
        .unwrap();
    let mut pages = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = configuration.begin().unwrap();
    revision
        .insert(Key::from(10_u64), Rule { maximum: 2 })
        .unwrap();
    let configuration = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            10,
            Some(vec![
                (Key::from(1_u64), String::from("one")),
                (Key::from(2_u64), String::from("two")),
            ]),
        )]
    );

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("bad"))
        .unwrap();
    pages = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(
        runner.errors()[0].key(),
        &[10_u64, 1].into_iter().collect::<Key<_>>()
    );

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(2_u64), String::from("TWO"))
        .unwrap();
    pages = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            10,
            Some(vec![
                (Key::from(1_u64), String::from("one")),
                (Key::from(2_u64), String::from("TWO")),
            ]),
        )]
    );
    assert_eq!(runner.errors().len(), 1);

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(1_u64), String::from("ONE"))
        .unwrap();
    pages = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(
            10,
            Some(vec![
                (Key::from(1_u64), String::from("ONE")),
                (Key::from(2_u64), String::from("TWO")),
            ]),
        )]
    );
    assert!(runner.errors().is_empty());

    drop((pages, configuration));
}
