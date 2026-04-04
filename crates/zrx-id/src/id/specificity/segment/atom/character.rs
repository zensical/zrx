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

//! Character class.

use std::fmt::{self, Display, Write};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Character class.
///
/// For our case, we do not need to know the exact structure of the character
/// class, as we'll score it as a single character anyway, same as `*`. We also
/// don't care whether it's negated or not, as that doesn't affect scoring as
/// well. Therefore, we can just store the string slices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Character<'a> {
    /// String slices.
    values: Vec<&'a str>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> FromIterator<&'a str> for Character<'a> {
    /// Creates a character class from an iterator.
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = &'a str>,
    {
        Character {
            values: iter.into_iter().collect(),
        }
    }
}

// ----------------------------------------------------------------------------

impl Display for Character<'_> {
    /// Formats the character class for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_char('[')?;
        for value in &self.values {
            f.write_str(value)?;
        }
        f.write_char(']')
    }
}
