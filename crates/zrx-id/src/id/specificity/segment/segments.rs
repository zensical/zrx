// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the `Software`), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED `AS IS`, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Segment set.

use std::fmt::{self, Display, Write};
use std::slice::Iter;

use super::Segment;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Segment set.
///
/// Segment sets are derived from strings that contain [`Glob`][] expressions,
/// where each [`Segment`] is separated by a `/`, and consists of atoms.
///
/// [`Glob`]: globset::Glob
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segments<'a> {
    /// Vector of segments.
    inner: Vec<Segment<'a>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Segments<'_> {
    /// Creates an iterator over the segment set.
    #[inline]
    pub fn iter(&self) -> Iter<'_, Segment<'_>> {
        self.inner.iter()
    }
}

#[allow(clippy::must_use_candidate)]
impl Segments<'_> {
    /// Returns the number of segments.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any segments.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> FromIterator<Segment<'a>> for Segments<'a> {
    /// Creates a segment set from an iterator.
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Segment<'a>>,
    {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<'a> IntoIterator for &'a Segments<'a> {
    type Item = &'a Segment<'a>;
    type IntoIter = Iter<'a, Segment<'a>>;

    /// Creates an iterator over the segment set.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ----------------------------------------------------------------------------

impl Display for Segments<'_> {
    /// Formats the segment set for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, segment) in self.inner.iter().enumerate() {
            Display::fmt(segment, f)?;

            // Write separator if not last
            if i < self.inner.len() - 1 {
                f.write_char('/')?;
            }
        }

        // No errors occurred
        Ok(())
    }
}
