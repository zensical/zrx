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

//! Ordered window tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Run, Workflow};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<(u64, Option<u64>)> {
    run.output::<u64>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => {
                (*key.try_as_id().unwrap(), Some(value))
            }
            Change::Remove(key) => (*key.try_as_id().unwrap(), None),
        })
        .collect()
}

fn keyed(run: &mut Run<u64>) -> Vec<(Key<u64>, Option<u64>)> {
    run.output::<u64>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => (key, Some(value)),
            Change::Remove(key) => (key, None),
        })
        .collect()
}

// ----------------------------------------------------------------------------

#[test]
fn take_updates_only_its_ordered_boundary() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input.take(2));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    revision.insert(Key::from(4), 40).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(2, Some(20)), (4, Some(40))]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 10).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(4, None), (1, Some(10))]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(4), 41).unwrap();
    let input = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1)).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(1, None), (4, Some(41))]
    );

    drop(input);
}

#[test]
fn take_last_updates_only_its_ordered_boundary() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input.take_last(2));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    revision.insert(Key::from(4), 40).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(2, Some(20)), (4, Some(40))]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 10).unwrap();
    let input = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(5), 50).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(2, None), (5, Some(50))]
    );

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(5)).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(5, None), (2, Some(20))]
    );

    drop(input);
}

#[test]
fn skip_tracks_items_crossing_its_ordered_boundary() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input.skip(2));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    revision.insert(Key::from(4), 40).unwrap();
    revision.insert(Key::from(6), 60).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(6, Some(60))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1), 10).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(4, Some(40))]);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1)).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(4, None)]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(6), 61).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(6, Some(61))]);

    drop(input);
}

#[test]
fn skip_last_tracks_items_crossing_its_ordered_boundary() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input.skip_last(2));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2), 20).unwrap();
    revision.insert(Key::from(4), 40).unwrap();
    revision.insert(Key::from(6), 60).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(2, Some(20))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(8), 80).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(4, Some(40))]);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(2)).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(2, None)]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(4), 41).unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(4, Some(41))]);

    drop(input);
}

#[test]
fn windows_order_complete_hierarchical_keys() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input.take(2));
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let first: Key<u64> = [1_u64, 2].into_iter().collect();
    let second: Key<u64> = [2_u64, 1].into_iter().collect();

    let mut revision = input.begin().unwrap();
    revision.insert(first.clone(), 12).unwrap();
    revision.insert(second.clone(), 21).unwrap();
    let input = revision.seal().unwrap();
    let initial = keyed(&mut runner.settle().unwrap());
    assert_eq!(
        initial,
        [(first.clone(), Some(12)), (second.clone(), Some(21)),]
    );

    let inserted: Key<u64> = [1_u64, 1].into_iter().collect();
    let mut revision = input.begin().unwrap();
    revision.insert(inserted.clone(), 11).unwrap();
    let input = revision.seal().unwrap();
    let shifted = keyed(&mut runner.settle().unwrap());
    assert_eq!(shifted, [(second, None), (inserted, Some(11))]);

    drop(input);
}
