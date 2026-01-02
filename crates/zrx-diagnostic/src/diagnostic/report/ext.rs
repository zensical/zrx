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

//! Report extensions.

use std::error::Error;

use crate::diagnostic::IntoDiagnostic;

use super::convert::IntoReport;
use super::Report;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Extension of [`Result`].
///
/// This trait provides methods to attach diagnostics to a result, converting
/// it to a [`Report`], which is the recommended method to return diagnostics.
pub trait ResultExt<T, E>
where
    E: Error,
{
    /// Adds diagnostics to the result.
    ///
    /// This method attaches the given diagnostics to the result, converting it
    /// into a [`Report`]. Anything that implements [`IntoDiagnostic`] can be
    /// provided, so the recommended way is to implement [`IntoDiagnostic`]
    /// for custom types, allowing for ergonomic conversions.
    ///
    /// # Errors
    ///
    /// Note that this method never returns an error by itself, as this trait is
    /// primarily provided to wrap values in- or outside of results in reports.
    /// Consequentially, this trait isn't named `TryResultExt`, as it's not
    /// falling under Rust's try-semantics.
    fn with_diagnostics<D>(self, diagnostics: D) -> Result<Report<T>, E>
    where
        D: IntoIterator,
        D::Item: IntoDiagnostic;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T, E> ResultExt<T, E> for Result<T, E>
where
    E: Error,
{
    /// Adds diagnostics to the result.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Error;
    /// use zrx_diagnostic::report::IntoReport;
    /// use zrx_diagnostic::{hint, ResultExt};
    ///
    /// // Define function returning a report
    /// fn f() -> impl IntoReport<i32, Error> {
    ///     Ok(42).with_diagnostics([
    ///         hint!("Insufficient data for meaningful answer"),
    ///     ])
    /// }
    ///
    /// // Invoke function and create report
    /// let res = f().into_report();
    /// ```
    #[inline]
    fn with_diagnostics<D>(self, diagnostics: D) -> Result<Report<T>, E>
    where
        D: IntoIterator,
        D::Item: IntoDiagnostic,
    {
        self.into_report().with_diagnostics(diagnostics)
    }
}

impl<T, E> ResultExt<T, E> for Result<Report<T>, E>
where
    E: Error,
{
    /// Adds more diagnostics to the result.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Error;
    /// use zrx_diagnostic::report::IntoReport;
    /// use zrx_diagnostic::{hint, ResultExt};
    ///
    /// // Define function returning a report
    /// fn f() -> impl IntoReport<i32, Error> {
    ///     Ok(42)
    ///         .with_diagnostics([
    ///             hint!("Insufficient data for meaningful answer"),
    ///         ])
    ///         .with_diagnostics([
    ///             hint!("Try to be more specific next time")
    ///         ])
    /// }
    ///
    /// // Invoke function and create report
    /// let res = f().into_report();
    /// ```
    #[inline]
    fn with_diagnostics<D>(self, diagnostics: D) -> Result<Report<T>, E>
    where
        D: IntoIterator,
        D::Item: IntoDiagnostic,
    {
        self.map(|mut report| {
            for diagnostic in diagnostics {
                report.add(diagnostic.into_diagnostic());
            }
            report
        })
    }
}
