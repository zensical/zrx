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

//! Product tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Run, Workflow};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type OutputChange = (Vec<u64>, Option<(String, u64)>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<OutputChange> {
    let mut values: Vec<_> = run
        .output::<(String, u64)>()
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

// ----------------------------------------------------------------------------

#[test]
fn product_reconciles_independent_lanes_and_retracts_pairs() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let product = left.product(&right);
        workflow.output(&product);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "a".to_owned()).unwrap();
    revision.insert(Key::from(2_u64), "b".to_owned()).unwrap();
    let mut left = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(3_u64), 30).unwrap();
    revision.insert(Key::from(4_u64), 40).unwrap();
    let right = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [
            (vec![1, 3], Some(("a".to_owned(), 30))),
            (vec![1, 4], Some(("a".to_owned(), 40))),
            (vec![2, 3], Some(("b".to_owned(), 30))),
            (vec![2, 4], Some(("b".to_owned(), 40))),
        ]
    );

    let mut revision = left.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    left = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![1, 3], None), (vec![1, 4], None)]
    );

    drop((left, right));
}

// ----------------------------------------------------------------------------

#[test]
fn product_publishes_each_ready_overlapping_pair_transition_once() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        workflow.output(&left.product(&right));
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut left_revision = left.begin().unwrap();
    left_revision
        .insert(Key::from(1_u64), "first".into())
        .unwrap();
    let mut left = left_revision.seal().unwrap();
    let mut right_revision = right.begin().unwrap();
    right_revision.insert(Key::from(2_u64), 10).unwrap();
    let mut right = right_revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![1, 2], Some((String::from("first"), 10)))]
    );

    let mut left_revision = left.begin().unwrap();
    let mut right_revision = right.begin().unwrap();
    left_revision
        .insert(Key::from(1_u64), "second".into())
        .unwrap();
    right_revision.insert(Key::from(2_u64), 20).unwrap();
    left = left_revision.seal().unwrap();
    right = right_revision.seal().unwrap();

    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [
            (vec![1, 2], Some((String::from("second"), 10))),
            (vec![1, 2], Some((String::from("second"), 20))),
        ]
    );
    drop((left, right));
}

// ----------------------------------------------------------------------------

#[test]
fn product_reports_ambiguous_flattened_keys_without_committing_input() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let product = left.product(&right);
        workflow.output(&product);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    revision
        .insert([1_u64].into_iter().collect(), "a".to_owned())
        .unwrap();
    revision
        .insert([1_u64, 2].into_iter().collect(), "b".to_owned())
        .unwrap();
    let left = revision.seal().unwrap();

    let mut revision = right.begin().unwrap();
    revision
        .insert([2_u64, 3].into_iter().collect(), 20)
        .unwrap();
    let mut right = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()).len(), 2);

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(3_u64), 30).unwrap();
    right = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(
        run.report()
            .invocations()
            .iter()
            .map(|invocation| invocation.outcomes.error_count())
            .sum::<usize>(),
        1
    );
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].key(), &Key::from(3_u64));

    let mut revision = right.begin().unwrap();
    revision.remove(Key::from(3_u64)).unwrap();
    right = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());
    assert!(runner.errors().is_empty());

    drop((left, right));
}
