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

//! Sparse historical errors returned by one action invocation.

use std::any::TypeId;

use super::Error;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

pub(crate) enum EvaluationChange<I> {
    Reject {
        evaluation: Evaluation<I>,
        error: Error,
    },
    Resolve(Evaluation<I>),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

pub(crate) struct Evaluation<I> {
    pub(crate) domain: TypeId,
    pub(crate) key: I,
    equals: fn(&I, &I) -> bool,
}

// ----------------------------------------------------------------------------

pub(crate) struct EvaluationChanges<I> {
    changes: Vec<EvaluationChange<I>>,
}

pub(crate) struct DefaultEvaluation;

// ----------------------------------------------------------------------------

/// Sparse historical errors; successful evaluations produce no entry.
#[derive(Debug, Default)]
pub struct Outcomes {
    failures: Vec<Error>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Evaluation<I> {
    fn new<D>(key: I) -> Self
    where
        D: 'static,
        I: Eq,
    {
        Self {
            domain: TypeId::of::<D>(),
            key,
            equals: PartialEq::eq,
        }
    }

    pub(crate) fn matches(&self, domain: TypeId, key: &I) -> bool {
        self.domain == domain && (self.equals)(&self.key, key)
    }
}

// ----------------------------------------------------------------------------

impl<I> EvaluationChanges<I> {
    pub(super) fn reject<D>(&mut self, key: I, error: Error)
    where
        D: 'static,
        I: Eq,
    {
        self.changes.push(EvaluationChange::Reject {
            evaluation: Evaluation::new::<D>(key),
            error,
        });
    }

    pub(super) fn resolve<D>(&mut self, key: I)
    where
        D: 'static,
        I: Eq,
    {
        self.changes
            .push(EvaluationChange::Resolve(Evaluation::new::<D>(key)));
    }
}

// ----------------------------------------------------------------------------

impl Outcomes {
    pub(super) fn report(&mut self, error: Error) {
        self.failures.push(error);
    }

    /// Returns errors observed during this invocation.
    #[must_use]
    pub fn failures(&self) -> &[Error] {
        &self.failures
    }

    /// Returns whether the invocation reported no errors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.failures.is_empty()
    }

    /// Returns the number of errors reported by this invocation.
    #[must_use]
    pub fn len(&self) -> usize {
        self.failures.len()
    }

    /// Returns the number of errors reported by this invocation.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.failures.len()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Default for EvaluationChanges<I> {
    fn default() -> Self {
        Self { changes: Vec::new() }
    }
}

impl<I> IntoIterator for EvaluationChanges<I> {
    type Item = EvaluationChange<I>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.changes.into_iter()
    }
}
