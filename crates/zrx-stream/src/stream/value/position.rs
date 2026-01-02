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

use zrx_scheduler::Value;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Position.
///
/// This data type attaches an index to a value, indicating its position within
/// a sorted stream. It is primarily used in the context of [`Stream::sort`][],
/// which keeps an internal store of items and updates their positions as the
/// items entering the stream change.
///
/// Note that ordering of positions is determined by their index, and in case
/// a tie-breaker is necessary, by the value itself.
///
/// [`Stream::sort`]: crate::stream::Stream::sort
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position<T> {
    /// Index in sort order.
    pub index: usize,
    /// Associated value.
    pub value: T,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Position<T> {
    /// Creates a position.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::value::Position;
    ///
    /// // Create position
    /// let position = Position::new(0, "a");
    /// ```
    pub fn new(index: usize, value: T) -> Self {
        Self { index, value }
    }

    /// Returns the index and associated value, consuming the position.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::value::Position;
    ///
    /// // Create position
    /// let position = Position::new(0, "a");
    /// assert_eq!(
    ///     position.into_parts(),
    ///     (0, "a"),
    /// );
    /// ```
    #[inline]
    pub fn into_parts(self) -> (usize, T) {
        (self.index, self.value)
    }

    /// Returns the associated value, consuming the position.
    ///
    /// ```
    /// use zrx_stream::value::Position;
    ///
    /// // Create position
    /// let position = Position::new(0, "a");
    /// assert_eq!(position.into_value(), "a");
    /// ```
    #[inline]
    pub fn into_value(self) -> T {
        self.value
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> Value for Position<T> where T: Value {}
