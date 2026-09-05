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

//! Stream execution tests.

use zrx_executor::strategy::{Immediate, WorkSharing};
use zrx_scheduler::{Report, Settlement};
use zrx_stream::{Advance, Change, Error, Key, Workflow};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[test]
fn reusable_runner_settles_after_repeated_submission_rejection() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    use std::time::Duration;
    use zrx_executor::{Error, Strategy, task::Task};

    #[derive(Debug)]
    struct RejectFirst(Arc<AtomicUsize>);

    impl Strategy for RejectFirst {
        fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
            if self.0.fetch_add(1, Ordering::Relaxed) < 3 {
                return Err(Error::Submit(task));
            }
            task.execute().execute();
            Ok(())
        }

        fn num_workers(&self) -> usize {
            1
        }
        fn num_tasks_running(&self) -> usize {
            0
        }
        fn num_tasks_pending(&self) -> usize {
            0
        }
        fn capacity(&self) -> usize {
            1
        }
    }

    let attempts = Arc::new(AtomicUsize::new(0));
    let strategy = RejectFirst(Arc::clone(&attempts));
    let (completed, waiting) = mpsc::channel();
    let thread = std::thread::spawn(move || {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<u64>();
            workflow.output(&input);
        });
        let mut runner = workflow.runner_with(strategy).unwrap();
        let mut revision = runner.input::<u64>().unwrap().begin().unwrap();
        revision.insert(Key::from(1), 42).unwrap();
        let _input = revision.seal().unwrap();
        let mut run = runner.settle().unwrap();
        assert!(matches!(
            run.report().settlements(),
            [Settlement::Complete(_)]
        ));
        let changes: Vec<_> = run.output::<u64>().unwrap().collect();
        assert!(
            matches!(changes.as_slice(), [Change::Insert(key, 42)] if key == &Key::from(1))
        );
        completed.send(()).unwrap();
    });
    waiting
        .recv_timeout(Duration::from_secs(2))
        .expect("runner waited for a nonexistent completion");
    thread.join().unwrap();
    assert_eq!(attempts.load(Ordering::Relaxed), 4);
}

#[test]
fn reusable_runner_advances_while_a_revision_is_open() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let revision = input.begin().unwrap();

    assert!(matches!(runner.advance().unwrap(), Advance::Progress(_)));
    assert!(matches!(runner.advance().unwrap(), Advance::Idle));

    drop(revision);
    loop {
        if matches!(runner.advance().unwrap(), Advance::Settled) {
            break;
        }
    }
}

#[test]
fn reusable_runner_pumps_an_input_larger_than_session_capacity() {
    const ITEMS: usize = 70_000;

    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
    });
    let output = workflow.output::<u64>().unwrap();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let mut revision = input.begin().unwrap();
    let mut changes = (0..ITEMS).map(|key| {
        let key = u64::try_from(key).unwrap();
        Change::Insert(Key::from(key), key)
    });
    let mut submitted = 0;
    let mut received = 0;

    loop {
        let count = revision.emit_from(&mut changes).unwrap();
        if count == 0 {
            break;
        }
        submitted += count;
        loop {
            match runner.advance().unwrap() {
                Advance::Output(batch) => {
                    assert_eq!(batch.output(), output.id());
                    received += batch.into_changes::<u64>().count();
                }
                Advance::Progress(_) => {}
                Advance::Idle => break,
                Advance::Settled => {
                    panic!("open revision settled before it was sealed")
                }
            }
        }
    }
    assert_eq!(submitted, ITEMS);
    assert!(received != 0, "open revision emitted no incremental output");

    let _input = revision.seal().unwrap();
    let mut report = Report::default();
    loop {
        match runner.advance().unwrap() {
            Advance::Output(batch) => {
                assert_eq!(batch.output(), output.id());
                received += batch.into_changes::<u64>().count();
            }
            Advance::Progress(next) => report.append(next),
            Advance::Settled => break,
            Advance::Idle => panic!("sealed revision stalled"),
        }
    }

    assert_eq!(received, ITEMS);
    assert!(matches!(report.settlements(), [Settlement::Complete(_)]));
}

#[test]
fn reusable_runner_exposes_multiple_outputs_incrementally() {
    const ITEMS: usize = 2_500;

    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
        workflow.output(&input);
    });
    let outputs: Vec<_> = workflow.outputs().copied().collect();
    let mut runner = workflow.runner_with(WorkSharing::new(2)).unwrap();
    let input = runner.input::<u64>().unwrap();
    let mut revision = input.begin().unwrap();
    let mut changes = (0..ITEMS).map(|key| {
        let key = u64::try_from(key).unwrap();
        Change::Insert(Key::from(key), key)
    });
    let mut received = [0, 0];

    loop {
        let count = revision.emit_from(&mut changes).unwrap();
        if count == 0 {
            break;
        }
        loop {
            match runner.advance().unwrap() {
                Advance::Output(batch) => {
                    let index = outputs
                        .iter()
                        .position(|output| output.id() == batch.output())
                        .expect("egress belongs to one declared output");
                    received[index] += batch.into_changes::<u64>().count();
                }
                Advance::Progress(_) => {}
                Advance::Idle => break,
                Advance::Settled => {
                    panic!("open revision settled before it was sealed")
                }
            }
        }
    }
    assert!(received.iter().all(|count| *count != 0));

    let _input = revision.seal().unwrap();
    loop {
        match runner.advance().unwrap() {
            Advance::Output(batch) => {
                let index = outputs
                    .iter()
                    .position(|output| output.id() == batch.output())
                    .expect("egress belongs to one declared output");
                received[index] += batch.into_changes::<u64>().count();
            }
            Advance::Progress(_) => {}
            Advance::Settled => break,
            Advance::Idle => panic!("sealed revision stalled"),
        }
    }

    assert_eq!(received, [ITEMS; 2]);
}

#[test]
fn settle_with_visits_multiple_outputs_incrementally() {
    const ITEMS: usize = 2_500;

    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
        workflow.output(&input);
    });
    let outputs: Vec<_> = workflow.outputs().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let mut revision = input.begin().unwrap();
    for key in 0..ITEMS {
        let key = u64::try_from(key).unwrap();
        revision.insert(Key::from(key), key).unwrap();
    }
    let _input = revision.seal().unwrap();
    let mut received = [0, 0];

    let report = runner
        .settle_with(|batch| {
            let index = outputs
                .iter()
                .position(|output| output.id() == batch.output())
                .expect("egress belongs to one declared output");
            received[index] += batch.into_changes::<u64>().count();
        })
        .unwrap();

    assert_eq!(received, [ITEMS; 2]);
    assert!(matches!(report.settlements(), [Settlement::Complete(_)]));
}

#[test]
fn settle_with_supports_a_workflow_without_outputs() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let _input = workflow.input::<u64>();
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 10).unwrap();
    let _input = revision.seal().unwrap();
    let mut outputs = 0;

    let report = runner.settle_with(|_| outputs += 1).unwrap();

    assert_eq!(outputs, 0);
    assert!(matches!(report.settlements(), [Settlement::Complete(_)]));
}

#[test]
fn settle_with_rejects_an_open_revision_before_visiting_output() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let revision = input.begin().unwrap();
    let mut outputs = 0;

    assert!(matches!(
        runner.settle_with(|_| outputs += 1),
        Err(Error::Open(1))
    ));
    assert_eq!(outputs, 0);

    drop(revision);
    let report = runner.settle_with(|_| outputs += 1).unwrap();
    assert_eq!(outputs, 0);
    assert!(matches!(report.settlements(), [Settlement::Aborted(_)]));
}
