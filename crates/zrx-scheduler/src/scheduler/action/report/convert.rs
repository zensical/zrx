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

use zrx_diagnostic::report::Report;

use crate::scheduler::action::error::IntoError;
use crate::scheduler::action::Error;
use crate::scheduler::value::Value;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Report`]
///
/// This trait provides a focused implementation of the [`IntoReport`][] trait,
/// as defined in the diagnostics crate, adding the appropriate trait bounds to
/// the implementations, so that anything that implements [`Value`] can be used
/// as a report value, providing a more ergonomic API.
///
/// For more information on why this is necessary, see [`IntoReport`][].
///
/// [`IntoReport`]: zrx_diagnostic::report::IntoReport
pub trait IntoReport<T = ()> {
    /// Converts into a report.
    ///
    /// # Errors
    ///
    /// Note that this method never returns an error by itself, as this trait is
    /// primarily provided for wrapping values returned from infallible methods.
    /// Consequentially, this trait isn't named `TryIntoReport`, as it's not
    /// falling under Rust's try-semantics.
    fn into_report(self) -> Result<Report<T>, Error>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> IntoReport<T> for T
where
    T: Value,
{
    /// Creates a report from a value `T` and wraps it in a result.
    #[inline]
    fn into_report(self) -> Result<Report<T>, Error> {
        Ok(Report::new(self))
    }
}

impl<T> IntoReport<T> for Report<T>
where
    T: Value,
{
    /// Returns the report as is and wraps it in a result.
    #[inline]
    fn into_report(self) -> Result<Report<T>, Error> {
        Ok(self)
    }
}

impl<T, E> IntoReport<T> for Result<T, E>
where
    T: Value,
    E: IntoError,
{
    /// Creates a report from a value `T` in a result.
    #[inline]
    fn into_report(self) -> Result<Report<T>, Error> {
        self.map_err(IntoError::into_error).map(Report::new)
    }
}

impl<T, E> IntoReport<T> for Result<Report<T>, E>
where
    T: Value,
    E: IntoError,
{
    /// Returns the report in a result as is.
    #[inline]
    fn into_report(self) -> Result<Report<T>, Error> {
        self.map_err(IntoError::into_error)
    }
}
