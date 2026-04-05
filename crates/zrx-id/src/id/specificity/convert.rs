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

//! Specificity computation.

use crate::id::expression::{Expression, Operand, Operator, Term};
use crate::id::matcher::selector::Selector;
use crate::id::Id;

use super::segment::atom::{Character, Wildcard};
use super::segment::{Atom, Segment, Segments, ToSegments};
use super::Specificity;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Computation of [`Specificity`].
pub trait ToSpecificity {
    /// Computes the specificity of the value.
    fn to_specificity(&self) -> Specificity;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl ToSpecificity for Expression {
    /// Computes the specificity of the expression.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::specificity::ToSpecificity;
    /// use zrx_id::{selector, Expression};
    ///
    /// // Create expression and compute specificity
    /// let expr = Expression::any(|expr| {
    ///     expr.with(selector!(location = "**/*.jpg")?)?
    ///         .with(selector!(location = "**/*.png")?)
    /// })?;
    /// assert_eq!(expr.to_specificity(), (0, 1, 1, 4).into());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_specificity(&self) -> Specificity {
        let iter = self.operands().iter().map(ToSpecificity::to_specificity);
        match self.operator() {
            Operator::Any => iter.reduce(Specificity::min).unwrap_or_default(),
            Operator::All => iter.reduce(Specificity::sum).unwrap_or_default(),
            Operator::Not => Specificity::default(),
        }
    }
}

impl ToSpecificity for Term {
    /// Computes the specificity of the term.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::expression::Term;
    /// use zrx_id::selector;
    /// use zrx_id::specificity::ToSpecificity;
    ///
    /// // Create term and compute specificity
    /// let term = Term::from(selector!(location = "**/*.md")?);
    /// assert_eq!(term.to_specificity(), (0, 1, 1, 3).into());
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::expression::Operand;
    /// use zrx_id::selector;
    /// use zrx_id::specificity::ToSpecificity;
    ///
    /// // Create operand and compute specificity
    /// let operand = Operand::from(selector!(location = "**/*.md")?);
    /// assert_eq!(operand.to_specificity(), (0, 1, 1, 3).into());
    /// # Ok(())
    /// # }
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::id;
    /// use zrx_id::specificity::ToSpecificity;
    ///
    /// // Create identifier and compute specificity
    /// let id = id!(provider = "file", context = ".", location = "index.md")?;
    /// assert_eq!(id.to_specificity(), (3, 0, 0, 13).into());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_specificity(&self) -> Specificity {
        let iter = 1..7;
        iter.map(|index| self.as_ref().get(index).to_specificity())
            .reduce(Specificity::sum)
            .unwrap_or_default()
    }
}

impl ToSpecificity for Selector {
    /// Computes the specificity of the selector.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::ToSpecificity;
    ///
    /// // Create selector and compute specificity
    /// let selector = selector!(location = "**/*.md")?;
    /// assert_eq!(selector.to_specificity(), (0, 1, 1, 3).into());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_specificity(&self) -> Specificity {
        let iter = 1..7;
        iter.map(|index| self.as_ref().get(index).to_specificity())
            .reduce(Specificity::sum)
            .unwrap_or_default()
    }
}

// ----------------------------------------------------------------------------

impl ToSpecificity for Segments<'_> {
    /// Computes the specificity of the segment set.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.iter()
            .map(ToSpecificity::to_specificity)
            .reduce(Specificity::sum)
            .unwrap_or_default()
    }
}

impl ToSpecificity for Segment<'_> {
    /// Computes the specificity of the segment.
    #[inline]
    fn to_specificity(&self) -> Specificity {
        self.iter()
            .map(ToSpecificity::to_specificity)
            .reduce(Specificity::min_sum_len)
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
                .reduce(Specificity::min)
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
    /// Computes the specificity of the character class.
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
