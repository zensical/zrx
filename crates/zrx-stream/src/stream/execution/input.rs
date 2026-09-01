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

//! Stream input.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use zrx_scheduler::{Session, Value, Writer};

use crate::stream::{Change, Id, Key};

use super::Error;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Default)]
pub struct Progress {
    open: AtomicUsize,
    pending: AtomicUsize,
}

// ----------------------------------------------------------------------------

/// Admission accounting before a scheduler revision becomes observable.
///
/// Dropping this state cancels the tentative open count. Committing it after
/// [`Session::begin`] succeeds transfers that count to [`OpenProgress`].
struct OpeningProgress {
    progress: Option<Arc<Progress>>,
}

// ----------------------------------------------------------------------------

/// Accounting authority for exactly one open workflow revision.
///
/// Closing or dropping this token performs the single `open -> pending`
/// transition before the scheduler terminal event can become observable.
struct OpenProgress {
    progress: Option<Arc<Progress>>,
}

// ----------------------------------------------------------------------------

/// Typed transferable input capability for one workflow endpoint.
///
/// Opening a revision consumes this capability, so one endpoint has at most
/// one open revision. Sealing or aborting returns the capability while prior
/// physical work may still be draining.
///
/// ```compile_fail
/// use zrx_executor::strategy::Immediate;
/// use zrx_stream::Workflow;
///
/// # fn main() -> Result<(), zrx_stream::Error> {
/// let workflow = Workflow::<u64>::build(|workflow| {
///     let input = workflow.input::<u64>();
///     workflow.output(&input);
/// });
/// let mut runner = workflow.runner_with(Immediate::new())?;
/// let input = runner.input::<u64>()?;
/// let _open = input.begin()?;
/// let _second = input.begin()?; // the input capability was consumed
/// # Ok(())
/// # }
/// ```
#[must_use]
pub struct Input<I, T>
where
    I: Id,
{
    session: Session<Key<I>, T>,
    progress: Arc<Progress>,
}

// ----------------------------------------------------------------------------

/// Affine writer for one open workflow input revision.
///
/// The provider must submit at most one final net change per key. The stream
/// preserves submission order but does not consolidate a revision-wide key
/// map to enforce that contract.
///
/// ```compile_fail
/// use zrx_executor::strategy::Immediate;
/// use zrx_stream::{Key, Workflow};
///
/// # fn main() -> Result<(), zrx_stream::Error> {
/// let workflow = Workflow::<u64>::build(|workflow| {
///     let input = workflow.input::<u64>();
///     workflow.output(&input);
/// });
/// let mut runner = workflow.runner_with(Immediate::new())?;
/// let input = runner.input::<u64>()?;
/// let mut revision = input.begin()?;
/// let _input = revision.seal()?;
/// revision.insert(Key::from(1), 1)?; // sealing consumed the revision
/// # Ok(())
/// # }
/// ```
#[must_use = "an open workflow revision must be sealed or aborted"]
pub struct Revision<I, T>
where
    I: Id,
{
    writer: Option<Writer<Key<I>, T>>,
    progress: OpenProgress,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Progress {
    fn opening(progress: Arc<Self>) -> OpeningProgress {
        progress.open.fetch_add(1, Ordering::Release);
        OpeningProgress { progress: Some(progress) }
    }

    fn cancel_opening(&self) {
        let open = self.open.fetch_sub(1, Ordering::Release);
        assert_ne!(open, 0, "workflow revision accounting underflowed");
    }

    fn close(&self) {
        // Publish pending authority before releasing the open authority. A
        // runner that observes the release of the last open revision must
        // therefore also observe either this pending count or its settlement.
        self.pending.fetch_add(1, Ordering::Relaxed);
        let open = self.open.fetch_sub(1, Ordering::Release);
        assert_ne!(open, 0, "workflow revision accounting underflowed");
    }

    pub fn settled(&self, count: usize) {
        let pending = self.pending.fetch_sub(count, Ordering::AcqRel);
        assert!(
            pending >= count,
            "scheduler settled more revisions than the workflow submitted"
        );
    }

    pub fn open(&self) -> usize {
        self.open.load(Ordering::Acquire)
    }

    pub fn pending(&self) -> usize {
        self.pending.load(Ordering::Acquire)
    }
}

// ----------------------------------------------------------------------------

impl OpeningProgress {
    fn commit(mut self) -> OpenProgress {
        OpenProgress { progress: self.progress.take() }
    }
}

// ----------------------------------------------------------------------------

impl OpenProgress {
    fn close(&mut self) -> Arc<Progress> {
        let progress = self
            .progress
            .take()
            .expect("open revision owns progress authority");
        progress.close();
        progress
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Input<I, T>
where
    I: Id,
    T: Value,
{
    pub(in crate::stream::execution) fn new(
        session: Session<Key<I>, T>, progress: Arc<Progress>,
    ) -> Self {
        Self { session, progress }
    }

    /// Opens one affine input revision.
    ///
    /// # Errors
    ///
    /// Returns an error if the installed scheduler input disconnected or
    /// exhausted revision identities.
    pub fn begin(self) -> Result<Revision<I, T>, Error> {
        // Acquire workflow accounting before `Session::begin` publishes the
        // scheduler Begin event. `OpeningProgress` rolls this back if
        // admission fails before a revision exists.
        let progress = Progress::opening(self.progress);
        let writer = self.session.begin()?;
        Ok(Revision {
            writer: Some(writer),
            progress: progress.commit(),
        })
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Revision<I, T>
where
    I: Id,
    T: Value,
{
    /// Emits one keyed change.
    pub fn emit(&mut self, change: Change<I, T>) -> Result<(), Error> {
        self.writer().emit(change)?;
        Ok(())
    }

    /// Emits one keyed insertion or replacement.
    pub fn insert(&mut self, key: Key<I>, value: T) -> Result<(), Error> {
        self.writer().insert(key, value)?;
        Ok(())
    }

    /// Emits one keyed removal.
    pub fn remove(&mut self, key: Key<I>) -> Result<(), Error> {
        self.writer().remove(key)?;
        Ok(())
    }

    /// Emits at most one scheduler-native batch from an iterator.
    ///
    /// The iterator retains every unconsumed change. Interleave calls with
    /// [`super::Runner::advance`] so a revision larger than the bounded input
    /// channel can make progress without staging another collection.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch can no longer reach the installed
    /// workflow.
    pub fn emit_from<Changes>(
        &mut self, changes: &mut Changes,
    ) -> Result<usize, Error>
    where
        Changes: Iterator<Item = Change<I, T>>,
    {
        Ok(self.writer().emit_from(changes)?)
    }

    /// Seals this revision and returns its reusable input capability.
    pub fn seal(mut self) -> Result<Input<I, T>, Error> {
        let writer = self.writer.take().expect("open revision owns its writer");
        // Transfer workflow accounting before the scheduler End event can be
        // observed and settled by a concurrent runner.
        let progress = self.progress.close();
        let session = writer.seal();
        Ok(Input::new(session?, progress))
    }

    /// Aborts this revision and returns its reusable input capability.
    ///
    /// Locally buffered changes are discarded and undispatched scheduler work
    /// is fenced. Already dispatched operators are not rolled back, so their
    /// retained state can affect a later valid revision even when this abort
    /// itself publishes no output.
    pub fn abort(mut self) -> Result<Input<I, T>, Error> {
        let writer = self.writer.take().expect("open revision owns its writer");
        // Transfer workflow accounting before the scheduler Abort event can
        // be observed and settled by a concurrent runner.
        let progress = self.progress.close();
        let session = writer.abort();
        Ok(Input::new(session?, progress))
    }

    fn writer(&mut self) -> &mut Writer<Key<I>, T> {
        self.writer.as_mut().expect("open revision owns its writer")
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Drop for OpeningProgress {
    fn drop(&mut self) {
        if let Some(progress) = self.progress.take() {
            progress.cancel_opening();
        }
    }
}

// ----------------------------------------------------------------------------

impl Drop for OpenProgress {
    fn drop(&mut self) {
        if let Some(progress) = self.progress.take() {
            progress.close();
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Drop for Revision<I, T>
where
    I: Id,
{
    fn drop(&mut self) {
        if let Some(writer) = self.writer.take() {
            // Close workflow accounting before dropping the writer publishes
            // its implicit scheduler abort.
            drop(self.progress.close());
            drop(writer);
        }
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::Progress;

    #[test]
    fn failed_opening_rolls_back_without_creating_pending_work() {
        let progress = Arc::new(Progress::default());
        let opening = Progress::opening(Arc::clone(&progress));
        assert_eq!(progress.open(), 1);
        assert_eq!(progress.pending(), 0);

        drop(opening);
        assert_eq!(progress.open(), 0);
        assert_eq!(progress.pending(), 0);
    }

    #[test]
    fn committed_opening_closes_into_exactly_one_pending_revision() {
        let progress = Arc::new(Progress::default());
        let mut open = Progress::opening(Arc::clone(&progress)).commit();
        assert_eq!(progress.open(), 1);
        assert_eq!(progress.pending(), 0);

        drop(open.close());
        assert_eq!(progress.open(), 0);
        assert_eq!(progress.pending(), 1);

        progress.settled(1);
        assert_eq!(progress.pending(), 0);
        drop(open);
        assert_eq!(progress.pending(), 0);
    }

    #[test]
    fn dropping_open_progress_performs_the_close_transition_once() {
        let progress = Arc::new(Progress::default());
        let open = Progress::opening(Arc::clone(&progress)).commit();

        drop(open);
        assert_eq!(progress.open(), 0);
        assert_eq!(progress.pending(), 1);
    }
}
