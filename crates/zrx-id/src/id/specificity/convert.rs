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

//! Specificity conversion.

use std::cmp;

use super::segment::atom::{Character, Wildcard};
use super::segment::convert::ToSegments;
use super::segment::{Atom, Segment, Segments};
use super::Specificity;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Computes the [`Specificity`].
pub trait IntoSpecificity {
    /// Computes the specificity of the value.
    fn into_specificity(self) -> Specificity;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl IntoSpecificity for Segments<'_> {
    /// Computes the specificity of the segments set.
    #[inline]
    fn into_specificity(self) -> Specificity {
        self.into_iter()
            .map(IntoSpecificity::into_specificity)
            .reduce(Specificity::sum)
            .unwrap_or_default()
    }
}

impl IntoSpecificity for Segment<'_> {
    /// Computes the specificity of the segment.
    #[inline]
    fn into_specificity(self) -> Specificity {
        self.into_iter()
            .map(IntoSpecificity::into_specificity)
            .reduce(Specificity::min)
            .unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------

impl IntoSpecificity for Atom<'_> {
    /// Computes the specificity of the atom.
    #[inline]
    fn into_specificity(self) -> Specificity {
        match self {
            Atom::Literal(literal) => {
                let len = u16::try_from(literal.len()).unwrap_or(u16::MAX);
                Specificity(1, 0, 0, len)
            }
            Atom::Wildcard(wildcard) => wildcard.into_specificity(),
            Atom::Character(character) => character.into_specificity(),
            Atom::Group(data) => data
                .into_iter()
                .map(IntoSpecificity::into_specificity)
                .reduce(cmp::min)
                .unwrap_or_default(),
        }
    }
}

impl IntoSpecificity for Wildcard {
    /// Computes the specificity of the wildcard.
    #[inline]
    fn into_specificity(self) -> Specificity {
        match self {
            Wildcard::Character => Specificity(0, 1, 0, 0),
            Wildcard::Sequence => Specificity(0, 1, 0, 0),
            Wildcard::Traversal => Specificity(0, 0, 1, 0),
        }
    }
}

impl IntoSpecificity for Character<'_> {
    /// Computes the specificity of the character.
    #[inline]
    fn into_specificity(self) -> Specificity {
        Specificity(0, 1, 0, 1)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> IntoSpecificity for T
where
    T: ToSegments,
{
    #[inline]
    fn into_specificity(self) -> Specificity {
        self.to_segments().into_specificity()
    }
}
