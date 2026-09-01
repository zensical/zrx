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

//! Stream output.

use std::collections::{BTreeSet, VecDeque};
use std::marker::PhantomData;

use zrx_scheduler::action::Port;
use zrx_scheduler::plan::OutputId;
use zrx_scheduler::{Egress, EgressIter, Report, Value};

use crate::stream::Id;
use crate::stream::workflow::{Direction, LookupError, Output as OutputPort};
use crate::stream::{Change, Key};

use super::Error;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Settled consequences of one reusable workflow execution cycle.
#[must_use]
pub struct Run<I>
where
    I: Id,
{
    outputs: Vec<OutputPort>,
    egress: VecDeque<Egress<Key<I>>>,
    claimed: BTreeSet<OutputId>,
    report: Report,
}

// ----------------------------------------------------------------------------

/// Lazy owning iterator over one settled workflow output.
pub struct Output<I, T>
where
    I: Id,
{
    batches: VecDeque<EgressIter<Key<I>, T>>,
    current: Option<EgressIter<Key<I>, T>>,
    marker: PhantomData<fn() -> I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Run<I>
where
    I: Id,
{
    pub(in crate::stream::execution) fn new(
        outputs: Vec<OutputPort>, egress: VecDeque<Egress<Key<I>>>,
        report: Report,
    ) -> Self {
        Self {
            outputs,
            egress,
            claimed: BTreeSet::new(),
            report,
        }
    }

    /// Returns scheduler diagnostics and revision settlements.
    pub const fn report(&self) -> &Report {
        &self.report
    }

    /// Takes the sole output carrying `T` as a lazy change iterator.
    ///
    /// # Errors
    ///
    /// Returns an error when no unique matching output exists or it was
    /// already taken.
    pub fn output<T>(&mut self) -> Result<Output<I, T>, Error>
    where
        T: Value,
    {
        let port = Port::of::<Key<I>, T>();
        let mut outputs =
            self.outputs.iter().filter(|output| output.port() == port);
        let Some(output) = outputs.next().copied() else {
            return Err(LookupError::Missing {
                direction: Direction::Output,
                value: std::any::type_name::<T>(),
            }
            .into());
        };
        if outputs.next().is_some() {
            return Err(LookupError::Ambiguous {
                direction: Direction::Output,
                value: std::any::type_name::<T>(),
            }
            .into());
        }
        self.output_at(output)
    }

    /// Takes one exact erased output through its statically known type.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign or mismatched endpoint or if the output
    /// was already taken.
    pub fn output_at<T>(
        &mut self, output: OutputPort,
    ) -> Result<Output<I, T>, Error>
    where
        T: Value,
    {
        let expected = Port::of::<Key<I>, T>();
        if output.port() != expected || !self.outputs.contains(&output) {
            return Err(LookupError::Missing {
                direction: Direction::Output,
                value: std::any::type_name::<T>(),
            }
            .into());
        }
        if !self.claimed.insert(output.id()) {
            return Err(Error::Taken(output.id()));
        }

        let mut batches = VecDeque::new();
        let mut retained = VecDeque::with_capacity(self.egress.len());
        while let Some(batch) = self.egress.pop_front() {
            if batch.output() == output.id() {
                batches.push_back(batch.into_changes::<T>());
            } else {
                retained.push_back(batch);
            }
        }
        self.egress = retained;
        Ok(Output {
            batches,
            current: None,
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Iterator for Output<I, T>
where
    I: Id,
    T: Value,
{
    type Item = Change<I, T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(change) = self.current.as_mut().and_then(Iterator::next)
            {
                return Some(change);
            }
            self.current = self.batches.pop_front();
            self.current.as_ref()?;
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let current = self.current.as_ref().map_or(0, ExactSizeIterator::len);
        let remaining: usize =
            self.batches.iter().map(ExactSizeIterator::len).sum();
        let len = current + remaining;
        (len, Some(len))
    }
}

impl<I, T> ExactSizeIterator for Output<I, T>
where
    I: Id,
    T: Value,
{
}

impl<I, T> std::iter::FusedIterator for Output<I, T>
where
    I: Id,
    T: Value,
{
}
