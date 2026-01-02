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

//! Range.

use std::fmt::{self, Write};

use super::position::Position;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Range.
///
/// Ranges are used to represent start and end positions of text in a text file,
/// which is particularly useful for error reporting and debugging purposes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Range {
    /// Start position.
    pub start: Position,
    /// End position.
    pub end: Position,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Range {
    /// Creates a range with the given start and end positions.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::location::{Position, Range};
    ///
    /// // Create range
    /// let range = Range::new(
    ///     Position::new(0, 0),
    ///     Position::new(4, 0)
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn new<S, E>(start: S, end: E) -> Self
    where
        S: Into<Position>,
        E: Into<Position>,
    {
        Self {
            start: start.into(),
            end: end.into(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<P> From<P> for Range
where
    P: Into<Position>,
{
    /// Creates a range at a single position.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::location::{Position, Range};
    ///
    /// // Create range from position
    /// let range = Range::from(Position::new(0, 0));
    /// ```
    #[inline]
    fn from(position: P) -> Self {
        let start = position.into();
        Self { start, end: start }
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for Range {
    /// Formats the range for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.start.fmt(f)?;
        if self.start != self.end {
            f.write_char('-')?;
            self.end.fmt(f)?;
        }

        // No errors occurred
        Ok(())
    }
}
