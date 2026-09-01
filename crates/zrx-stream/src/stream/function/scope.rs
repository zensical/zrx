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

//! Function scope.

use std::borrow::Cow;
use std::time::Instant;

use zrx_diagnostic::sink::Sink;
use zrx_scheduler::action::Recorder;

use crate::stream::Key;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Function scope.
pub struct Scope<'a, I> {
    key: &'a Key<I>,
    records: &'a mut dyn Recorder,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, I> Scope<'a, I> {
    pub(crate) fn new(key: &'a Key<I>, records: &'a mut dyn Recorder) -> Self {
        Self { key, records }
    }

    /// Returns the key of the current stream item.
    #[must_use]
    pub const fn key(&self) -> &Key<I> {
        self.key
    }

    /// Emits a named zero-duration annotation.
    pub fn mark(&mut self, name: impl Into<Cow<'static, str>>) {
        self.records.mark(name.into());
    }

    /// Measures one explicitly named operation.
    pub fn measure<R>(
        &mut self, name: impl Into<Cow<'static, str>>,
        callback: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let start = Instant::now();
        let result = callback(self);
        self.records.measure(name.into(), start.elapsed());
        result
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Sink for Scope<'_, I> {
    fn emit(&mut self, diagnostic: zrx_diagnostic::Diagnostic) {
        self.records.emit(diagnostic);
    }
}
