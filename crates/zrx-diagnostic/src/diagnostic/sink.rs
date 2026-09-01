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

//! Diagnostic sink.

use super::Diagnostic;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Diagnostic sink.
///
/// The sink accepts a fully constructed [`Diagnostic`] so it remains object
/// safe and can be borrowed as `&mut dyn Sink` by callback scopes and adapters.
/// Diagnostic macros construct values with call-site location information, so
/// we can pass those values explicitly to [`Sink::emit`] to record them.
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::error;
/// use zrx_diagnostic::sink::Sink;
///
/// // Create diagnostic sink
/// let mut sink = Vec::new();
///
/// // Create diagnostic with static string
/// sink.emit(error!("Static"));
///
/// // Create diagnostic with format string
/// sink.emit(error!("Format: {}", true));
/// ```
pub trait Sink {
    /// Emits the given diagnostic.
    fn emit(&mut self, diagnostic: Diagnostic);
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Sink for Vec<Diagnostic> {
    /// Emits the given diagnostic.
    #[inline]
    fn emit(&mut self, diagnostic: Diagnostic) {
        self.push(diagnostic);
    }
}
