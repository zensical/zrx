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

//! Position.

use std::fmt::{self, Write};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Position.
///
/// Positions are used to represent a specific point in a text file, typically
/// for error reporting or debugging purposes. They are defined by a line and
/// column number, both of which use 0-based indexing, which is what the
/// [Language Server Protocol][LSP] and [Source Maps] require.
///
/// However, when printing via [`fmt::Display`], the line and column numbers
/// are formatted as 1-based indices, which is what most editors use.
///
/// [LSP]: https://microsoft.github.io/language-server-protocol/
/// [Source Maps]: https://developer.mozilla.org/en-US/docs/Glossary/Source_map
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    /// Line number, 0-based.
    pub line: u32,
    /// Column number, 0-based.
    pub column: u32,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Position {
    /// Creates a position with the given line and column numbers.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::location::Position;
    ///
    /// // Create position
    /// let position = Position::new(0, 0);
    /// ```
    #[must_use]
    pub fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<(u32, u32)> for Position {
    /// Creates a position from a tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_diagnostic::location::Position;
    ///
    /// // Create position from tuple
    /// let position: Position = (0, 0).into();
    /// ```
    #[inline]
    fn from((line, column): (u32, u32)) -> Self {
        Self { line, column }
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for Position {
    /// Formats the position for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.line.saturating_add(1).fmt(f)?;
        f.write_char(':')?;
        self.column.saturating_add(1).fmt(f)
    }
}
