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

//! Value extensions.

use zrx_diagnostic::report::Report;
use zrx_diagnostic::IntoDiagnostic;

use super::Value;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Extension of [`Value`].
///
/// This trait provides methods to attach diagnostics to a value, converting
/// it to a [`Report`], which allows for a more ergonomic API.
pub trait ValueExt: Sized {
    /// Adds diagnostics to the value.
    ///
    /// This method attaches the given diagnostics to the value, converting it
    /// into a [`Report`]. Anything that implements [`IntoDiagnostic`] can be
    /// provided, so the recommended way is to implement [`IntoDiagnostic`]
    /// for custom types, allowing for ergonomic conversions.
    ///
    /// # Errors
    ///
    /// Note that this method never returns an error by itself, as this trait is
    /// primarily provided to wrap values in- or outside of results in reports.
    /// Consequentially, this trait isn't named `TryValueExt`, as it's not
    /// falling under Rust's try-semantics.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::hint;
    /// use zrx_scheduler::ValueExt;
    ///
    /// // Create report
    /// let report = 42.with_diagnostics([
    ///     hint!("Insufficient data for meaningful answer"),
    /// ]);
    /// ```
    #[inline]
    fn with_diagnostics<D>(self, diagnostics: D) -> Report<Self>
    where
        D: IntoIterator,
        D::Item: IntoDiagnostic,
    {
        Report::new(self).with(diagnostics)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> ValueExt for T where T: Value {}
