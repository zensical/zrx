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

use crate::id::filter::expression::{Operand, Operator};
use crate::id::filter::{Expression, Term};
use crate::id::format::Format;
use crate::id::matcher::selector::Selector;
use crate::id::Id;

use super::segment::atom::{Character, Wildcard};
use super::segment::convert::ToSegments;
use super::segment::{Atom, Segment, Segments};
use super::Specificity;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Computes the [`Specificity`].
pub trait ToSpecificity {
    /// Computes the specificity of the value.
    fn to_specificity(&self) -> Specificity;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl ToSpecificity for Expression {
    /// Computes the specificity of the expression.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        let iter = self.operands().iter().map(ToSpecificity::to_specificity);
        match self.operator() {
            Operator::Any => iter.reduce(Specificity::any).unwrap_or_default(),
            Operator::All => iter.reduce(Specificity::all).unwrap_or_default(),
            Operator::Not => Specificity::default(),
        }
    }
}

impl ToSpecificity for Term {
    /// Computes the specificity of the term.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        match self {
            Term::Id(id) => id.to_specificity(),
            Term::Selector(selector) => selector.to_specificity(),
        }
    }
}

impl ToSpecificity for Operand {
    /// Computes the specificity of the operand.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        match self {
            Operand::Expression(expr) => expr.to_specificity(),
            Operand::Term(term) => term.to_specificity(),
        }
    }
}

// ----------------------------------------------------------------------------

impl ToSpecificity for Id {
    /// Computes the specificity of the identifier.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.as_ref().to_specificity()
    }
}

impl ToSpecificity for Selector {
    /// Computes the specificity of the selector.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.as_ref().to_specificity()
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize> ToSpecificity for Format<N> {
    /// Computes the specificity of the formatted string.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        let iter = 0..N;
        iter.map(|index| self.get(index).to_specificity())
            .reduce(Specificity::all)
            .unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------

impl ToSpecificity for Segments<'_> {
    /// Computes the specificity of the segments set.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.iter()
            .map(ToSpecificity::to_specificity)
            .reduce(Specificity::all)
            .unwrap_or_default()
    }
}

impl ToSpecificity for Segment<'_> {
    /// Computes the specificity of the segment.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.iter()
            .map(ToSpecificity::to_specificity)
            .reduce(Specificity::any)
            .unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------

impl ToSpecificity for Atom<'_> {
    /// Computes the specificity of the atom.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        match self {
            Atom::Literal(literal) => {
                let len = u16::try_from(literal.len()).unwrap_or(u16::MAX);
                Specificity(1, 0, 0, len)
            }
            Atom::Wildcard(wildcard) => wildcard.to_specificity(),
            Atom::Character(character) => character.to_specificity(),
            Atom::Group(data) => data
                .iter()
                .map(ToSpecificity::to_specificity)
                .reduce(cmp::min)
                .unwrap_or_default(),
        }
    }
}

impl ToSpecificity for Wildcard {
    /// Computes the specificity of the wildcard.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        match self {
            Wildcard::Character => Specificity(0, 1, 0, 0),
            Wildcard::Sequence => Specificity(0, 1, 0, 0),
            Wildcard::Traversal => Specificity(0, 0, 1, 0),
        }
    }
}

impl ToSpecificity for Character<'_> {
    /// Computes the specificity of the character.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        Specificity(0, 1, 0, 1)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> ToSpecificity for T
where
    T: ToSegments,
{
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.to_segments().to_specificity()
    }
}
