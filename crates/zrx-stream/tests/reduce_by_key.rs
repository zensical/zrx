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

//! Reduce-by-key tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use zrx_executor::strategy::Immediate;
use zrx_stream::function::{Collection, Scope};
use zrx_stream::{Change, Key, Run, StreamTupleExt, Workflow};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type OutputChange = (u64, Option<u32>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values(run: &mut Run<u64>) -> Vec<OutputChange> {
    run.output::<u32>()
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

// ----------------------------------------------------------------------------

#[test]
fn reduce_by_key_flushes_complete_dirty_groups_at_revision_end() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>().map(|value: &u32| *value);
        let reduced = source.reduce_by_key(
            |value: &u32| Key::from(u64::from(*value % 2)),
            |members: &dyn Collection<Key<u64>, u32>| {
                Some(members.values().copied().sum::<u32>())
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    revision.insert(Key::from(2_u64), 12).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(22))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 11).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(0_u64, Some(12)), (1_u64, Some(11))]
    );

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(2_u64)).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, None)]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_by_key_preserves_a_failed_aggregate_until_a_relevant_change() {
    let reject = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&reject);
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let reduced = source.reduce_by_key(
            |_: &u32| Key::from(0_u64),
            move |members: &dyn Collection<Key<u64>, u32>| {
                if observed.swap(false, Ordering::Relaxed) {
                    anyhow::bail!("aggregate rejected")
                }
                Ok(Some(members.values().copied().sum::<u32>()))
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(10))]);

    reject.store(true, Ordering::Relaxed);
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);

    let revision = input.begin().unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 0);
    assert_eq!(runner.errors().len(), 1);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(0_u64, Some(20))]);
    assert_eq!(failures(&run), 0);
    assert!(runner.errors().is_empty());

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_by_key_preserves_the_previous_group_after_selector_failure() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let reduced = source.reduce_by_key(
            |value: &u32| {
                if *value == 99 {
                    anyhow::bail!("group rejected")
                }
                Ok(Key::from(u64::from(*value % 2)))
            },
            |members: &dyn Collection<Key<u64>, u32>| {
                Some(members.values().copied().sum::<u32>())
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(10))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 99).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);

    let revision = input.begin().unwrap();
    input = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 11).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(0_u64, None), (1_u64, Some(11))]);
    assert_eq!(failures(&run), 0);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_by_key_recomputes_remaining_members_after_selector_failure() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let reduced = source.reduce_by_key(
            |value: &u32| {
                if *value == 99 {
                    anyhow::bail!("group rejected")
                }
                Ok(Key::from(0_u64))
            },
            |members: &dyn Collection<Key<u64>, u32>| {
                Some(members.values().copied().sum::<u32>())
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    revision.insert(Key::from(2_u64), 12).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(22))]);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 99).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(values(&mut run).is_empty());
    assert_eq!(failures(&run), 1);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_by_key_flushes_other_groups_when_one_reducer_fails() {
    let reject_even = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&reject_even);
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let reduced = source.reduce_by_key(
            |value: &u32| Key::from(u64::from(*value % 2)),
            move |scope: &mut Scope<'_, u64>,
                  members: &dyn Collection<Key<u64>, u32>| {
                if scope.key() == &Key::from(0_u64)
                    && observed.swap(false, Ordering::Relaxed)
                {
                    anyhow::bail!("even aggregate rejected")
                }
                Ok(Some(members.values().copied().sum::<u32>()))
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    revision.insert(Key::from(2_u64), 11).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(0_u64, Some(10)), (1_u64, Some(11))]
    );

    reject_even.store(true, Ordering::Relaxed);
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    revision.insert(Key::from(2_u64), 21).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(values(&mut run), [(1_u64, Some(21))]);
    assert_eq!(failures(&run), 1);

    let revision = input.begin().unwrap();
    input = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(20))]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_by_key_observes_each_source_through_converging_lanes() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<u16>().map(|value: &u16| u32::from(*value));
        let right = workflow.input::<u32>();
        let joined = (left, right).join();
        let reduced = joined.reduce_by_key(
            |_: &(u32, u32)| Key::from(0_u64),
            |scope: &mut Scope<'_, u64>,
             members: &dyn Collection<Key<u64>, (u32, u32)>| {
                assert_eq!(scope.key(), &Key::from(0_u64));
                Some(
                    members
                        .values()
                        .map(|(left, right)| left + right)
                        .sum::<u32>(),
                )
            },
        );
        workflow.output(&reduced);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input::<u16>().unwrap();
    let right = runner.input::<u32>().unwrap();

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), 1).unwrap();
    let mut left = revision.seal().unwrap();
    assert!(values(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 2).unwrap();
    let right = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(3))]);

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), 2).unwrap();
    left = revision.seal().unwrap();
    assert_eq!(values(&mut runner.settle().unwrap()), [(0_u64, Some(4))]);

    drop(left);
    drop(right);
}
