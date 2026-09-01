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

//! Join tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{
    Advance, Change, Key, StreamSetExt, StreamTupleExt, Value, Workflow,
};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type OutputChange<T> = (Key<u64>, Option<T>);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn values<T>(run: &mut zrx_stream::Run<u64>) -> Vec<OutputChange<T>>
where
    T: Value,
{
    run.output::<T>()
        .unwrap()
        .map(|change| match change {
            Change::Insert(key, value) => (key, Some(value)),
            Change::Remove(key) => (key, None),
        })
        .collect()
}

// ----------------------------------------------------------------------------

#[test]
fn join_publishes_only_the_latest_overlapping_key_transition() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        workflow.output(&(left, right).join());
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut left_revision = left.begin().unwrap();
    let mut right_revision = right.begin().unwrap();
    left_revision
        .insert(Key::from(1_u64), "one".to_owned())
        .unwrap();
    right_revision.insert(Key::from(1_u64), 10).unwrap();
    let left = left_revision.seal().unwrap();
    let right = right_revision.seal().unwrap();

    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some(("one".to_owned(), 10_u64)))]
    );
    drop((left, right));
}

#[test]
fn aborted_dispatched_state_can_affect_a_later_revision() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        workflow.output(&(left, right).join());
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    let mut changes = (0..1_024_u64)
        .map(|key| Change::Insert(Key::from(key), format!("left-{key}")));
    assert_eq!(revision.emit_from(&mut changes).unwrap(), 1_024);
    loop {
        match runner.advance().unwrap() {
            Advance::Output(_) => panic!("join published without right input"),
            Advance::Progress(_) => {}
            Advance::Idle => break,
            Advance::Settled => panic!("open revision settled"),
        }
    }
    let left = revision.abort().unwrap();
    assert!(values::<(String, u64)>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(0_u64), 7_u64).unwrap();
    let right = revision.seal().unwrap();
    assert_eq!(
        values::<(String, u64)>(&mut runner.settle().unwrap()),
        [(Key::from(0_u64), Some((String::from("left-0"), 7_u64)))]
    );
    drop((left, right));
}

// ----------------------------------------------------------------------------

#[test]
fn coalesce_preserves_a_visible_change_across_overlapping_hidden_work() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let preferred = workflow.input::<String>();
        let alternate = workflow.input::<String>();
        workflow.output(&(preferred, alternate).coalesce());
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let preferred = runner.input_at::<String>(inputs[0]).unwrap();
    let alternate = runner.input_at::<String>(inputs[1]).unwrap();

    let mut preferred_revision = preferred.begin().unwrap();
    let mut alternate_revision = alternate.begin().unwrap();
    preferred_revision
        .insert(Key::from(1_u64), "preferred".into())
        .unwrap();
    alternate_revision
        .insert(Key::from(1_u64), "hidden".into())
        .unwrap();
    let preferred = preferred_revision.seal().unwrap();
    let alternate = alternate_revision.seal().unwrap();

    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some(String::from("preferred")))]
    );
    drop((preferred, alternate));
}

// ----------------------------------------------------------------------------

#[test]
fn left_join_replaces_optional_right_values() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let joined = (left, right).left_join();
        workflow.output(&joined);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut right = revision.seal().unwrap();
    assert!(
        values::<(String, Option<u64>)>(&mut runner.settle().unwrap())
            .is_empty()
    );

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "a".to_owned()).unwrap();
    let mut left = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some(("a".to_owned(), Some(10_u64))))]
    );

    let mut revision = right.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    right = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some(("a".to_owned(), None::<u64>)))]
    );

    let mut revision = left.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    left = revision.seal().unwrap();
    assert_eq!(
        values::<(String, Option<u64>)>(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), None)]
    );

    drop((left, right));
}

#[test]
fn full_join_tracks_presence_in_every_lane() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let joined = (left, right).full_join();
        workflow.output(&joined);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "a".to_owned()).unwrap();
    let mut left = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some((Some("a".to_owned()), None::<u64>)))]
    );

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut right = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some((Some("a".to_owned()), Some(10_u64))))]
    );

    let mut revision = left.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    left = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some((None::<String>, Some(10_u64))))]
    );

    let mut revision = right.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    right = revision.seal().unwrap();
    assert_eq!(
        values::<(Option<String>, Option<u64>)>(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), None)]
    );

    drop((left, right));
}

#[test]
fn semi_join_tracks_matching_membership_only() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let joined = (left, right).semi_join();
        workflow.output(&joined);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "a".to_owned()).unwrap();
    let left = revision.seal().unwrap();
    assert!(values::<String>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut right = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("a".to_owned()))]
    );

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    right = revision.seal().unwrap();
    assert!(values::<String>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    right = revision.seal().unwrap();
    assert_eq!(
        values::<String>(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), None)]
    );

    drop((left, right));
}

#[test]
fn anti_join_tracks_absent_membership_only() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let joined = (left, right).anti_join();
        workflow.output(&joined);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "a".to_owned()).unwrap();
    let left = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("a".to_owned()))]
    );

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let mut right = revision.seal().unwrap();
    assert_eq!(
        values::<String>(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), None)]
    );

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 20).unwrap();
    right = revision.seal().unwrap();
    assert!(values::<String>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    right = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("a".to_owned()))]
    );

    drop((left, right));
}

#[test]
fn coalesce_emits_only_visible_priority_changes() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let preferred = workflow.input::<String>();
        let alternate = workflow.input::<String>();
        let fallback = workflow.input::<String>();
        let selected = (preferred, alternate, fallback).coalesce();
        workflow.output(&selected);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let preferred = runner.input_at::<String>(inputs[0]).unwrap();
    let alternate = runner.input_at::<String>(inputs[1]).unwrap();
    let fallback = runner.input_at::<String>(inputs[2]).unwrap();

    let mut revision = fallback.begin().unwrap();
    revision
        .insert(Key::from(1_u64), "fallback".to_owned())
        .unwrap();
    let mut fallback = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("fallback".to_owned()))]
    );

    let mut revision = alternate.begin().unwrap();
    revision
        .insert(Key::from(1_u64), "alternate".to_owned())
        .unwrap();
    let mut alternate = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("alternate".to_owned()))]
    );

    let mut revision = fallback.begin().unwrap();
    revision
        .insert(Key::from(1_u64), "hidden".to_owned())
        .unwrap();
    fallback = revision.seal().unwrap();
    assert!(values::<String>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = preferred.begin().unwrap();
    revision
        .insert(Key::from(1_u64), "preferred".to_owned())
        .unwrap();
    let mut preferred = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("preferred".to_owned()))]
    );

    let mut revision = alternate.begin().unwrap();
    revision
        .insert(Key::from(1_u64), "updated".to_owned())
        .unwrap();
    alternate = revision.seal().unwrap();
    assert!(values::<String>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = preferred.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    preferred = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("updated".to_owned()))]
    );

    let mut revision = alternate.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    alternate = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some("hidden".to_owned()))]
    );

    let mut revision = fallback.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    fallback = revision.seal().unwrap();
    assert_eq!(
        values::<String>(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), None)]
    );

    drop((preferred, alternate, fallback));
}

// ----------------------------------------------------------------------------

#[test]
fn join_reconciles_independent_lanes_and_retains_only_semantic_state() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let left = workflow.input::<String>();
        let right = workflow.input::<u64>();
        let joined = (left, right).join();
        workflow.output(&joined);
    });
    let inputs: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let left = runner.input_at::<String>(inputs[0]).unwrap();
    let right = runner.input_at::<u64>(inputs[1]).unwrap();

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "a".to_owned()).unwrap();
    let mut left = revision.seal().unwrap();
    assert!(values::<(String, u64)>(&mut runner.settle().unwrap()).is_empty());

    let mut revision = right.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    revision.insert(Key::from(2_u64), 20).unwrap();
    let mut right = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some(("a".to_owned(), 10_u64)))]
    );

    let mut revision = left.begin().unwrap();
    revision.insert(Key::from(1_u64), "b".to_owned()).unwrap();
    left = revision.seal().unwrap();
    assert_eq!(
        values(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), Some(("b".to_owned(), 10_u64)))]
    );

    let mut revision = right.begin().unwrap();
    revision.remove(Key::from(1_u64)).unwrap();
    right = revision.seal().unwrap();
    assert_eq!(
        values::<(String, u64)>(&mut runner.settle().unwrap()),
        [(Key::from(1_u64), None)]
    );

    drop((left, right));
}
