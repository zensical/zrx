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

//! Atom.

use std::fmt::{self, Display, Write};

use super::set::Segments;

mod character;
mod wildcard;

pub use character::Character;
pub use wildcard::Wildcard;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Atom.
///
/// Atoms are the basic building blocks of [`Segments`], representing literals,
/// wildcards, character classes and groups of alternatives. Each [`Segment`][]
/// contains a set of atoms that define which [`Specificity`][] the segment has,
/// where specificity is determined by the least specific atom in the segment.
///
/// [`Segment`]: crate::id::specificity::segment::Segment
/// [`Specificity`]: crate::id::specificity::Specificity
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Atom<'a> {
    /// Literal, e.g., `main.rs`
    Literal(&'a str),
    /// Wildcard, i.e., `?`, `*`, or `**`.
    Wildcard(Wildcard),
    /// Character class, e.g., `[xyz]`.
    Character(Character<'a>),
    /// Alternate group, e.g., `{*.rs,*.md}`.
    Group(Vec<Segments<'a>>),
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Display for Atom<'_> {
    /// Formats the atom for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Atom::Literal(literal) => Display::fmt(literal, f),
            Atom::Wildcard(wildcard) => Display::fmt(wildcard, f),
            Atom::Character(character) => Display::fmt(character, f),
            Atom::Group(group) => {
                f.write_char('{')?;
                for (i, segments) in group.iter().enumerate() {
                    Display::fmt(&segments, f)?;

                    // Write comma if not last
                    if i < group.len() - 1 {
                        f.write_char(',')?;
                    }
                }
                f.write_char('}')
            }
        }
    }
}
