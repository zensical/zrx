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

//! Differential-correctness conformance tests.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;
use std::sync::mpsc;
use std::thread;

use zrx_executor::strategy::Immediate;
use zrx_store::Collection;
use zrx_stream::{
    Change, Error, Input, Key, Stream, StreamSetExt, StreamTupleExt, Value,
    Workflow,
};

#[path = "support/conformance.rs"]
mod support;

use support::{Differential, Terminal, key, path};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

enum Step {
    Seal(Vec<Change<u64, i64>>),
    Abort(Vec<Change<u64, i64>>),
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum ProviderCommand {
    Seal,
    Abort,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
struct Rule(u64);

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for Rule {}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn apply(state: &mut BTreeMap<Key<u64>, i64>, changes: &[Change<u64, i64>]) {
    for change in changes {
        match change {
            Change::Insert(key, value) => {
                state.insert(key.clone(), *value);
            }
            Change::Remove(key) => {
                assert!(state.remove(key).is_some());
            }
        }
    }
}

fn unary<T>(
    name: &'static str,
    build: impl FnOnce(&Stream<u64, i64>) -> Stream<u64, T>,
    expected: impl Fn(&BTreeMap<Key<u64>, i64>) -> BTreeMap<Key<u64>, T>,
) where
    T: Value + Debug + PartialEq,
{
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<i64>();
        workflow.output(&build(&input));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut input = Some(runner.input::<i64>().unwrap());
    let mut source = BTreeMap::new();
    let mut oracle = Differential::new(name);
    let steps = [
        Step::Seal(vec![
            Change::Insert(key(3), 30),
            Change::Insert(key(1), 11),
            Change::Insert(key(4), 24),
            Change::Insert(key(2), 20),
        ]),
        Step::Seal(vec![
            Change::Insert(key(1), 12),
            Change::Remove(key(2)),
            Change::Insert(key(5), 35),
        ]),
        Step::Seal(Vec::new()),
        Step::Abort(vec![Change::Insert(key(1), 999), Change::Remove(key(3))]),
        Step::Seal(vec![Change::Remove(key(4)), Change::Insert(key(6), 42)]),
    ];

    for step in steps {
        let mut revision = input.take().unwrap().begin().unwrap();
        let (changes, terminal) = match step {
            Step::Seal(changes) => (changes, Terminal::Complete),
            Step::Abort(changes) => (changes, Terminal::Aborted),
        };
        if terminal == Terminal::Complete {
            apply(&mut source, &changes);
        }
        for change in changes {
            revision.emit(change).unwrap();
        }
        input = Some(match terminal {
            Terminal::Complete => revision.seal().unwrap(),
            Terminal::Aborted => revision.abort().unwrap(),
        });

        let snapshot = expected(&source);
        let mut run = runner.settle().unwrap();
        oracle.observe(&mut run, &snapshot, &[terminal]);

        // Settlement is an owning observation: a second drain may neither
        // replay output nor report the same revision again.
        let mut quiescent = runner.settle().unwrap();
        oracle.observe(&mut quiescent, &snapshot, &[]);
    }

    drop(input);
}

fn mapped(
    source: &BTreeMap<Key<u64>, i64>, function: impl Fn(i64) -> Option<i64>,
) -> BTreeMap<Key<u64>, i64> {
    source
        .iter()
        .filter_map(|(key, value)| {
            function(*value).map(|value| (key.clone(), value))
        })
        .collect()
}

fn unique_by_key(source: &BTreeMap<Key<u64>, i64>) -> BTreeMap<Key<u64>, i64> {
    let mut claims = BTreeMap::<Key<u64>, Vec<i64>>::new();
    for value in source.values() {
        let derived = u64::try_from(value.rem_euclid(10)).unwrap();
        claims.entry(key(derived)).or_default().push(*value);
    }
    claims
        .into_iter()
        .filter_map(|(derived, values)| {
            (values.len() == 1).then(|| (derived, values[0]))
        })
        .collect()
}

#[test]
fn unary_operators_match_reference_state_across_complete_lifecycle() {
    unary(
        "map",
        |source| source.map(|value: &i64| value * 2),
        |source| mapped(source, |value| Some(value * 2)),
    );
    unary(
        "filter",
        |source| source.filter(|value: &i64| value % 2 == 0),
        |source| mapped(source, |value| (value % 2 == 0).then_some(value)),
    );
    unary(
        "filter_map",
        |source| {
            source
                .filter_map(|value: &i64| (value % 3 == 0).then_some(value / 3))
        },
        |source| mapped(source, |value| (value % 3 == 0).then_some(value / 3)),
    );
    unary(
        "unique_by_key",
        |source| {
            source.unique_by_key(|value: &i64| {
                Key::from(u64::try_from(*value).unwrap() + 100)
            })
        },
        |source| {
            source
                .values()
                .map(|value| {
                    (key(u64::try_from(*value).unwrap() + 100), *value)
                })
                .collect()
        },
    );
    unary(
        "group_by_key",
        |source| {
            source.group_by_key(|value: &i64| {
                Key::from(u64::try_from(value.rem_euclid(2)).unwrap())
            })
        },
        |source| {
            source
                .iter()
                .map(|(source, value)| {
                    let group = u64::try_from(value.rem_euclid(2)).unwrap();
                    (key(group).concat(source), *value)
                })
                .collect()
        },
    );
    unary(
        "flat_map",
        |source| {
            source.flat_map(|value: &i64| {
                vec![(key(0), *value), (key(1), -*value)]
            })
        },
        |source| {
            source
                .iter()
                .flat_map(|(source, value)| {
                    [
                        (source.concat(key(0)), *value),
                        (source.concat(key(1)), -*value),
                    ]
                })
                .collect()
        },
    );
}

#[test]
fn unique_by_key_matches_live_claim_projection_across_complete_lifecycle() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<i64>();
        let indexed = input.unique_by_key(|value: &i64| {
            let derived = u64::try_from(value.rem_euclid(10)).unwrap();
            key(derived)
        });
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut input = Some(runner.input::<i64>().unwrap());
    let mut source = BTreeMap::new();
    let mut oracle = Differential::new("unique_by_key");
    let steps = [
        (
            Step::Seal(vec![
                Change::Insert(key(1), 10),
                Change::Insert(key(2), 20),
                Change::Insert(key(3), 31),
            ]),
            1,
        ),
        (Step::Seal(vec![Change::Insert(key(2), 22)]), 0),
        (Step::Seal(vec![Change::Insert(key(3), 30)]), 1),
        (Step::Seal(vec![Change::Remove(key(1))]), 0),
        (Step::Seal(Vec::new()), 0),
        (
            Step::Abort(vec![
                Change::Insert(key(2), 20),
                Change::Remove(key(3)),
            ]),
            0,
        ),
        (Step::Seal(vec![Change::Insert(key(2), 21)]), 0),
    ];

    for (step, expected_failures) in steps {
        let mut revision = input.take().unwrap().begin().unwrap();
        let (changes, terminal) = match step {
            Step::Seal(changes) => (changes, Terminal::Complete),
            Step::Abort(changes) => (changes, Terminal::Aborted),
        };
        if terminal == Terminal::Complete {
            apply(&mut source, &changes);
        }
        for change in changes {
            revision.emit(change).unwrap();
        }
        input = Some(match terminal {
            Terminal::Complete => revision.seal().unwrap(),
            Terminal::Aborted => revision.abort().unwrap(),
        });

        let snapshot = unique_by_key(&source);
        let mut run = runner.settle().unwrap();
        oracle.observe_with_failures(
            &mut run,
            &snapshot,
            &[terminal],
            expected_failures,
        );

        let mut quiescent = runner.settle().unwrap();
        oracle.observe(&mut quiescent, &snapshot, &[]);
    }

    drop(input);
}

#[test]
fn ordered_windows_match_reference_state_across_complete_lifecycle() {
    unary(
        "take",
        |source| source.take(2),
        |source| {
            source
                .iter()
                .take(2)
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        },
    );
    unary(
        "take_last",
        |source| source.take_last(2),
        |source| {
            source
                .iter()
                .rev()
                .take(2)
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        },
    );
    unary(
        "skip",
        |source| source.skip(2),
        |source| {
            source
                .iter()
                .skip(2)
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        },
    );
    unary(
        "skip_last",
        |source| source.skip_last(2),
        |source| {
            source
                .iter()
                .take(source.len().saturating_sub(2))
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        },
    );
}

fn reduce_snapshot(
    source: &BTreeMap<Key<u64>, i64>,
) -> BTreeMap<Key<u64>, i64> {
    [(path([]), source.values().sum())].into_iter().collect()
}

#[test]
fn revision_terminal_reducers_match_reference_snapshots() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<i64>();
        let reduced = source.reduce(|values: &dyn Collection<Key<u64>, i64>| {
            Some(values.iter().map(|(_, value)| *value).sum::<i64>())
        });
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut input = runner.input::<i64>().unwrap();
    let mut source = BTreeMap::new();
    let mut oracle = Differential::new("reduce");

    for changes in [
        vec![Change::Insert(key(1), 10), Change::Insert(key(2), 20)],
        vec![Change::Insert(key(1), 15), Change::Remove(key(2))],
        Vec::new(),
        vec![Change::Remove(key(1))],
    ] {
        let mut revision = input.begin().unwrap();
        apply(&mut source, &changes);
        for change in changes {
            revision.emit(change).unwrap();
        }
        input = revision.seal().unwrap();
        let mut run = runner.settle().unwrap();
        oracle.observe(
            &mut run,
            &reduce_snapshot(&source),
            &[Terminal::Complete],
        );
    }
    drop(input);
}

#[test]
fn grouped_reduction_matches_reference_snapshots() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<i64>();
        let reduced = source.reduce_by_key(
            |value: &i64| key(u64::try_from(value.rem_euclid(2)).unwrap()),
            |values: &dyn Collection<Key<u64>, i64>| {
                Some(values.iter().map(|(_, value)| *value).sum::<i64>())
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut input = runner.input::<i64>().unwrap();
    let mut source = BTreeMap::new();
    let mut oracle = Differential::new("reduce_by_key");

    for changes in [
        vec![
            Change::Insert(key(1), 11),
            Change::Insert(key(2), 20),
            Change::Insert(key(3), 31),
        ],
        vec![Change::Insert(key(1), 12), Change::Remove(key(2))],
        Vec::new(),
        vec![Change::Remove(key(3))],
    ] {
        let mut revision = input.begin().unwrap();
        apply(&mut source, &changes);
        for change in changes {
            revision.emit(change).unwrap();
        }
        input = revision.seal().unwrap();

        let mut expected = BTreeMap::new();
        for value in source.values() {
            let group = key(u64::try_from(value.rem_euclid(2)).unwrap());
            *expected.entry(group).or_insert(0) += value;
        }
        let mut run = runner.settle().unwrap();
        oracle.observe(&mut run, &expected, &[Terminal::Complete]);
    }
    drop(input);
}

fn close_left(
    input: Input<u64, i64>, state: &mut BTreeMap<Key<u64>, i64>,
    changes: Vec<Change<u64, i64>>,
) -> Input<u64, i64> {
    let mut revision = input.begin().unwrap();
    apply(state, &changes);
    for change in changes {
        revision.emit(change).unwrap();
    }
    revision.seal().unwrap()
}

fn join_expected(
    left: &BTreeMap<Key<u64>, i64>, right: &BTreeMap<Key<u64>, u64>,
) -> BTreeMap<Key<u64>, (i64, u64)> {
    left.iter()
        .filter_map(|(key, left)| {
            right.get(key).map(|right| (key.clone(), (*left, *right)))
        })
        .collect()
}

fn binary<T>(
    name: &'static str,
    build: impl FnOnce(Stream<u64, i64>, Stream<u64, u64>) -> Stream<u64, T>,
    expected: impl Fn(
        &BTreeMap<Key<u64>, i64>,
        &BTreeMap<Key<u64>, u64>,
    ) -> BTreeMap<Key<u64>, T>,
) where
    T: Value + Debug + PartialEq,
{
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<i64>();
        let right = workflow.input::<u64>();
        workflow.output(&build(left, right));
    });
    let endpoints: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut left = runner.input_at::<i64>(endpoints[0]).unwrap();
    let mut right = runner.input_at::<u64>(endpoints[1]).unwrap();
    let mut left_state = BTreeMap::new();
    let mut right_state = BTreeMap::new();
    let mut oracle = Differential::new(name);

    let mut revision = left.begin().unwrap();
    revision.insert(key(1), 10).unwrap();
    revision.insert(key(2), 20).unwrap();
    left = revision.seal().unwrap();
    left_state.extend([(key(1), 10), (key(2), 20)]);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &expected(&left_state, &right_state),
        &[Terminal::Complete],
    );

    let mut revision = right.begin().unwrap();
    revision.insert(key(1), 100).unwrap();
    revision.insert(key(3), 300).unwrap();
    right = revision.seal().unwrap();
    right_state.extend([(key(1), 100), (key(3), 300)]);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &expected(&left_state, &right_state),
        &[Terminal::Complete],
    );

    // Converging revisions overlap and both mutate key 1 before settlement.
    let mut left_revision = left.begin().unwrap();
    let mut right_revision = right.begin().unwrap();
    left_revision.insert(key(1), 11).unwrap();
    left_revision.remove(key(2)).unwrap();
    left_revision.insert(key(4), 40).unwrap();
    right_revision.insert(key(1), 101).unwrap();
    right_revision.insert(key(4), 400).unwrap();
    left = left_revision.seal().unwrap();
    right = right_revision.seal().unwrap();
    left_state.insert(key(1), 11);
    left_state.remove(&key(2));
    left_state.insert(key(4), 40);
    right_state.insert(key(1), 101);
    right_state.insert(key(4), 400);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &expected(&left_state, &right_state),
        &[Terminal::Complete, Terminal::Complete],
    );

    // A locally buffered abort is terminal but cannot alter publication.
    let mut revision = left.begin().unwrap();
    revision.insert(key(1), 999).unwrap();
    revision.remove(key(4)).unwrap();
    left = revision.abort().unwrap();
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &expected(&left_state, &right_state),
        &[Terminal::Aborted],
    );

    let mut revision = right.begin().unwrap();
    revision.remove(key(1)).unwrap();
    revision.remove(key(3)).unwrap();
    right = revision.seal().unwrap();
    right_state.remove(&key(1));
    right_state.remove(&key(3));
    let snapshot = expected(&left_state, &right_state);
    let mut run = runner.settle().unwrap();
    oracle.observe(&mut run, &snapshot, &[Terminal::Complete]);
    let mut quiescent = runner.settle().unwrap();
    oracle.observe(&mut quiescent, &snapshot, &[]);

    drop((left, right));
}

#[test]
fn binary_operators_match_reference_state_under_convergence() {
    binary(
        "inner_join",
        |left, right| (left, right).join(),
        join_expected,
    );
    binary(
        "left_join",
        |left, right| (left, right).left_join(),
        |left, right| {
            left.iter()
                .map(|(key, left)| {
                    (key.clone(), (*left, right.get(key).copied()))
                })
                .collect()
        },
    );
    binary(
        "full_join",
        |left, right| (left, right).full_join(),
        |left, right| {
            left.keys()
                .chain(right.keys())
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .map(|key| {
                    let value =
                        (left.get(&key).copied(), right.get(&key).copied());
                    (key, value)
                })
                .collect()
        },
    );
    binary(
        "semi_join",
        |left, right| (left, right).semi_join(),
        |left, right| {
            left.iter()
                .filter(|(key, _)| right.contains_key(*key))
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        },
    );
    binary(
        "anti_join",
        |left, right| (left, right).anti_join(),
        |left, right| {
            left.iter()
                .filter(|(key, _)| !right.contains_key(*key))
                .map(|(key, value)| (key.clone(), *value))
                .collect()
        },
    );
    binary(
        "product",
        |left, right| left.product(&right),
        |left, right| {
            left.iter()
                .flat_map(|(left_key, left)| {
                    right.iter().map(move |(right_key, right)| {
                        (left_key.concat(right_key), (*left, *right))
                    })
                })
                .collect()
        },
    );
}

#[test]
fn overlapping_join_revisions_publish_only_current_reference_state() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<i64>();
        let right = workflow.input::<u64>();
        workflow.output(&(left, right).join());
    });
    let endpoints: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut left = runner.input_at::<i64>(endpoints[0]).unwrap();
    let mut right = runner.input_at::<u64>(endpoints[1]).unwrap();
    let mut left_state = BTreeMap::new();
    let mut right_state = BTreeMap::new();
    let mut oracle = Differential::new("join");

    let mut left_revision = left.begin().unwrap();
    let mut right_revision = right.begin().unwrap();
    left_revision.insert(key(1), 10).unwrap();
    left_revision.insert(key(2), 20).unwrap();
    right_revision.insert(key(1), 100).unwrap();
    right_revision.insert(key(2), 200).unwrap();
    left = left_revision.seal().unwrap();
    right = right_revision.seal().unwrap();
    left_state.extend([(key(1), 10), (key(2), 20)]);
    right_state.extend([(key(1), 100), (key(2), 200)]);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &join_expected(&left_state, &right_state),
        &[Terminal::Complete, Terminal::Complete],
    );

    // Two revisions mutate the same joined key before either settles. The
    // terminal publisher must suppress stale intermediate state.
    let mut left_revision = left.begin().unwrap();
    let mut right_revision = right.begin().unwrap();
    left_revision.insert(key(1), 11).unwrap();
    right_revision.insert(key(1), 101).unwrap();
    left = left_revision.seal().unwrap();
    right = right_revision.seal().unwrap();
    left_state.insert(key(1), 11);
    right_state.insert(key(1), 101);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &join_expected(&left_state, &right_state),
        &[Terminal::Complete, Terminal::Complete],
    );

    let left_changes = vec![Change::Remove(key(1))];
    left = close_left(left, &mut left_state, left_changes);
    let mut right_revision = right.begin().unwrap();
    right_revision.insert(key(2), 201).unwrap();
    right_state.insert(key(2), 201);
    right = right_revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &join_expected(&left_state, &right_state),
        &[Terminal::Complete, Terminal::Complete],
    );

    drop((left, right));
}

#[test]
fn coalesce_and_product_match_independent_input_reference_state() {
    let coalesce = Workflow::<u64>::build(|workflow| {
        let preferred = workflow.input::<i64>();
        let fallback = workflow.input::<i64>();
        workflow.output(&(preferred, fallback).coalesce());
    });
    let endpoints: Vec<_> = coalesce.inputs().copied().collect();
    let mut runner = coalesce.runner_with(Immediate::new()).unwrap();
    let preferred = runner.input_at::<i64>(endpoints[0]).unwrap();
    let fallback = runner.input_at::<i64>(endpoints[1]).unwrap();
    let mut preferred_revision = preferred.begin().unwrap();
    let mut fallback_revision = fallback.begin().unwrap();
    preferred_revision.insert(key(1), 10).unwrap();
    fallback_revision.insert(key(1), 100).unwrap();
    fallback_revision.insert(key(2), 200).unwrap();
    let preferred = preferred_revision.seal().unwrap();
    let fallback = fallback_revision.seal().unwrap();
    let mut oracle = Differential::new("coalesce");
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &[(key(1), 10_i64), (key(2), 200_i64)].into_iter().collect(),
        &[Terminal::Complete, Terminal::Complete],
    );
    drop((preferred, fallback));

    let product = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<i64>();
        let right = workflow.input::<u64>();
        workflow.output(&left.product(&right));
    });
    let endpoints: Vec<_> = product.inputs().copied().collect();
    let mut runner = product.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<i64>(endpoints[0]).unwrap();
    let right = runner.input_at::<u64>(endpoints[1]).unwrap();
    let mut left_revision = left.begin().unwrap();
    let mut right_revision = right.begin().unwrap();
    left_revision.insert(key(1), 10).unwrap();
    left_revision.insert(key(2), 20).unwrap();
    right_revision.insert(key(3), 30).unwrap();
    right_revision.insert(key(4), 40).unwrap();
    let left = left_revision.seal().unwrap();
    let right = right_revision.seal().unwrap();
    let expected = [
        (path([1, 3]), (10_i64, 30_u64)),
        (path([1, 4]), (10_i64, 40_u64)),
        (path([2, 3]), (20_i64, 30_u64)),
        (path([2, 4]), (20_i64, 40_u64)),
    ]
    .into_iter()
    .collect();
    let mut oracle = Differential::new("product");
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &expected,
        &[Terminal::Complete, Terminal::Complete],
    );
    drop((left, right));
}

fn selected_snapshot(
    values: &BTreeMap<Key<u64>, u64>, rules: &BTreeMap<Key<u64>, Rule>,
) -> BTreeMap<Key<u64>, Vec<(Key<u64>, u64)>> {
    rules
        .iter()
        .map(|(rule_key, rule)| {
            let selected = values
                .iter()
                .filter(|(key, _)| {
                    key.try_as_id().is_ok_and(|id| *id <= rule.0)
                })
                .map(|(key, value)| (key.clone(), *value))
                .collect();
            (rule_key.clone(), selected)
        })
        .collect()
}

#[test]
fn dynamic_selection_matches_revision_complete_reference_snapshots() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let values = workflow.input::<u64>();
        let rules = workflow.input::<Rule>();
        let selected = values.select(&rules, |rule: &Rule| {
            let maximum = rule.0;
            move |key: &Key<u64>| key.try_as_id().is_ok_and(|id| *id <= maximum)
        });
        workflow.output(&selected);
    });
    let endpoints: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut values = runner.input_at::<u64>(endpoints[0]).unwrap();
    let mut rules = runner.input_at::<Rule>(endpoints[1]).unwrap();
    let mut value_state = BTreeMap::new();
    let mut rule_state = BTreeMap::new();
    let mut oracle = Differential::new("select");

    let mut revision = values.begin().unwrap();
    revision.insert(key(1), 10).unwrap();
    revision.insert(key(2), 20).unwrap();
    revision.insert(key(3), 30).unwrap();
    values = revision.seal().unwrap();
    value_state.extend([(key(1), 10), (key(2), 20), (key(3), 30)]);
    let mut run = runner.settle().unwrap();
    oracle.observe(&mut run, &BTreeMap::new(), &[Terminal::Complete]);

    let mut revision = rules.begin().unwrap();
    revision.insert(key(10), Rule(2)).unwrap();
    rules = revision.seal().unwrap();
    rule_state.insert(key(10), Rule(2));
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &selected_snapshot(&value_state, &rule_state),
        &[Terminal::Complete],
    );

    // Both reachable inputs change before either terminal is drained.
    let mut value_revision = values.begin().unwrap();
    let mut rule_revision = rules.begin().unwrap();
    value_revision.remove(key(1)).unwrap();
    value_revision.insert(key(2), 22).unwrap();
    rule_revision.insert(key(10), Rule(3)).unwrap();
    values = value_revision.seal().unwrap();
    rules = rule_revision.seal().unwrap();
    value_state.remove(&key(1));
    value_state.insert(key(2), 22);
    rule_state.insert(key(10), Rule(3));
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &selected_snapshot(&value_state, &rule_state),
        &[Terminal::Complete, Terminal::Complete],
    );

    let mut revision = values.begin().unwrap();
    revision.remove(key(2)).unwrap();
    values = revision.abort().unwrap();
    let snapshot = selected_snapshot(&value_state, &rule_state);
    let mut run = runner.settle().unwrap();
    oracle.observe(&mut run, &snapshot, &[Terminal::Aborted]);

    let mut revision = rules.begin().unwrap();
    revision.remove(key(10)).unwrap();
    rules = revision.seal().unwrap();
    rule_state.remove(&key(10));
    let mut run = runner.settle().unwrap();
    oracle.observe(&mut run, &BTreeMap::new(), &[Terminal::Complete]);
    drop((values, rules));
}

fn barrier_snapshot(
    discovered: &BTreeSet<Key<u64>>, completed: &BTreeMap<Key<u64>, u64>,
    rules: &BTreeMap<Key<u64>, Rule>,
) -> BTreeMap<Key<u64>, Vec<(Key<u64>, u64)>> {
    rules
        .iter()
        .filter_map(|(rule_key, rule)| {
            let required: Vec<_> = discovered
                .iter()
                .filter(|key| key.try_as_id().is_ok_and(|id| *id <= rule.0))
                .collect();
            required
                .iter()
                .all(|key| completed.contains_key(*key))
                .then(|| {
                    let values = required
                        .into_iter()
                        .map(|key| (key.clone(), *completed.get(key).unwrap()))
                        .collect();
                    (rule_key.clone(), values)
                })
        })
        .collect()
}

#[test]
fn dynamic_barrier_reopens_and_closes_against_reference_state() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let discovered = workflow.input::<()>();
        let completed = workflow.input::<u64>();
        let rules = workflow.input::<Rule>();
        let barrier = discovered.barrier(&completed, &rules, |rule: &Rule| {
            let maximum = rule.0;
            move |key: &Key<u64>| key.try_as_id().is_ok_and(|id| *id <= maximum)
        });
        workflow.output(&barrier);
    });
    let endpoints: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut discovered = runner.input_at::<()>(endpoints[0]).unwrap();
    let mut completed = runner.input_at::<u64>(endpoints[1]).unwrap();
    let mut rules = runner.input_at::<Rule>(endpoints[2]).unwrap();
    let mut discovered_state = BTreeSet::new();
    let mut completed_state = BTreeMap::new();
    let mut rule_state = BTreeMap::new();
    let mut oracle = Differential::new("barrier");

    let mut revision = rules.begin().unwrap();
    revision.insert(key(10), Rule(2)).unwrap();
    rules = revision.seal().unwrap();
    rule_state.insert(key(10), Rule(2));
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &barrier_snapshot(&discovered_state, &completed_state, &rule_state),
        &[Terminal::Complete],
    );

    let mut revision = discovered.begin().unwrap();
    revision.insert(key(1), ()).unwrap();
    revision.insert(key(2), ()).unwrap();
    discovered = revision.seal().unwrap();
    discovered_state.extend([key(1), key(2)]);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &barrier_snapshot(&discovered_state, &completed_state, &rule_state),
        &[Terminal::Complete],
    );

    for (id, value) in [(1, 100), (2, 200)] {
        let mut revision = completed.begin().unwrap();
        revision.insert(key(id), value).unwrap();
        completed = revision.seal().unwrap();
        completed_state.insert(key(id), value);
        let mut run = runner.settle().unwrap();
        oracle.observe(
            &mut run,
            &barrier_snapshot(&discovered_state, &completed_state, &rule_state),
            &[Terminal::Complete],
        );
    }

    // Late discovery and a wider rule converge before settlement and reopen
    // the previously published barrier.
    let mut discovered_revision = discovered.begin().unwrap();
    let mut rule_revision = rules.begin().unwrap();
    discovered_revision.insert(key(3), ()).unwrap();
    rule_revision.insert(key(10), Rule(3)).unwrap();
    discovered = discovered_revision.seal().unwrap();
    rules = rule_revision.seal().unwrap();
    discovered_state.insert(key(3));
    rule_state.insert(key(10), Rule(3));
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &barrier_snapshot(&discovered_state, &completed_state, &rule_state),
        &[Terminal::Complete, Terminal::Complete],
    );

    let mut revision = completed.begin().unwrap();
    revision.insert(key(3), 300).unwrap();
    completed = revision.seal().unwrap();
    completed_state.insert(key(3), 300);
    let mut run = runner.settle().unwrap();
    oracle.observe(
        &mut run,
        &barrier_snapshot(&discovered_state, &completed_state, &rule_state),
        &[Terminal::Complete],
    );

    let mut revision = completed.begin().unwrap();
    revision.remove(key(2)).unwrap();
    completed = revision.abort().unwrap();
    let snapshot =
        barrier_snapshot(&discovered_state, &completed_state, &rule_state);
    let mut run = runner.settle().unwrap();
    oracle.observe(&mut run, &snapshot, &[Terminal::Aborted]);
    let mut quiescent = runner.settle().unwrap();
    oracle.observe(&mut quiescent, &snapshot, &[]);

    drop((discovered, completed, rules));
}

#[test]
fn provider_and_runner_share_linearizable_revision_accounting() {
    const REVISIONS: u64 = 128;

    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let (opened_send, opened_receive) = mpsc::channel();
    let (command_send, command_receive) = mpsc::channel();
    let (closed_send, closed_receive) = mpsc::channel();
    let (drained_send, drained_receive) = mpsc::channel();
    let mut expected = BTreeMap::new();
    let mut oracle = Differential::new("provider concurrency");

    thread::scope(|scope| {
        scope.spawn(move || {
            let mut input = input;
            for revision in 0..REVISIONS {
                let mut open = input.begin().unwrap();
                open.insert(key(revision), revision).unwrap();
                opened_send.send(revision).unwrap();
                let command = command_receive.recv().unwrap();
                input = match command {
                    ProviderCommand::Seal => open.seal().unwrap(),
                    ProviderCommand::Abort => open.abort().unwrap(),
                };
                closed_send.send(command).unwrap();
                drained_receive.recv().unwrap();
            }
        });

        for revision in 0..REVISIONS {
            assert_eq!(opened_receive.recv().unwrap(), revision);
            assert!(
                matches!(runner.settle(), Err(Error::Open(1))),
                "runner settled while provider revision {revision} was open",
            );

            let command = if revision % 4 == 3 {
                ProviderCommand::Abort
            } else {
                ProviderCommand::Seal
            };
            command_send.send(command).unwrap();
            let command = closed_receive.recv().unwrap();
            let terminal = match command {
                ProviderCommand::Seal => {
                    expected.insert(key(revision), revision);
                    Terminal::Complete
                }
                ProviderCommand::Abort => Terminal::Aborted,
            };
            let mut run = runner.settle().unwrap();
            oracle.observe(&mut run, &expected, &[terminal]);
            drained_send.send(()).unwrap();
        }
    });

    let mut quiescent = runner.settle().unwrap();
    oracle.observe(&mut quiescent, &expected, &[]);
}
