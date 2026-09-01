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

//! Global reduction and scalar-signal tests.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use zrx_executor::strategy::Immediate;
use zrx_stream::function::{Collection, Scope};
use zrx_stream::{Advance, Change, Key, Run, Signal, Workflow};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn scalar(run: &mut Run<u64>) -> Vec<Option<u32>> {
    run.output::<u32>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => {
                assert!(key.iter().next().is_none());
                Some(value)
            }
            Change::Remove(key) => {
                assert!(key.iter().next().is_none());
                None
            }
        })
        .collect()
}

fn assert_signal(_: &Signal<u64, u32>) {}

fn failures(run: &Run<u64>) -> usize {
    run.report()
        .invocations()
        .iter()
        .map(|invocation| invocation.outcomes.error_count())
        .sum()
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_publishes_one_empty_key_and_signal_map_preserves_it() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let total = source.reduce(
            |scope: &mut Scope<'_, u64>,
             members: &dyn Collection<Key<u64>, u32>| {
                assert!(scope.key().iter().next().is_none());
                Some(members.values().copied().sum::<u32>())
            },
        );
        assert_signal(&total);
        let doubled = total.map(|value: &u32| *value * 2);
        assert_signal(&doubled);
        workflow.output(&doubled);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    revision.insert(Key::from(2_u64), 12).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), [Some(44)]);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    revision.remove(Key::from(2_u64)).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), [Some(0)]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_evaluates_an_empty_first_revision_and_retracts_absence() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let total =
            source.reduce(move |members: &dyn Collection<Key<u64>, u32>| {
                observed.fetch_add(1, Ordering::Relaxed);
                (!members.is_empty())
                    .then(|| members.values().copied().sum::<u32>())
            });
        workflow.output(&total);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let revision = input.begin().unwrap();
    let mut input = revision.seal().unwrap();
    assert!(scalar(&mut runner.settle().unwrap()).is_empty());
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), [Some(10)]);

    let mut revision = input.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    input = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), [None]);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_initializes_after_an_aborted_first_revision() {
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let total =
            source.reduce(move |members: &dyn Collection<Key<u64>, u32>| {
                observed.fetch_add(1, Ordering::Relaxed);
                Some(members.values().copied().sum::<u32>())
            });
        workflow.output(&total);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    // Drive the begin event through the operator before aborting so this tests
    // committed action state, rather than a locally buffered revision.
    let revision = input.begin().unwrap();
    loop {
        match runner.advance().unwrap() {
            Advance::Output(_) => panic!("reduce published an open revision"),
            Advance::Progress(_) => {}
            Advance::Idle => break,
            Advance::Settled => panic!("open revision settled"),
        }
    }
    let input = revision.abort().unwrap();
    assert!(scalar(&mut runner.settle().unwrap()).is_empty());
    assert_eq!(calls.load(Ordering::Relaxed), 0);

    let revision = input.begin().unwrap();
    let input = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), [Some(0)]);
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn reduce_preserves_a_failed_aggregate_until_a_relevant_change() {
    let reject = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&reject);
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u32>();
        let total =
            source.reduce(move |members: &dyn Collection<Key<u64>, u32>| {
                if observed.swap(false, Ordering::Relaxed) {
                    anyhow::bail!("aggregate rejected")
                }
                Ok(Some(members.values().copied().sum::<u32>()))
            });
        workflow.output(&total);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u32>().unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut input = revision.seal().unwrap();
    assert_eq!(scalar(&mut runner.settle().unwrap()), [Some(10)]);

    reject.store(true, Ordering::Relaxed);
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(scalar(&mut run).is_empty());
    assert_eq!(failures(&run), 1);
    assert_eq!(runner.errors().len(), 1);

    let revision = input.begin().unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(scalar(&mut run).is_empty());
    assert_eq!(failures(&run), 0);
    assert_eq!(runner.errors().len(), 1);

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    input = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert_eq!(scalar(&mut run), [Some(20)]);
    assert_eq!(failures(&run), 0);
    assert!(runner.errors().is_empty());

    drop(input);
}

// ----------------------------------------------------------------------------

#[test]
fn product_treats_a_signal_as_a_broadcast_identity() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let pages = workflow.input::<String>();
        let values = workflow.input::<u32>();
        let total = values.reduce(|members: &dyn Collection<Key<u64>, u32>| {
            Some(members.values().copied().sum::<u32>())
        });
        let output = pages.product(&total);
        workflow.output(&output);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let pages = runner.input_at::<String>(inputs[0]).unwrap();
    let values = runner.input_at::<u32>(inputs[1]).unwrap();

    let mut revision = values.begin().unwrap();
    revision.insert(Key::from(1_u64), 3).unwrap();
    let values = revision.seal().unwrap();
    let mut run = runner.settle().unwrap();
    assert!(run.output::<(String, u32)>().unwrap().next().is_none());

    let mut revision = pages.begin().unwrap();
    revision
        .insert(Key::from(7_u64), String::from("page"))
        .unwrap();
    let pages = revision.seal().unwrap();
    let changes: Vec<_> = runner
        .settle()
        .unwrap()
        .output::<(String, u32)>()
        .unwrap()
        .collect();
    assert!(matches!(
        changes.as_slice(),
        [Change::Insert(key, (page, 3))]
            if key == &Key::from(7_u64) && page == "page"
    ));

    drop((pages, values));
}
