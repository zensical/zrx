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

//! Action diagnostics and author annotations.

use std::borrow::Cow;
use std::time::Duration;

use zrx_diagnostic::Diagnostic;
use zrx_diagnostic::sink::Sink;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Destination for action-authored records.
pub trait Recorder: Sink {
    /// Emits a named zero-duration annotation.
    fn mark(&mut self, name: Cow<'static, str>);

    /// Emits an explicitly measured operation.
    fn measure(&mut self, name: Cow<'static, str>, elapsed: Duration);
}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// One structured diagnostic or named author annotation.
#[derive(Debug)]
pub enum Record {
    /// Structured workspace diagnostic.
    Diagnostic(Diagnostic),
    /// Named zero-duration annotation.
    Annotation(Annotation),
    /// Explicitly measured author operation.
    Measurement(Measurement),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Ordered action-authored records from one invocation.
#[derive(Debug, Default)]
pub struct Instrumentation {
    records: Vec<Record>,
}

// ----------------------------------------------------------------------------

/// Named duration measured explicitly by action code.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Measurement {
    name: Cow<'static, str>,
    elapsed: Duration,
}

// ----------------------------------------------------------------------------

/// Named zero-duration annotation without diagnostic severity semantics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    name: Cow<'static, str>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Instrumentation {
    /// Returns records in author emission order.
    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// Returns whether no action record was emitted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub(super) fn push(&mut self, record: Record) {
        self.records.push(record);
    }
}

// ----------------------------------------------------------------------------

impl Measurement {
    pub(super) fn new(
        name: impl Into<Cow<'static, str>>, elapsed: Duration,
    ) -> Self {
        Self { name: name.into(), elapsed }
    }

    /// Returns the measured operation name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the elapsed monotonic duration.
    #[must_use]
    pub const fn elapsed(&self) -> Duration {
        self.elapsed
    }
}

// ----------------------------------------------------------------------------

impl Annotation {
    /// Creates one named annotation.
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the annotation name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
