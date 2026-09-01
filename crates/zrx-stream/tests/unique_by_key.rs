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

//! Unique-secondary-index tests.

use std::collections::BTreeMap;

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Run, Workflow};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type OutputChange = (Key<u64>, Option<u32>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn failures(run: &Run<u64>) -> usize {
    run.report()
        .invocations()
        .iter()
        .map(|invocation| invocation.outcomes.error_count())
        .sum()
}

fn values(run: &mut Run<u64>) -> Vec<OutputChange> {
    run.output::<u32>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => (key, Some(value)),
            Change::Remove(key) => (key, None),
        })
        .collect()
}

fn state(changes: &[OutputChange]) -> BTreeMap<Key<u64>, u32> {
    let mut state = BTreeMap::new();
    for (key, value) in changes {
        match value {
            Some(value) => {
                state.insert(key.clone(), *value);
            }
            None => {
                state.remove(key);
            }
        }
    }
    state
}

// ----------------------------------------------------------------------------

#[test]
fn unique_by_key_moves_and_retracts_claims() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let indexed =
            source.unique_by_key(|value: &u32| Key::from(u64::from(*value)));
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(10_u64), Some(10))]);
    assert_eq!(failures(&run), 0);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(
        values(&mut run),
        [(Key::from(10_u64), None), (Key::from(20_u64), Some(20))]
    );
    assert_eq!(failures(&run), 0);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(20_u64), None)]);
    assert_eq!(failures(&run), 0);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn unique_by_key_collisions_are_order_independent_within_one_revision() {
    for (first, second, removed, remaining) in [
        ((1, 10), (2, 20), 1, 20),
        ((1, 10), (2, 20), 2, 10),
        ((2, 20), (1, 10), 1, 20),
        ((2, 20), (1, 10), 2, 10),
    ] {
        let workflow = Workflow::<u64>::build(|workflow| {
            let source = workflow.input::<u32>();
            let indexed = source
                .unique_by_key(|value: &u32| Key::from(u64::from(*value % 10)));
            workflow.output(&indexed);
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let input = runner.input::<u32>().unwrap();

        let mut revision = input.begin().unwrap();
        revision.insert(Key::from(first.0), first.1).unwrap();
        revision.insert(Key::from(second.0), second.1).unwrap();
        let mut input = revision.seal().unwrap();
        let mut run = runner.settle().unwrap();
        let changes = values(&mut run);
        assert_eq!(state(&changes), BTreeMap::new());
        assert_eq!(failures(&run), 1);

        let mut revision = input.begin().unwrap();
        revision.remove(Key::from(removed)).unwrap();
        input = revision.seal().unwrap();
        let mut run = runner.settle().unwrap();
        assert_eq!(values(&mut run), [(Key::from(0_u64), Some(remaining))]);
        assert_eq!(failures(&run), 0);

        drop(input);
    }
}

// ----------------------------------------------------------------------------

#[test]
fn unique_by_key_collision_across_revisions_recovers() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let indexed = source
            .unique_by_key(|value: &u32| Key::from(u64::from(*value % 10)));
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(0_u64), Some(10))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(0_u64), None)]);
    assert_eq!(failures(&run), 1);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(2_u64)).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(0_u64), Some(10))]);
    assert_eq!(failures(&run), 0);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(0_u64), None)]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(3_u64), 30).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(0_u64), Some(30))]);
    assert_eq!(failures(&run), 0);

    let mut quiescent = runner.settle().unwrap();
    assert!(values(&mut quiescent).is_empty());
    assert_eq!(failures(&quiescent), 0);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn unique_by_key_replaces_and_moves_claims_through_conflict() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let indexed = source
            .unique_by_key(|value: &u32| Key::from(u64::from(*value % 10)));
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    revision.insert(Key::from(2_u64), 11).unwrap();
    let mut input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(
        values(&mut run),
        [(Key::from(0_u64), Some(10)), (Key::from(1_u64), Some(11))]
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(0_u64), Some(20))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2_u64), 30).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(
        values(&mut run),
        [(Key::from(1_u64), None), (Key::from(0_u64), None)]
    );
    assert_eq!(failures(&run), 1);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2_u64), 31).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(
        values(&mut run),
        [(Key::from(0_u64), Some(20)), (Key::from(1_u64), Some(31))]
    );
    assert_eq!(failures(&run), 0);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn unique_by_key_callback_failure_preserves_the_accepted_claim() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let indexed = source.unique_by_key(|value: &u32| {
            anyhow::ensure!(*value != 99, "derived key rejected");
            Ok::<_, anyhow::Error>(Key::from(u64::from(*value)))
        });
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(10_u64), Some(10))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 99).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(runner.errors()[0].key(), &Key::from(1_u64));

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(
        values(&mut run),
        [(Key::from(10_u64), None), (Key::from(20_u64), Some(20))]
    );
    assert_eq!(failures(&run), 0);
    assert!(runner.errors().is_empty());

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn callback_and_uniqueness_errors_with_the_same_key_recover_independently() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let indexed = source.unique_by_key(|value: &u32| {
            anyhow::ensure!(*value != 99, "derivation rejected");
            Ok::<_, anyhow::Error>(Key::from(u64::from(*value % 10)))
        });
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(2_u64), 11).unwrap();
    revision.insert(Key::from(3_u64), 21).unwrap();
    revision.insert(Key::from(1_u64), 99).unwrap();
    let mut input = revision.seal().unwrap();
    let _run = runner.settle().unwrap();
    assert_eq!(runner.errors().len(), 2);
    assert!(
        runner
            .errors()
            .iter()
            .all(|error| error.key() == &Key::from(1_u64))
    );

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 32).unwrap();
    input = revision.seal().unwrap();
    let _run = runner.settle().unwrap();
    assert_eq!(runner.errors().len(), 1);
    assert_eq!(
        runner.errors()[0].error().to_string(),
        "unique_by_key derived key is not unique"
    );

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(3_u64)).unwrap();
    let _input = revision.seal().unwrap();
    let _run = runner.settle().unwrap();
    assert!(runner.errors().is_empty());
}

// ----------------------------------------------------------------------------

#[test]
fn unique_by_key_aborted_change_does_not_block_later_repair() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let indexed =
            source.unique_by_key(|value: &u32| Key::from(u64::from(*value)));
        workflow.output(&indexed);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.abort().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 0);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(Key::from(20_u64), Some(20))]);

    drop(input);
}
