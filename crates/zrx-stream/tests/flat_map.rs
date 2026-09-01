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

//! Flat-map tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Run, Workflow};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type OutputChange = (Vec<u64>, Option<u64>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<OutputChange> {
    let mut values: Vec<_> = run
        .output::<u64>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => {
                (key.iter().copied().collect::<Vec<_>>(), Some(value))
            }
            Change::Remove(key) => {
                (key.iter().copied().collect::<Vec<_>>(), None)
            }
        })
        .collect();
    values.sort_by(|left, right| left.0.cmp(&right.0));
    values
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
fn flat_map_replaces_and_retracts_each_source_owned_set() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<Vec<u64>>();
        let expanded = source.flat_map(|values: &Vec<u64>| {
            values
                .iter()
                .map(|value| (Key::from(*value), *value))
                .collect::<Vec<_>>()
        });
        workflow.output(&expanded);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<Vec<u64>>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), vec![10, 11]).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![1, 10], Some(10)), (vec![1, 11], Some(11)),]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), vec![11, 12]).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [
            (vec![1, 10], None),
            (vec![1, 11], Some(11)),
            (vec![1, 12], Some(12)),
        ]
    );

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![1, 11], None), (vec![1, 12], None)]
    );

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn flat_map_rejects_duplicate_members_without_replacing_the_old_set() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<Vec<u64>>();
        let expanded = source.flat_map(|values: &Vec<u64>| {
            values
                .iter()
                .map(|value| (Key::from(*value), *value))
                .collect::<Vec<_>>()
        });
        workflow.output(&expanded);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<Vec<u64>>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), vec![10]).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![1, 10], Some(10))]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), vec![20, 20]).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].key(), &Key::from(1_u64));

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(vec![1, 10], None)]);
    assert!(runner.errors().is_empty());

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn flat_map_rejects_a_competing_source_owner() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<Vec<Vec<u64>>>();
        let expanded = source.flat_map(|suffixes: &Vec<Vec<u64>>| {
            suffixes
                .iter()
                .map(|suffix| {
                    (suffix.iter().copied().collect::<Key<_>>(), 1_u64)
                })
                .collect::<Vec<_>>()
        });
        workflow.output(&expanded);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<Vec<Vec<u64>>>().unwrap();

    let mut revision = input.begin().unwrap();
    revision
        .insert([1_u64].into_iter().collect(), vec![vec![2, 3]])
        .unwrap();
    revision
        .insert([1_u64, 2].into_iter().collect(), vec![vec![3]])
        .unwrap();
    let input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(vec![1, 2, 3], Some(1))]);
    assert_eq!(failures(&run), 1);

    drop(input);
}
