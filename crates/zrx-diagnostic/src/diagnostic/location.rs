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

//! Location.

use std::fmt::{self, Write};
use std::panic;

mod macros;
mod position;
mod range;

pub use position::Position;
pub use range::Range;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Location.
///
/// Locations represent ranges within text files, which can be used to provide
/// helpful context for a [`Diagnostic`][]. Each location must have a resolvable
/// URI, and a [`Range`] that indicates the start and end position within the
/// text file. URIs might contain absolute or relative paths.
///
/// [`Diagnostic`]: crate::diagnostic::Diagnostic
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Location {
    /// Document URI.
    pub uri: String,
    /// Document range.
    pub range: Range,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Location {
    /// Creates a location.
    ///
    /// In order to capture a specific location in a Rust source file, use the
    /// [`location!`][] macro, which captures precise line and column positions.
    ///
    /// [`location!`]: crate::location!
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::location::{Location, Position};
    ///
    /// // Create location
    /// let location =
    ///     Location::new("path/to/file.rs", Position::new(0, 0));
    /// ```
    #[inline]
    pub fn new<U, L>(uri: U, location: L) -> Self
    where
        U: Into<String>,
        L: Into<Range>,
    {
        Self {
            uri: uri.into(),
            range: location.into(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<&panic::Location<'_>> for Location {
    /// Converts a panic location into a location.
    fn from(location: &panic::Location) -> Self {
        Self::new(
            location.file(),
            Position::new(
                location.line().saturating_sub(1),
                location.column().saturating_sub(1),
            ),
        )
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for Location {
    /// Formats the location for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.uri.fmt(f)?;
        f.write_char(':')?;
        self.range.fmt(f)
    }
}
