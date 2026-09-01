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

//! Differential conformance test support.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;

use zrx_scheduler::{RevisionId, Settlement};
use zrx_stream::{Change, Key, Run, Value};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terminal {
    Complete,
    Aborted,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stateful differential oracle for one externally published stream.
///
/// Every observed change is applied to the previously published state before
/// that state is compared with a directly computed reference snapshot. The
/// harness also proves that each admitted revision settles exactly once and
/// that a quiescent runner cannot replay retained publication.
pub struct Differential<T> {
    name: &'static str,
    state: BTreeMap<Key<u64>, T>,
    settled: HashSet<RevisionId>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Differential<T>
where
    T: Value + Debug + PartialEq,
{
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            state: BTreeMap::new(),
            settled: HashSet::new(),
        }
    }

    pub fn observe(
        &mut self, run: &mut Run<u64>, expected: &BTreeMap<Key<u64>, T>,
        terminals: &[Terminal],
    ) {
        self.observe_with_failures(run, expected, terminals, 0);
    }

    pub fn observe_with_failures(
        &mut self, run: &mut Run<u64>, expected: &BTreeMap<Key<u64>, T>,
        terminals: &[Terminal], expected_failures: usize,
    ) {
        let changes: Vec<_> = run.output::<T>().unwrap().collect();
        for change in &changes {
            match change {
                Change::Insert(key, value) => {
                    self.state.insert(key.clone(), value.clone());
                }
                Change::Remove(key) => {
                    // Stateless transforms may emit an idempotent retraction
                    // for a nonmatching replacement: they do not own enough
                    // state to know whether an earlier value was visible.
                    self.state.remove(key);
                }
            }
        }
        assert_eq!(
            &self.state, expected,
            "{} diverged after changes {changes:?}",
            self.name,
        );

        let failures = run
            .report()
            .invocations()
            .iter()
            .map(|invocation| invocation.outcomes.error_count())
            .sum::<usize>();
        assert_eq!(
            failures, expected_failures,
            "{} reported an unexpected number of failures",
            self.name,
        );

        let settlements = run.report().settlements();
        assert_eq!(
            settlements.len(),
            terminals.len(),
            "{} settled an unexpected number of revisions: {settlements:?}",
            self.name,
        );
        for (settlement, expected) in settlements.iter().zip(terminals) {
            let (revision, actual) = match settlement {
                Settlement::Complete(revision) => {
                    (*revision, Terminal::Complete)
                }
                Settlement::Aborted(revision) => (*revision, Terminal::Aborted),
            };
            assert_eq!(actual, *expected, "{} settlement kind", self.name);
            assert!(
                self.settled.insert(revision),
                "{} settled revision {revision} more than once",
                self.name,
            );
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

pub fn key(value: u64) -> Key<u64> {
    Key::from(value)
}

pub fn path(values: impl IntoIterator<Item = u64>) -> Key<u64> {
    values.into_iter().collect()
}
