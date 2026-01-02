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

//! Diagnostic.

mod convert;
pub mod location;
mod macros;
pub mod report;
mod severity;
mod tag;

pub use convert::IntoDiagnostic;
pub use location::Location;
pub use severity::Severity;
pub use tag::Tag;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Diagnostic.
///
/// Diagnostics are messages that provide information about notable events that
/// occur during the execution of a function. This crate makes it easy to amend
/// and function to include diagnostics by returning an implementation of the
/// [`IntoReport`][] trait.
///
/// There are two primary cases for using diagnostics:
///
/// - __Integration__: By implementing the [`IntoDiagnostic`] trait, any type
///   can be converted into a [`Diagnostic`], which allows for the integration
///   of notable events originating in third-party libraries and tools into the
///   diagnostic system.
///
/// - __Information__: Logging is one of the primary use cases for diagnostics,
///   as it avoids requiring a central facility for logging with global state.
///   The macros [`error!`], [`warning!`] and friends allow to quickly create
///   diagnostics, also capturing location information.
///
/// While all members of this struct are public, there are also some dedicated
/// methods with identical names, providing a builder-like interface.
///
/// [`error!`]: crate::error!
/// [`warning!`]: crate::warning!
/// [`IntoReport`]: crate::diagnostic::report::IntoReport
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::{Diagnostic, Location, Severity};
///
/// // Create diagnostic
/// let diagnostic =
///     Diagnostic::new(Severity::Error, "File not found")
///         .location(Location::new("path/to/file.rs", (0, 0)))
///         .code(404);
/// ```
#[derive(Clone, Debug)]
pub struct Diagnostic {
    /// Severity.
    pub severity: Severity,
    /// Message.
    pub message: String,
    /// Location, optional.
    pub location: Option<Location>,
    /// Code, optional.
    pub code: Option<usize>,
    /// Tags.
    pub tags: Vec<Tag>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Diagnostic {
    /// Creates a diagnostic.
    ///
    /// While this method allows to create diagnostics, the macros [`error!`],
    /// [`warning!`] and friends are recommended, as they automatically capture
    /// the location, and provide a more convenient interface. Manual creation
    /// is intended for [`IntoDiagnostic`][] implementations.
    ///
    /// [`error!`]: crate::error!
    /// [`warning!`]: crate::warning!
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::{Diagnostic, Severity};
    ///
    /// // Create diagnostic
    /// let diagnostic =
    ///     Diagnostic::new(Severity::Error, "File not found");
    /// ```
    #[must_use]
    pub fn new<M>(severity: Severity, message: M) -> Self
    where
        M: Into<String>,
    {
        Self {
            severity,
            message: message.into(),
            location: None,
            code: None,
            tags: Vec::new(),
        }
    }

    /// Sets the location of the diagnostic.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::{Diagnostic, Location, Severity};
    ///
    /// // Create diagnostic and set location
    /// let diagnostic =
    ///     Diagnostic::new(Severity::Error, "File not found")
    ///         .location(Location::new("path/to/file.rs", (0, 0)));
    /// ```
    #[inline]
    #[must_use]
    pub fn location<L>(mut self, location: L) -> Self
    where
        L: Into<Location>,
    {
        self.location = Some(location.into());
        self
    }

    /// Sets the code of the diagnostic.
    ///
    /// In case your diagnostics are associated with a specific code, you can
    /// use this method to set it, which is useful for categorization.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::{Diagnostic, Severity};
    ///
    /// // Create diagnostic and set code
    /// let diagnostic =
    ///     Diagnostic::new(Severity::Error, "File not found")
    ///         .code(404);
    /// ```
    #[inline]
    #[must_use]
    pub fn code(mut self, code: usize) -> Self {
        self.code = Some(code);
        self
    }

    /// Adds a tag to the diagnostic.
    ///
    /// This method allows to add a tag to the diagnostic, which can be used for
    /// categorization or filtering. Tags are unique, so when a tag is already
    /// present, it won't be added. While only few tags are currently supported,
    /// we'll add more as we see fit.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::{Diagnostic, Severity, Tag};
    ///
    /// // Create diagnostic and add tags
    /// let diagnostic =
    ///     Diagnostic::new(Severity::Error, "File not found")
    ///         .tag(Tag::Unnecessary);
    /// ```
    #[inline]
    #[must_use]
    pub fn tag<T>(mut self, tag: T) -> Self
    where
        T: Into<Tag>,
    {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
        self
    }
}
