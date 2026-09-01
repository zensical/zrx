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

//! Group-by-key tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Run, Workflow};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type OutputChange = (Vec<u64>, Option<u32>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<OutputChange> {
    run.output::<u32>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => {
                (key.iter().copied().collect(), Some(value))
            }
            Change::Remove(key) => (key.iter().copied().collect(), None),
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

// ----------------------------------------------------------------------------

#[test]
fn group_by_key_moves_and_retracts_source_owned_memberships() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let grouped =
            source.group_by_key(|value: &u32| Key::from(u64::from(*value)));
        workflow.output(&grouped);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![10, 1], Some(10))]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(vec![10, 1], None), (vec![20, 1], Some(20))]
    );

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(vec![20, 1], None)]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn group_by_key_rejects_a_flattened_key_collision() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<Vec<u64>>();
        let grouped = source.group_by_key(|group: &Vec<u64>| {
            group.iter().copied().collect::<Key<_>>()
        });
        workflow.output(&grouped);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<Vec<u64>>().unwrap();

    let mut revision = input.begin().unwrap();
    revision
        .insert([2_u64, 3].into_iter().collect(), vec![1_u64])
        .unwrap();
    revision.insert(Key::from(3_u64), vec![1_u64, 2]).unwrap();
    let input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    let values: Vec<_> = run
        .output::<Vec<u64>>()
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
    assert_eq!(values, [(vec![1, 2, 3], Some(vec![1_u64]))]);
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].key(), &Key::from(3_u64));

    drop(input);
}
