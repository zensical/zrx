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

//! Pull-driven one-shot workflow construction and execution.

use std::collections::VecDeque;
use std::iter::FusedIterator;

use zrx_scheduler::{CurrentError, EgressIter, Report, Value};

use crate::stream::Id;
use crate::stream::workflow::{Builder, Input as InputPort};
use crate::stream::{Change, Key, Stream};

use super::{Advance, Error, Revision, Runner};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

trait Pump<I>
where
    I: Id,
{
    fn pump(&mut self, runner: &mut Runner<I>) -> Result<bool, Error>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Feed<'a, I>
where
    I: Id,
{
    pump: Box<dyn Pump<I> + 'a>,
}

// ----------------------------------------------------------------------------

struct Changes<I, T, Iter>
where
    I: Id,
    T: Value,
    Iter: Iterator<Item = Change<I, T>>,
{
    input: InputPort,
    iter: Iter,
    revision: Option<Revision<I, T>>,
    started: bool,
}

// ----------------------------------------------------------------------------

/// Scoped owner of one pull-driven workflow construction.
pub struct Scope<'a, I>
where
    I: Id,
{
    builder: Builder<I>,
    feeds: VecDeque<Feed<'a, I>>,
}

// ----------------------------------------------------------------------------

/// Pull-driven execution of one isolated workflow with one typed output.
///
/// Each call to [`Iterator::next`] admits bounded input, drives scheduler work,
/// and stops as soon as one output item becomes available. Dropping this value
/// stops further admission and discards the isolated execution owner.
pub struct Execution<'a, I, T>
where
    I: Id,
    T: Value,
{
    // Feeds precede the runner so dropping an unfinished execution aborts open
    // revisions before dropping their scheduler.
    feeds: VecDeque<Feed<'a, I>>,
    runner: Runner<I>,
    output: crate::stream::workflow::Output,
    current: Option<EgressIter<Key<I>, T>>,
    report: Report,
    pump: bool,
    terminal: bool,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, I> Feed<'a, I>
where
    I: Id,
{
    fn new(pump: impl Pump<I> + 'a) -> Self {
        Self { pump: Box::new(pump) }
    }

    fn pump(&mut self, runner: &mut Runner<I>) -> Result<bool, Error> {
        self.pump.pump(runner)
    }
}

// ----------------------------------------------------------------------------

impl<'a, I> Scope<'a, I>
where
    I: Id,
{
    fn new() -> Self {
        Self {
            builder: Builder::new(),
            feeds: VecDeque::new(),
        }
    }

    /// Creates an input stream from keyed values supplied by an iterator.
    ///
    /// The iterator is consumed one scheduler-native batch at a time as the
    /// returned execution iterator is consumed.
    #[allow(clippy::iter_not_returning_iterator)]
    pub fn iter<K, T>(
        &mut self, values: impl IntoIterator<Item = (K, T)> + 'a,
    ) -> Stream<I, T>
    where
        K: Into<Key<I>> + 'a,
        T: Value,
    {
        let changes = values
            .into_iter()
            .map(|(key, value)| Change::Insert(key.into(), value));
        self.feed(changes)
    }

    /// Creates an input stream from explicit keyed changes.
    ///
    /// The iterator is consumed one scheduler-native batch at a time as the
    /// returned execution iterator is consumed.
    pub fn changes<T>(
        &mut self, changes: impl IntoIterator<Item = Change<I, T>> + 'a,
    ) -> Stream<I, T>
    where
        T: Value,
    {
        self.feed(changes.into_iter())
    }

    fn feed<T, Iter>(&mut self, changes: Iter) -> Stream<I, T>
    where
        T: Value,
        Iter: Iterator<Item = Change<I, T>> + 'a,
    {
        let (input, stream) = self.builder.input_endpoint::<T>();
        self.feeds.push_back(Feed::new(Changes {
            input,
            iter: changes,
            revision: None,
            started: false,
        }));
        stream
    }

    fn finish<T>(
        mut self, output: &Stream<I, T>,
    ) -> (
        crate::Workflow<I>,
        VecDeque<Feed<'a, I>>,
        crate::stream::workflow::Output,
    )
    where
        T: Value,
    {
        let output = self.builder.output_endpoint(output);
        (self.builder.finish(), self.feeds, output)
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Execution<'_, I, T>
where
    I: Id,
    T: Value,
{
    /// Returns all scheduler consequences observed so far.
    pub const fn report(&self) -> &Report {
        &self.report
    }

    /// Returns the current errors owned by this workflow execution.
    #[must_use]
    pub fn errors(&self) -> &[CurrentError<Key<I>>] {
        self.runner.errors()
    }

    /// Consumes the remaining execution and returns its complete report.
    ///
    /// # Errors
    ///
    /// Returns the first input, scheduler, or execution error encountered.
    pub fn finish(mut self) -> Result<Report, Error> {
        for change in self.by_ref() {
            change?;
        }
        Ok(self.report)
    }

    fn next_change(&mut self) -> Result<Option<Change<I, T>>, Error> {
        loop {
            if let Some(change) = self.current.as_mut().and_then(Iterator::next)
            {
                return Ok(Some(change));
            }
            self.current = None;

            if self.pump {
                if let Some(mut feed) = self.feeds.pop_front()
                    && !feed.pump(&mut self.runner)?
                {
                    self.feeds.push_back(feed);
                }
                self.pump = false;
            }

            match self.runner.advance()? {
                Advance::Output(batch) => {
                    assert_eq!(
                        batch.output(),
                        self.output.id(),
                        "single-output execution received another output"
                    );
                    self.current = Some(batch.into_changes::<T>());
                }
                Advance::Progress(report) => self.report.append(report),
                Advance::Settled => {
                    assert!(
                        self.feeds.is_empty(),
                        "execution settled before its inputs completed"
                    );
                    return Ok(None);
                }
                Advance::Idle if !self.feeds.is_empty() => self.pump = true,
                Advance::Idle => return Err(Error::Stalled),
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, Iter> Pump<I> for Changes<I, T, Iter>
where
    I: Id,
    T: Value,
    Iter: Iterator<Item = Change<I, T>>,
{
    fn pump(&mut self, runner: &mut Runner<I>) -> Result<bool, Error> {
        if !self.started {
            let input = runner.input_at::<T>(self.input)?;
            self.revision = Some(input.begin()?);
            self.started = true;
            return Ok(false);
        }

        let revision = self
            .revision
            .as_mut()
            .expect("started feed owns its revision");
        if revision.emit_from(&mut self.iter)? != 0 {
            return Ok(false);
        }

        let revision = self
            .revision
            .take()
            .expect("started feed owns its revision");
        let _input = revision.seal()?;
        Ok(true)
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Iterator for Execution<'_, I, T>
where
    I: Id,
    T: Value,
{
    type Item = Result<Change<I, T>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.terminal {
            return None;
        }
        match self.next_change() {
            Ok(Some(change)) => Some(Ok(change)),
            Ok(None) => {
                self.terminal = true;
                None
            }
            Err(error) => {
                self.terminal = true;
                Some(Err(error))
            }
        }
    }
}

impl<I, T> FusedIterator for Execution<'_, I, T>
where
    I: Id,
    T: Value,
{
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Builds one isolated workflow with one output and returns lazy execution.
///
/// No input iterator is consumed and no action runs before the returned
/// iterator is consumed.
///
/// # Errors
///
/// Returns an error if the constructed workflow cannot be lowered.
///
/// # Examples
///
/// ```no_run
/// use zrx_stream::function::with_value;
/// use zrx_stream::{run, Change};
///
/// # fn main() -> Result<(), zrx_stream::Error> {
/// let values = [2_u64, 3, 4];
/// let mut output = run::<u64, _>(|scope| {
///     let input = scope.iter(
///         values.iter().enumerate().map(|(key, value)| {
///             (u64::try_from(key).unwrap(), *value)
///         }),
///     );
///     input.map(with_value(|value: &u64| *value * 2))
/// })?;
///
/// let changes: Result<Vec<Change<u64, u64>>, _> = output.by_ref().collect();
/// changes?;
/// # Ok(())
/// # }
/// ```
pub fn run<'a, I, T>(
    build: impl FnOnce(&mut Scope<'a, I>) -> Stream<I, T>,
) -> Result<Execution<'a, I, T>, Error>
where
    I: Id,
    T: Value,
{
    let mut scope = Scope::new();
    let output = build(&mut scope);
    let (workflow, feeds, output) = scope.finish(&output);
    let runner = workflow.runner()?;
    Ok(Execution {
        feeds,
        runner,
        output,
        current: None,
        report: Report::default(),
        pump: true,
        terminal: false,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use zrx_scheduler::Settlement;

    use crate::stream::StreamTupleExt;
    use crate::stream::{Change, Key};

    use super::run;

    #[test]
    fn input_and_execution_are_lazy_until_output_is_consumed() {
        let consumed = Cell::new(0);
        let mut output = run::<u64, _>(|scope| {
            let input = scope.iter((0_u64..10_000).map(|key| {
                consumed.set(consumed.get() + 1);
                (key, u32::try_from(key).unwrap())
            }));
            input.map(|value: &u32| *value * 2)
        })
        .unwrap();

        assert_eq!(consumed.get(), 0);
        assert!(matches!(
            output.next(),
            Some(Ok(Change::Insert(key, 0))) if key == Key::from(0_u64)
        ));
        assert!(consumed.get() < 10_000);
    }

    #[test]
    fn borrowed_iterators_flow_through_owned_outputs() {
        let values = [2_u32, 3, 4];
        let output = run::<u64, _>(|scope| {
            let values =
                scope.iter(values.iter().enumerate().map(|(index, value)| {
                    (u64::try_from(index).unwrap(), *value)
                }));
            values.map(|value: &u32| *value * 2)
        })
        .unwrap();

        let output: Result<Vec<_>, _> = output.collect();
        let output = output.unwrap();
        assert!(matches!(
            output.as_slice(),
            [
                Change::Insert(first, 4),
                Change::Insert(second, 6),
                Change::Insert(third, 8),
            ] if first == &Key::from(0_u64)
                && second == &Key::from(1_u64)
                && third == &Key::from(2_u64)
        ));
    }

    #[test]
    fn explicit_changes_preserve_retractions() {
        let output = run::<u64, _>(|scope| {
            scope.changes([
                Change::Insert(Key::from(1_u64), String::from("one")),
                Change::Remove(Key::from(2_u64)),
            ])
        })
        .unwrap();

        let output: Result<Vec<_>, _> = output.collect();
        let output = output.unwrap();
        assert!(matches!(
            output.as_slice(),
            [Change::Insert(first, value), Change::Remove(second)]
                if first == &Key::from(1_u64)
                    && value == "one"
                    && second == &Key::from(2_u64)
        ));
    }

    #[test]
    fn independent_inputs_join_into_one_lazy_output() {
        let mut output = run::<u64, _>(|scope| {
            let left = scope.iter([(1_u64, String::from("one"))]);
            let right = scope.iter([(1_u64, 10_u64), (2_u64, 20)]);
            (left, right).join()
        })
        .unwrap();

        let changes: Result<Vec<_>, _> = output.by_ref().collect();
        let changes = changes.unwrap();
        assert!(
            matches!(
                changes.as_slice(),
                [Change::Insert(key, (left, 10))]
                    if key == &Key::from(1_u64) && left == "one"
            ),
            "unexpected changes: {changes:?}"
        );
        assert_eq!(output.report().settlements().len(), 2);
    }

    #[test]
    fn input_larger_than_the_session_channel_remains_bounded() {
        let mut output = run::<u64, _>(|scope| {
            scope.iter((0_u64..70_000).map(|key| (key, key)))
        })
        .unwrap();

        assert_eq!(
            output
                .by_ref()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .len(),
            70_000
        );
        assert!(matches!(
            output.report().settlements(),
            [Settlement::Complete(_)]
        ));
    }
}
