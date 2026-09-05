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

//! Physical placement and return transport for opaque scheduler work.

use crossbeam::channel::{self, Receiver, Sender, TryRecvError};
use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::time::{Duration, Instant};

use zrx_executor::strategy::Immediate;
use zrx_executor::task::Task;
use zrx_executor::{Error, Executor, Strategy};

// Fallback readiness when executor capacity can return without a completion
// on this runtime's port. Ordinary scheduler ticks may retry sooner.
const RETRY_DELAY: Duration = Duration::from_millis(1);

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// One worker result transported back to the scheduler thread.
pub enum Return<T> {
    /// The opaque scheduler work completed normally.
    Completed(T),
    /// The opaque scheduler work panicked.
    Panicked(Box<dyn Any + Send>),
}

// ----------------------------------------------------------------------------

/// Physical result of one accepted placement submission.
pub enum Submission<T> {
    /// Work ran synchronously and returned directly to the scheduler.
    Inline(Return<T>),
    /// Work was transferred to the executor and will return through the port.
    Worker,
}

// ----------------------------------------------------------------------------

/// Physical execution resource consumed when installing one runtime.
pub enum Backend<S>
where
    S: Strategy,
{
    /// Execute directly on the scheduler thread.
    Inline,
    /// Execute through the supplied strategy.
    Worker(Executor<S>),
}

// ----------------------------------------------------------------------------

enum Kind<T, S>
where
    T: Send + 'static,
    S: Strategy,
{
    /// Execute directly on the scheduler thread.
    Inline,
    /// Execute through a shared worker strategy and private return port.
    Worker(Worker<T, S>),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Physical execution location for one runtime.
pub struct Placement<T, S>
where
    T: Send + 'static,
    S: Strategy,
{
    kind: Kind<T, S>,
}

// ----------------------------------------------------------------------------

/// Exclusive worker submission and completion port for one runtime.
struct Worker<T, S>
where
    T: Send + 'static,
    S: Strategy,
{
    executor: Executor<S>,
    sender: Sender<Return<T>>,
    receiver: Receiver<Return<T>>,
    overflow: Option<Retained>,
    outstanding: usize,
    limit: usize,
}

// ----------------------------------------------------------------------------

/// One rejected task and the deadline that guarantees its next retry.
struct Retained {
    task: Box<dyn Task>,
    deadline: Instant,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<S> Backend<S>
where
    S: Strategy,
{
    pub fn worker(strategy: S) -> Self {
        Self::Worker(Executor::new(strategy))
    }

    pub fn workers(&self) -> usize {
        match self {
            Self::Inline => 1,
            Self::Worker(executor) => executor.num_workers(),
        }
    }

    pub fn capacity(&self) -> usize {
        match self {
            Self::Inline => 0,
            Self::Worker(executor) => executor.capacity(),
        }
    }
}

impl Backend<Immediate> {
    pub const fn inline() -> Self {
        Self::Inline
    }
}

// ----------------------------------------------------------------------------

impl<T, S> Placement<T, S>
where
    T: Send + 'static,
    S: Strategy,
{
    /// Consumes a physical backend into resident placement state.
    pub fn new(backend: Backend<S>) -> Self {
        let limit = backend
            .workers()
            .checked_add(backend.capacity())
            .expect("executor window exceeds addressable memory")
            .max(1);
        Self::with_limit(backend, limit)
    }

    fn with_limit(backend: Backend<S>, limit: usize) -> Self {
        assert!(limit != 0, "placement limit must be non-zero");
        match backend {
            Backend::Inline => Self { kind: Kind::Inline },
            Backend::Worker(executor) => {
                let (sender, receiver) = channel::unbounded();
                Self {
                    kind: Kind::Worker(Worker {
                        executor,
                        sender,
                        receiver,
                        overflow: None,
                        outstanding: 0,
                        limit,
                    }),
                }
            }
        }
    }

    /// Returns whether another scheduler package may be submitted.
    #[must_use]
    pub fn accepts(&self) -> bool {
        match &self.kind {
            Kind::Inline => true,
            Kind::Worker(worker) => worker.accepts(),
        }
    }

    /// Submits opaque work, retaining a rejected task intact for retry.
    pub fn submit<F>(&mut self, work: F) -> Submission<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        match &mut self.kind {
            Kind::Inline => Submission::Inline(Return::Completed(work())),
            Kind::Worker(worker) => {
                worker.submit(work);
                Submission::Worker
            }
        }
    }

    /// Retries the one bounded rejected-submission queue.
    #[must_use]
    pub fn retry(&mut self) -> bool {
        match &mut self.kind {
            Kind::Inline => false,
            Kind::Worker(worker) => worker.retry(),
        }
    }

    /// Returns the fallback deadline owned by a retained submission.
    pub fn deadline(&self) -> Option<Instant> {
        match &self.kind {
            Kind::Inline => None,
            Kind::Worker(worker) => {
                worker.overflow.as_ref().map(|retained| retained.deadline)
            }
        }
    }

    /// Imports one return without blocking.
    pub fn try_recv(&mut self) -> Option<Return<T>> {
        match &mut self.kind {
            Kind::Inline => None,
            Kind::Worker(worker) => worker.try_recv(),
        }
    }

    /// Borrows the selectable worker-completion source, when one exists.
    pub const fn receiver(&self) -> Option<&Receiver<Return<T>>> {
        match &self.kind {
            Kind::Inline => None,
            Kind::Worker(worker) => Some(&worker.receiver),
        }
    }

    /// Returns whether no submitted or retained package remains.
    #[must_use]
    pub const fn is_idle(&self) -> bool {
        match &self.kind {
            Kind::Inline => true,
            Kind::Worker(worker) => worker.outstanding == 0,
        }
    }
}

// ----------------------------------------------------------------------------

impl<T, S> Worker<T, S>
where
    T: Send + 'static,
    S: Strategy,
{
    fn accepts(&self) -> bool {
        self.overflow.is_none() && self.outstanding < self.limit
    }

    fn submit<F>(&mut self, work: F)
    where
        F: FnOnce() -> T + Send + 'static,
    {
        assert!(self.accepts(), "placement overflow must be retried first");
        let sender = self.sender.clone();
        let task = AssertUnwindSafe(move || {
            let returned = match panic::catch_unwind(AssertUnwindSafe(work)) {
                Ok(value) => Return::Completed(value),
                Err(payload) => Return::Panicked(payload),
            };
            sender
                .send(returned)
                .expect("runtime completion port remains connected");
        });
        self.outstanding = self
            .outstanding
            .checked_add(1)
            .expect("placement count overflowed");
        if let Err(error) = self.executor.submit(task) {
            self.retain(error);
        }
    }

    fn retry(&mut self) -> bool {
        let Some(Retained { task, .. }) = self.overflow.take() else {
            return false;
        };
        match self.executor.submit(task) {
            Ok(()) => true,
            Err(error) => {
                self.retain(error);
                false
            }
        }
    }

    fn try_recv(&mut self) -> Option<Return<T>> {
        match self.receiver.try_recv() {
            Ok(returned) => {
                self.complete();
                Some(returned)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                panic!("runtime completion port disconnected")
            }
        }
    }

    fn retain(&mut self, error: Error) {
        match error {
            Error::Submit(task) => {
                #[cfg(feature = "tracing")]
                tracing::event!(
                    name: "executor.retained",
                    tracing::Level::TRACE,
                    outstanding = self.outstanding,
                    limit = self.limit,
                );
                assert!(
                    self.overflow
                        .replace(Retained {
                            task,
                            deadline: Instant::now() + RETRY_DELAY,
                        })
                        .is_none(),
                    "placement retained more than one rejected task"
                );
            }
            Error::Signal => panic!("executor signal poisoned"),
        }
    }

    fn complete(&mut self) {
        self.outstanding = self
            .outstanding
            .checked_sub(1)
            .expect("placement returned untracked work");
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<S> Clone for Backend<S>
where
    S: Strategy,
{
    fn clone(&self) -> Self {
        match self {
            Self::Inline => Self::Inline,
            Self::Worker(executor) => Self::Worker(executor.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;

    use crossbeam::channel::Select;
    use zrx_executor::strategy::{Immediate, WorkSharing};
    use zrx_executor::task::Task;
    use zrx_executor::{Error, Strategy};

    use super::{Backend, Placement, Return, Submission};

    fn inline<T>() -> Placement<T, Immediate>
    where
        T: Send + 'static,
    {
        Placement::new(Backend::inline())
    }

    fn worker<T, S>(strategy: S, limit: usize) -> Placement<T, S>
    where
        T: Send + 'static,
        S: Strategy,
    {
        Placement::with_limit(Backend::worker(strategy), limit)
    }

    fn recv<T, S>(placement: &mut Placement<T, S>) -> Return<T>
    where
        T: Send + 'static,
        S: Strategy,
    {
        {
            let mut select = Select::new();
            select.recv(
                placement
                    .receiver()
                    .expect("worker placement has a completion port"),
            );
            select.ready();
        }
        placement
            .try_recv()
            .expect("selected completion remains available")
    }

    #[derive(Debug, Default)]
    struct RejectOnce {
        rejected: AtomicBool,
    }

    impl Strategy for RejectOnce {
        fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
            if !self.rejected.swap(true, Ordering::Relaxed) {
                return Err(Error::Submit(task));
            }
            drop(task.execute());
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

    #[derive(Debug)]
    struct ObservedSubmit(Arc<AtomicUsize>);

    impl Strategy for ObservedSubmit {
        fn submit(&self, task: Box<dyn Task>) -> zrx_executor::Result {
            self.0.fetch_add(1, Ordering::Relaxed);
            drop(task.execute());
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

    #[test]
    fn explicit_inline_submission_returns_without_a_completion_port() {
        let caller = std::thread::current().id();
        let mut placement = inline();

        let Submission::Inline(Return::Completed(worker)) =
            placement.submit(|| std::thread::current().id())
        else {
            panic!("inline work entered the worker transport");
        };
        assert_eq!(worker, caller);
        assert!(placement.receiver().is_none());
        assert!(placement.try_recv().is_none());
        assert!(placement.is_idle());
    }

    #[test]
    fn custom_immediate_submission_is_not_bypassed() {
        let submissions = Arc::new(AtomicUsize::new(0));
        let mut placement = Placement::new(Backend::worker(ObservedSubmit(
            Arc::clone(&submissions),
        )));

        assert!(matches!(placement.submit(|| 1), Submission::Worker));
        assert!(matches!(recv(&mut placement), Return::Completed(1)));
        assert_eq!(submissions.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn worker_return_crosses_the_completion_port() {
        let caller = std::thread::current().id();
        let mut placement = worker(WorkSharing::new(1), 1);
        assert!(matches!(
            placement.submit(|| std::thread::current().id()),
            Submission::Worker
        ));

        let Return::Completed(worker) = recv(&mut placement) else {
            panic!("worker panicked");
        };
        assert_ne!(worker, caller);
        assert!(placement.is_idle());
    }

    #[test]
    fn logical_window_includes_undrained_returns() {
        let mut placement = worker(WorkSharing::new(1), 1);
        assert!(placement.accepts());
        assert!(matches!(placement.submit(|| 1), Submission::Worker));
        assert!(!placement.accepts());

        let Return::Completed(value) = recv(&mut placement) else {
            panic!("worker panicked");
        };
        assert_eq!(value, 1);
        assert!(placement.accepts());
    }

    #[test]
    fn one_rejected_task_blocks_submission_until_retry() {
        let mut placement = worker(RejectOnce::default(), 2);

        assert!(placement.deadline().is_none());
        assert!(matches!(placement.submit(|| 1), Submission::Worker));
        let deadline = placement
            .deadline()
            .expect("rejection owns retry readiness");
        assert_eq!(placement.deadline(), Some(deadline));
        assert!(!placement.accepts());
        assert!(placement.retry());
        assert!(placement.deadline().is_none());
        assert!(placement.accepts());

        let Return::Completed(value) = recv(&mut placement) else {
            panic!("retried worker panicked");
        };
        assert_eq!(value, 1);
        assert!(placement.is_idle());
    }

    #[test]
    fn worker_panic_returns_the_original_payload() {
        let mut placement = worker(WorkSharing::new(1), 1);
        assert!(matches!(
            placement.submit(|| -> () { panic!("action bug") }),
            Submission::Worker
        ));

        let Return::Panicked(payload) = recv(&mut placement) else {
            panic!("worker panic was lost");
        };
        assert_eq!(payload.downcast_ref::<&str>(), Some(&"action bug"));
        assert!(placement.is_idle());
    }

    #[test]
    fn independent_returns_may_arrive_out_of_order() {
        let (release, wait) = mpsc::channel();
        let mut placement = worker(WorkSharing::new(2), 2);
        assert!(matches!(
            placement.submit(move || {
                wait.recv().unwrap();
                1
            }),
            Submission::Worker
        ));
        assert!(matches!(placement.submit(|| 2), Submission::Worker));

        let Return::Completed(first) = recv(&mut placement) else {
            panic!("worker panicked");
        };
        assert_eq!(first, 2);
        release.send(()).unwrap();
        let Return::Completed(second) = recv(&mut placement) else {
            panic!("worker panicked");
        };
        assert_eq!(second, 1);
    }
}
