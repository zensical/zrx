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

//! Report.

use std::slice::Iter;

use super::convert::IntoDiagnostic;
use super::Diagnostic;

mod convert;
mod ext;

pub use convert::IntoReport;
pub use ext::ResultExt;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Report.
///
/// Reports allow to annotate any data of type `T` with [`Diagnostic`] messages,
/// including distinct notable event ranging from [`debug!`][] to [`error!`][].
/// While other diagnostic frameworks in Rust attach diagnostics to the [`Err`]
/// variant of a given [`Result`], we attach them to the [`Ok`] variant.
///
/// Note that reports should rarely need to be constructed imperatively, but
/// through [`IntoReport`], performing the conversion automatically.
///
/// [`debug!`]: crate::debug!
/// [`error!`]: crate::error!
/// [`Diagnostic`]: crate::diagnostic::Diagnostic
/// [`Report`]: crate::diagnostic::report::Report
#[derive(Clone, Debug, Default)]
pub struct Report<T = ()> {
    /// Annotated data.
    pub data: T,
    /// Diagnostics.
    diagnostics: Vec<Diagnostic>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Report<T> {
    /// Creates a report.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    ///
    /// // Create report
    /// let report = Report::new(42);
    /// ```
    #[must_use]
    pub fn new(data: T) -> Self {
        Self { data, diagnostics: Vec::new() }
    }

    /// Extends the report with the given diagnostics.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    /// use zrx_diagnostic::hint;
    ///
    /// // Create report with diagnostics
    /// let report = Report::new(42).with([
    ///     hint!("Insufficient data for meaningful answer"),
    /// ]);
    /// ```
    #[must_use]
    pub fn with<D>(mut self, diagnostics: D) -> Self
    where
        D: IntoIterator,
        D::Item: IntoDiagnostic,
    {
        for diagnostic in diagnostics {
            self.add(diagnostic);
        }
        self
    }

    /// Adds a diagnostic to the report.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    /// use zrx_diagnostic::hint;
    ///
    /// // Create report and add diagnostics
    /// let mut report = Report::new(42);
    /// report
    ///     .add(hint!("Insufficient data for meaningful answer"))
    ///     .add(hint!("Try to be more specific next time"));
    /// ```
    #[inline]
    pub fn add<D>(&mut self, diagnostic: D) -> &mut Self
    where
        D: IntoDiagnostic,
    {
        self.diagnostics.push(diagnostic.into_diagnostic());
        self
    }

    /// Merges with the given report, returning its annotated data.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    /// use zrx_diagnostic::hint;
    ///
    /// // Create report with diagnostics
    /// let report = Report::new(42).with([
    ///     hint!("Insufficient data for meaningful answer"),
    /// ]);
    ///
    /// // Merge reports and obtain data
    /// let mut target = Report::new(());
    /// let data = target.merge(report);
    /// assert_eq!(data, 42);
    /// ```
    pub fn merge<U>(&mut self, report: Report<U>) -> U {
        for diagnostic in report.diagnostics {
            self.add(diagnostic);
        }
        report.data
    }

    /// Maps the annotated data to a different type.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    ///
    /// // Create report and map data
    /// let report = Report::new(42).map(|data| data.to_string());
    /// ```
    #[inline]
    pub fn map<F, U>(self, f: F) -> Report<U>
    where
        F: FnOnce(T) -> U,
    {
        Report {
            data: f(self.data),
            diagnostics: self.diagnostics,
        }
    }

    /// Creates an iterator over the diagnostics of the report.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    /// use zrx_diagnostic::hint;
    ///
    /// // Create report and add diagnostics
    /// let mut report = Report::new(42);
    /// report
    ///     .add(hint!("Insufficient data for meaningful answer"))
    ///     .add(hint!("Try to be more specific next time"));
    ///
    /// // Create iterator over diagnostics
    /// for diagnostic in report.iter() {
    ///     println!("{diagnostic:?}");
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, Diagnostic> {
        self.diagnostics.iter()
    }
}

#[allow(clippy::must_use_candidate)]
impl<T> Report<T> {
    /// Returns the number of diagnostics.
    #[inline]
    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    /// Returns whether there are any diagnostics.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<D> FromIterator<D> for Report
where
    D: IntoDiagnostic,
{
    /// Creates a report from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    /// use zrx_diagnostic::hint;
    ///
    /// // Create report and add diagnostic
    /// let mut report = Report::from_iter([
    ///     hint!("Insufficient data for meaningful answer"),
    /// ]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = D>,
    {
        let mut report = Self::new(());
        for diagnostic in iter {
            report.add(diagnostic);
        }
        report
    }
}

impl<'a, T> IntoIterator for &'a Report<T> {
    type Item = &'a Diagnostic;
    type IntoIter = Iter<'a, Diagnostic>;

    /// Creates an iterator over the diagnostics of the report.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::report::Report;
    /// use zrx_diagnostic::hint;
    ///
    /// // Create report and add diagnostics
    /// let mut report = Report::new(42);
    /// report
    ///     .add(hint!("Insufficient data for meaningful answer"))
    ///     .add(hint!("Try to be more specific next time"));
    ///
    /// // Create iterator over diagnostics
    /// for diagnostic in &report {
    ///     println!("{diagnostic:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
