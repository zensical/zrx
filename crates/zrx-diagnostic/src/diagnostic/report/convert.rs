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

//! Report conversions.

use std::error::Error;
use std::result::Result;

use super::Report;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Report`]
///
/// This trait is generic over the [`Ok`] and [`Err`] variants of any result,
/// which means downstream consumers can implement this trait for custom types,
/// allowing them to convert any type into a report. It's primarily intended to
/// support adding diagnostics to fallible as well as infallible methods.
///
/// Note that the trait implementations provided here may not include sufficient
/// trait bounds, which makes explicit annotations necessary in some cases. For
/// this reason, downstream crates are recommended to provide their own custom
/// implementation of this trait, adding additional trait bounds on `T`.
pub trait IntoReport<T, E>
where
    E: Error,
{
    /// Converts into a report.
    ///
    /// # Errors
    ///
    /// Note that this method never returns an error by itself, as this trait is
    /// primarily provided for wrapping values returned from infallible methods.
    /// Consequentially, this trait isn't named `TryIntoReport`, as it's not
    /// falling under Rust's try-semantics.
    fn into_report(self) -> Result<Report<T>, E>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T, E> IntoReport<T, E> for T
where
    E: Error,
{
    /// Creates a report from a value `T` and wraps it in a result.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Error;
    /// use zrx_diagnostic::report::IntoReport;
    ///
    /// // Define function returning a value
    /// fn f() -> impl IntoReport<i32, Error> {
    ///     42
    /// }
    ///
    /// // Invoke function and create report
    /// let res = f().into_report();
    /// ```
    #[inline]
    fn into_report(self) -> Result<Report<T>, E> {
        Ok(Report::new(self))
    }
}

impl<T, E> IntoReport<T, E> for Result<T, E>
where
    E: Error,
{
    /// Creates a report from a value `T` in a result.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::io::Error;
    /// use zrx_diagnostic::report::IntoReport;
    ///
    /// // Define function returning a result
    /// fn f() -> impl IntoReport<i32, Error> {
    ///     Ok(42)
    /// }
    ///
    /// // Invoke function and create report
    /// let res = f().into_report();
    /// ```
    #[inline]
    fn into_report(self) -> Result<Report<T>, E> {
        self.map(Report::new)
    }
}

impl<T, E> IntoReport<T, E> for Result<Report<T>, E>
where
    E: Error,
{
    /// Returns the report in a result as is.
    #[inline]
    fn into_report(self) -> Result<Report<T>, E> {
        self
    }
}
