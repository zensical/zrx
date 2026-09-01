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

//! Movable invocation package for one committed action batch.

use crate::scheduler::{Id, RevisionId};

use crate::scheduler::action::control::{Event, ProgressEvent};
use crate::scheduler::action::{
    EvaluationChanges, InputSegments, Instrumentation, Job, Outcomes, Segment,
    WakeKey, WakeRequest,
};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Successful result of one committed action batch.
pub struct Completion<I>
where
    I: Id,
{
    /// Installed node whose job ran.
    pub node: usize,
    /// Persistent action state returned to its resident slot.
    pub job: Job<I>,
    /// Sealed output when the node is connected and emitted data.
    pub output: Option<Segment<I>>,
    /// Sparse historical errors.
    pub outcomes: Outcomes,
    /// Ordered current-error changes.
    pub evaluations: EvaluationChanges<I>,
    /// Ordered diagnostics and author annotations.
    pub instrumentation: Instrumentation,
    /// Keyed temporal requests produced by this invocation.
    pub wakes: Vec<WakeRequest>,
}

// ----------------------------------------------------------------------------

/// One job and its committed, revision-homogeneous lane segments.
pub struct Invocation<I>
where
    I: Id,
{
    revision: RevisionId,
    node: usize,
    job: Job<I>,
    inputs: InputSegments<I>,
    event: Option<Event>,
    connected: bool,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Invocation<I>
where
    I: Id,
{
    #[cfg(feature = "tracing")]
    pub fn node(&self) -> usize {
        self.node
    }

    #[cfg(feature = "tracing")]
    pub fn batch_items(&self) -> usize {
        self.inputs
            .as_slice()
            .iter()
            .flatten()
            .map(Segment::len)
            .sum()
    }

    /// Creates one movable action batch from prevalidated plan lanes.
    #[must_use]
    pub fn new(
        revision: RevisionId, node: usize, job: Job<I>,
        inputs: impl IntoIterator<Item = Option<Segment<I>>>, connected: bool,
    ) -> Self {
        let inputs = InputSegments::collect(inputs, job.inputs().len());
        debug_assert!(inputs.as_slice().iter().zip(job.inputs()).all(
            |(input, &port)| {
                input.as_ref().is_none_or(|segment| segment.port() == port)
            }
        ));
        Self {
            revision,
            node,
            job,
            inputs,
            event: None,
            connected,
        }
    }

    /// Creates one movable wake event invocation.
    ///
    #[must_use]
    pub fn wake(
        revision: RevisionId, node: usize, job: Job<I>, key: WakeKey,
        deadline: std::time::Instant, connected: bool,
    ) -> Self {
        let inputs = InputSegments::empty(job.inputs().len());
        Self {
            revision,
            node,
            job,
            inputs,
            event: Some(Event::Wake { key, deadline }),
            connected,
        }
    }

    /// Creates one movable shared progress event invocation.
    #[must_use]
    pub fn progress(
        revision: RevisionId, node: usize, job: Job<I>,
        progress: ProgressEvent, connected: bool,
    ) -> Self {
        let inputs = InputSegments::empty(job.inputs().len());
        Self {
            revision,
            node,
            job,
            inputs,
            event: Some(Event::Progress(progress)),
            connected,
        }
    }

    /// Runs the complete committed batch on the current thread.
    #[must_use]
    pub fn run(mut self) -> Completion<I> {
        let (output, outcomes, evaluations, wakes, instrumentation) = self
            .job
            .run(self.revision, self.inputs, self.event, self.connected);
        Completion {
            node: self.node,
            job: self.job,
            output,
            outcomes,
            evaluations,
            instrumentation,
            wakes,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::panic::AssertUnwindSafe;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex, mpsc};

    use zrx_executor::strategy::{Strategy, WorkSharing};
    use zrx_executor::task::Task;
    use zrx_executor::{Error as ExecutorError, Executor};

    use super::{Completion, Invocation};
    use crate::scheduler::Change;
    use crate::scheduler::RevisionId;
    use crate::scheduler::action::{
        Action, Concurrency, Context, Job, Record, Segment,
    };

    fn input(key: u64, value: u64) -> Segment<u64> {
        Segment::new(vec![Change::Insert(key, value)])
    }

    struct Count(Arc<AtomicUsize>);

    impl Action<u64> for Count {
        type Inputs = (u64,);
        type Output = ();

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context { inputs: input, output, .. } = context;
            input.for_each(output, |_, _| {
                self.0.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });
        }
    }

    #[test]
    fn completion_returns_persistent_job_for_repeated_execution() {
        let calls = Arc::new(AtomicUsize::new(0));
        let execution = Invocation::new(
            RevisionId::test(0),
            7,
            Job::new(Count(Arc::clone(&calls))),
            vec![Some(input(1, 1))],
            false,
        );
        let Completion { node, job, .. } = execution.run();
        assert_eq!(node, 7);

        let second = Invocation::new(
            RevisionId::test(0),
            7,
            job,
            vec![Some(input(2, 2))],
            false,
        );
        let _ = second.run();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[derive(Clone)]
    struct Replicable(Arc<AtomicUsize>);

    impl Action<u64> for Replicable {
        type Inputs = (u64,);
        type Output = ();

        fn concurrency(&self) -> Concurrency<Self> {
            Concurrency::adaptive()
        }

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context { inputs: input, output, .. } = context;
            input.for_each(output, |_, _| {
                self.0.fetch_add(1, Ordering::Relaxed);
                Ok(())
            });
        }
    }

    #[test]
    fn replicas_are_independent_movable_invocations() {
        let calls = Arc::new(AtomicUsize::new(0));
        let original =
            Job::<u64>::new::<Replicable>(Replicable(Arc::clone(&calls)));
        let replica = original.replica();
        let first = Invocation::new(
            RevisionId::test(0),
            0,
            original,
            vec![Some(input(1, 1))],
            false,
        );
        let second = Invocation::new(
            RevisionId::test(0),
            0,
            replica,
            vec![Some(input(2, 2))],
            false,
        );

        let first = std::thread::spawn(move || first.run());
        let second = std::thread::spawn(move || second.run());
        let _ = first.join().unwrap();
        let _ = second.join().unwrap();
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    struct RecordThread(Arc<Mutex<Vec<std::thread::ThreadId>>>);

    impl Action<u64> for RecordThread {
        type Inputs = (u64,);
        type Output = ();

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context { inputs: input, output, .. } = context;
            input.for_each(output, |_, emit| {
                self.0.lock().unwrap().push(std::thread::current().id());
                emit.mark("worker");
                Ok(())
            });
        }
    }

    #[derive(Debug)]
    struct Reject;

    impl Strategy for Reject {
        fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
            Err(ExecutorError::Submit(task))
        }

        fn num_workers(&self) -> usize {
            0
        }

        fn num_tasks_running(&self) -> usize {
            0
        }

        fn num_tasks_pending(&self) -> usize {
            0
        }

        fn capacity(&self) -> usize {
            0
        }
    }

    #[test]
    fn invocation_moves_to_worker_and_survives_rejection() {
        let caller = std::thread::current().id();
        let records = Arc::new(Mutex::new(Vec::new()));
        let execution = Invocation::new(
            RevisionId::test(0),
            0,
            Job::new(RecordThread(Arc::clone(&records))),
            vec![Some(input(1, 1))],
            false,
        );
        let (sender, receiver) = mpsc::sync_channel(1);
        let task = AssertUnwindSafe(move || {
            sender.send(execution.run()).unwrap();
        });
        let ExecutorError::Submit(task) = Executor::new(Reject)
            .submit(task)
            .expect_err("reject strategy accepted work")
        else {
            panic!("unexpected executor error");
        };
        Executor::new(WorkSharing::new(1)).submit(task).unwrap();
        let completion = receiver.recv().unwrap();
        assert_ne!(records.lock().unwrap()[0], caller);
        let [record] = completion.instrumentation.records() else {
            panic!("worker instrumentation was not returned")
        };
        assert!(
            matches!(record, Record::Annotation(marker) if marker.name() == "worker")
        );
    }

    struct Ignore;

    impl Action<u64> for Ignore {
        type Inputs = (u64,);
        type Output = ();

        fn execute(&mut self, _: Context<'_, u64, Self>) {}
    }

    #[test]
    fn unread_input_panics() {
        let execution = Invocation::new(
            RevisionId::test(0),
            3,
            Job::new(Ignore),
            vec![Some(input(1, 1))],
            false,
        );
        let result =
            std::panic::catch_unwind(AssertUnwindSafe(|| execution.run()));
        assert!(result.is_err());
    }

    struct Panic;

    impl Action<u64> for Panic {
        type Inputs = (u64,);
        type Output = ();

        fn execute(&mut self, _: Context<'_, u64, Self>) {
            panic!("action bug");
        }
    }

    #[test]
    fn action_panic_escapes_execution() {
        let execution = Invocation::new(
            RevisionId::test(0),
            0,
            Job::new(Panic),
            vec![Some(input(1, 1))],
            false,
        );
        let result =
            std::panic::catch_unwind(AssertUnwindSafe(|| execution.run()));
        assert!(result.is_err());
    }

    struct FailOne;

    impl Action<u64> for FailOne {
        type Inputs = (u64,);
        type Output = ();

        fn execute(&mut self, context: Context<'_, u64, Self>) {
            let Context { inputs: input, output, .. } = context;
            input.for_each(output, |change, _| match change {
                Change::Insert(_, value) if *value.as_ref() == 1 => {
                    Err(anyhow::anyhow!("user error").into())
                }
                _ => Ok(()),
            });
        }
    }

    #[test]
    fn explicit_item_error_is_sparse_and_does_not_stop_the_batch() {
        let segment = Segment::new(vec![
            Change::Insert(1_u64, 1_u64),
            Change::Insert(2_u64, 2_u64),
        ]);
        let completion = Invocation::new(
            RevisionId::test(0),
            0,
            Job::new(FailOne),
            vec![Some(segment)],
            false,
        )
        .run();

        assert_eq!(completion.outcomes.failures().len(), 1);
    }
}
