// Copyright (c) 2024 Zensical <contributors@zensical.org>

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

//! Span.

use std::ops::Range;

use super::error::{Error, Result};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Span.
///
/// Spans are structurally equivalent to [`Range`], which they can be converted
/// into, but define some methods that make them more convenient in our special
/// case. Spans use [`u16`] instead of [`usize`] to save on memory, as we don't
/// ever expect identifiers to exceed lengths of 65,535 bytes.
///
/// Note that spans must  always be inclusive on the start and exclusive on the
/// end, and do not allow the start to be greater than the end.
#[derive(Clone, Copy, Debug)]
pub struct Span {
    /// Start of the span.
    start: u16,
    /// End of the span.
    end: u16,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Span {
    /// Creates a span at the given position.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::span::Span;
    ///
    /// // Create span
    /// let span = Span::new(0, 2);
    /// ```
    #[must_use]
    pub const fn new(start: u16, end: u16) -> Self {
        debug_assert!(start <= end);
        Self { start, end }
    }

    /// Shifts the span.
    ///
    /// # Errors
    ///
    /// If the span overflows, [`Error::Length`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::span::Span;
    ///
    /// // Create and shift span
    /// let mut span = Span::new(0, 2);
    /// span.shift(2);
    /// assert_eq!(2..4, span.into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn shift(&mut self, by: i16) -> Result {
        if by >= 0 {
            self.shift_end(by)?;
            self.shift_start(by)
        } else {
            self.shift_start(by)?;
            self.shift_end(by)
        }
    }

    /// Shifts the start of the span.
    ///
    /// # Errors
    ///
    /// If the span overflows, [`Error::Length`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::span::Span;
    ///
    /// // Create and shift span
    /// let mut span = Span::new(2, 4);
    /// span.shift_start(-2)?;
    /// assert_eq!(0..4, span.into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn shift_start(&mut self, by: i16) -> Result {
        self.start
            .checked_add_signed(by)
            .ok_or(Error::Length)
            .map(|value| {
                debug_assert!(value <= self.end);
                self.start = value;
            })
    }

    /// Shifts the end of the span.
    ///
    /// # Errors
    ///
    /// If the span overflows, [`Error::Length`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::span::Span;
    ///
    /// // Create and shift span
    /// let mut span = Span::new(0, 2);
    /// span.shift_end(2);
    /// assert_eq!(0..4, span.into());
    /// # Ok(())
    /// # }
    /// ```
    pub fn shift_end(&mut self, by: i16) -> Result {
        self.end
            .checked_add_signed(by)
            .ok_or(Error::Length)
            .map(|value| {
                debug_assert!(value >= self.start);
                self.end = value;
            })
    }
}

#[allow(clippy::must_use_candidate)]
impl Span {
    /// Returns the length of the span.
    #[inline]
    pub fn len(&self) -> u16 {
        self.end - self.start
    }

    /// Returns whether the span is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<Span> for Range<T>
where
    T: From<u16>,
{
    /// Creates a range from a span.
    ///
    /// This method is provided for convenience to more easily convert between
    /// [`Span`] using [`u16`], and a [`Range`] using [`usize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::span::Span;
    ///
    /// // Create range from span
    /// let span = Span::new(0, 2);
    /// assert_eq!(0..2, span.into());
    /// ```
    #[inline]
    fn from(span: Span) -> Self {
        span.start.into()..span.end.into()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Initialize a set of spans.
///
/// This is a `const` function that allows to create a set of empty spans which
/// are spaced by a separator, executed at compile time.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub const fn init<const N: usize>() -> [Span; N] {
    let mut spans = [Span::new(0, 0); N];
    let mut index = 0;
    while index < N {
        let at = index as u16;
        spans[index] = Span::new(at, at);
        index += 1;
    }
    spans
}
