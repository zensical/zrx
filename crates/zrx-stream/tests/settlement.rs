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

//! Transitive revision-settlement tests.

use zrx_executor::strategy::Immediate;
use zrx_scheduler::Value;
use zrx_stream::function::Collection;
use zrx_stream::{Change, Key, Run, StreamSetExt, StreamTupleExt, Workflow};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone)]
struct Rule(u32);

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for Rule {}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<u32> {
    run.output::<Vec<u32>>()
        .unwrap()
        .find_map(|change| match change {
            Change::Insert(_, value) => Some(value),
            Change::Remove(_) => None,
        })
        .unwrap()
}

fn scalar(run: &mut Run<u64>) -> usize {
    run.output::<usize>()
        .unwrap()
        .find_map(|change| match change {
            Change::Insert(_, value) => Some(value),
            Change::Remove(_) => None,
        })
        .unwrap()
}

fn pairs(run: &mut Run<u64>) -> Vec<Change<u64, (u32, u32)>> {
    run.output::<(u32, u32)>().unwrap().collect()
}

fn failures(run: &Run<u64>) -> usize {
    run.report()
        .invocations()
        .iter()
        .map(|invocation| invocation.outcomes.error_count())
        .sum()
}

// ----------------------------------------------------------------------------

#[test]
fn downstream_reduction_waits_for_group_reductions() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u32>();
        let groups = input.reduce_by_key(
            |value: &u32| Key::from(u64::from(value % 2)),
            |values: &dyn Collection<Key<u64>, u32>| {
                Some(values.iter().map(|(_, value)| *value).sum::<u32>())
            },
        );
        let settled = groups.reduce(|values: &dyn Collection<Key<u64>, u32>| {
            let mut values =
                values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
            values.sort_unstable();
            Some(values)
        });
        workflow.output(&settled);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 10).unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [30]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 11).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [11, 20]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn downstream_reduction_waits_for_dynamic_selection() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let rules = workflow.input::<Rule>();
        let selected = pages.select(&rules, |rule: &Rule| {
            let maximum = rule.0;
            move |key: &Key<u64>| {
                key.try_as_id().is_ok_and(|id| *id <= u64::from(maximum))
            }
        });
        let settled = selected.reduce(
            |values: &dyn Collection<Key<u64>, Vec<(Key<u64>, String)>>| {
                Some(
                    values
                        .iter()
                        .map(|(_, selected)| selected.len())
                        .sum::<usize>(),
                )
            },
        );
        workflow.output(&settled);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let rules = runner.input_at::<Rule>(inputs[1]).unwrap();

    let mut revision = pages.begin().unwrap();
    revision.insert(Key::from(1), String::from("one")).unwrap();
    revision.insert(Key::from(2), String::from("two")).unwrap();
    let pages = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), 0);

    let mut revision = rules.begin().unwrap();
    revision.insert(Key::from(10), Rule(2)).unwrap();
    let rules = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), 2);

    let mut revision = pages.begin().unwrap();
    revision.remove(Key::from(1)).unwrap();
    let pages = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), 1);

    drop((pages, rules));
}

// ----------------------------------------------------------------------------

#[test]
fn downstream_reduction_waits_for_expanded_aggregate_work() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u32>();
        let aggregate =
            input.reduce(|values: &dyn Collection<Key<u64>, u32>| {
                Some(
                    values
                        .iter()
                        .map(|(key, value)| (key.clone(), *value))
                        .collect::<Vec<_>>(),
                )
            });
        let expanded = aggregate
            .flat_map(|values: &Vec<(Key<u64>, u32)>| values.clone())
            .map(|value: &u32| value * 2);
        let settled =
            expanded.reduce(|values: &dyn Collection<Key<u64>, u32>| {
                let mut values =
                    values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
                values.sort_unstable();
                Some(values)
            });
        workflow.output(&settled);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 10).unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [20, 40]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 11).unwrap();
    revision.insert(Key::from(3), 30).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [22, 40, 60]);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(2)).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [22, 60]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn sibling_join_publishes_one_coherent_transition_per_revision() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let left = source.map(|value: &u32| value + 1);
        let right = source.map(|value: &u32| value * 10);
        let joined = (left, right).join();
        workflow.output(&joined);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 1).unwrap();
    let input = revision.seal().unwrap();
    let changes = pairs(&mut runner.settle().unwrap());
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Insert(key, (2, 10))] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 2).unwrap();
    let input = revision.seal().unwrap();
    let changes = pairs(&mut runner.settle().unwrap());
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Insert(key, (3, 20))] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn sibling_join_does_not_publish_when_a_filtered_value_cannot_arrive() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let left = source.map(|value: &u32| *value);
        let right = source.filter_map(|value: &u32| {
            value.is_multiple_of(2).then_some(value * 10)
        });
        let joined = (left, right).join();
        workflow.output(&joined);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 2).unwrap();
    let input = revision.seal().unwrap();
    let changes = pairs(&mut runner.settle().unwrap());
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Insert(key, (2, 20))] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 3).unwrap();
    let input = revision.seal().unwrap();
    let changes = pairs(&mut runner.settle().unwrap());
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Remove(key)] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn sibling_join_preserves_failed_input_and_updates_successful_sibling() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let left = source.map(
            |value: &u32| -> std::result::Result<u32, std::io::Error> {
                if *value == 3 {
                    Err(std::io::Error::other("declared"))
                } else {
                    Ok(value + 1)
                }
            },
        );
        let right = source.map(|value: &u32| value * 10);
        let joined = (left, right).join();
        workflow.output(&joined);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 2).unwrap();
    let input = revision.seal().unwrap();
    let changes = pairs(&mut runner.settle().unwrap());
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Insert(key, (3, 20))] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 3).unwrap();
    let input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    let changes = pairs(&mut run);
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Insert(key, (3, 30))] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );
    assert_eq!(failures(&run), 1);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn unrelated_callback_failure_does_not_block_join_publication() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let left = source.map(|value: &u32| value + 1);
        let right = source.map(|value: &u32| value * 10);
        let _unrelated = source.map(
            |_value: &u32| -> std::result::Result<u32, std::io::Error> {
                Err(std::io::Error::other("unrelated"))
            },
        );
        let joined = (left, right).join();
        workflow.output(&joined);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 2).unwrap();
    let input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    let changes = pairs(&mut run);
    assert!(
        matches!(
            changes.as_slice(),
            [Change::Insert(key, (3, 20))] if key == &Key::from(1)
        ),
        "unexpected changes: {changes:?}"
    );
    assert_eq!(failures(&run), 1);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn terminal_reduction_accepts_shared_multi_input_convergence() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let first = workflow.input::<u32>();
        let second = workflow.input::<u32>();
        let combined = (first, second).coalesce();
        let left = combined.map(|value: &u32| value + 1);
        let right = combined.map(|value: &u32| value * 10);
        let joined = (left, right).join();
        let settled =
            joined.reduce(|members: &dyn Collection<Key<u64>, (u32, u32)>| {
                Some(members.len())
            });
        workflow.output(&settled);
    });

    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let first = runner.input_at::<u32>(inputs[0]).unwrap();
    let second = runner.input_at::<u32>(inputs[1]).unwrap();

    let mut revision = first.begin().unwrap();
    revision.insert(Key::from(1), 10).unwrap();
    let first = revision.seal().unwrap();
    let mut revision = second.begin().unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    let second = revision.seal().unwrap();

    let _run = runner.settle().unwrap();
    drop((first, second));
}
