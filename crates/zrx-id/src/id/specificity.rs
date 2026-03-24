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

//! Specificity.

use std::cmp::{self, Ordering};

mod convert;
pub mod segment;
mod specified;
mod tokens;

pub use convert::ToSpecificity;
pub use specified::Specified;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Specificity.
///
/// Specificity is an ordering and tie-breaking concept borrowed from CSS, where
/// more specific selectors take precedence over less specific ones. Specificity
/// is computable for the likes of [`Expression`][], [`Term`][], [`Operand`][],
/// [`Id`][], [`Selector`][], and [`Glob`][].
///
/// # Representation
///
/// Specificity is represented as a 4-tuple `(a, b, c, l)`, which is compared
/// in lexicographic order, meaning that components are compared in sequence:
///
/// - `a` – number of segments with literals only, e.g. `src`, `main.rs`.
/// - `b` – number of segments with single-wildcards, e.g. `*`, `?`, `[abc]`.
/// - `c` – number of segments with double-wildcards, compared in reverse.
/// - `l` – number of literals across all segments.
///
/// # Atoms
///
/// In a [`Segment`][], atoms are combined with [`Specificity::min_sum_len`] -
/// the structural component `a`, `b`, or `c` is assigned by taking the minimum
/// across all atoms in the segment, whereas the length component `l` receives
/// the sum across all atoms in the segment.
///
/// # Ids and selectors
///
/// The specificity of an [`Id`][] or [`Selector`][] is computed by summing the
/// specificities of its components, where the specificity of each component is
/// computed individually and then combined with [`Specificity::sum`][]. Empty
/// components receive the [`Specificity::default`], which is `(0, 0, 0, 0)`.
///
/// ``` sh
/// zrs:{git,file}:::{docs}:index.md: # (3, 0, 0, 15)
/// zrs::::docs:{index,about}.md:     # (2, 0, 0, 12)
/// zrs:::::index.{md,rst}:           # (1, 0, 0, 8)
/// zrs:::::{*}:                      # (0, 1, 0, 0)
/// ```
///
/// # Expressions
///
/// An [`Expression`][] is a combination of multiple [`Id`][] and [`Selector`][]
/// terms, with its specificity computed according to its [`Operator`][]:
///
/// - [`Expression::any`][]: takes the minimum. The expression is as specific
///   as its least specific operand, since any operand can match.
///
/// - [`Expression::all`][]: sums specificities. The expression is as specific
///   as the combination of all its operands, since all operands must match.
///
/// - [`Expression::not`][]: contributes nothing, i.e., `(0, 0, 0, 0)`, since
///   a negation is a guard that filters matches but does not select them.
///
/// Alternate groups, e.g. `{jpg,png}`, are equivalent to [`Expression::any`][]
/// at the [`Atom`][] level and follow the same rules.
///
/// [`Atom`]: crate::id::specificity::segment::Atom
/// [`Expression`]: crate::id::filter::Expression
/// [`Expression::all`]: crate::id::filter::Expression::all
/// [`Expression::any`]: crate::id::filter::Expression::any
/// [`Expression::not`]: crate::id::filter::Expression::not
/// [`Glob`]: globset::Glob
/// [`Id`]: crate::id::Id
/// [`Operand`]: crate::id::filter::expression::Operand
/// [`Operator`]: crate::id::filter::expression::Operator
/// [`Segment`]: crate::id::specificity::segment::Segment
/// [`Selector`]: crate::id::matcher::selector::Selector
/// [`Term`]: crate::id::filter::Term
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::filter::Expression;
/// use zrx_id::selector;
/// use zrx_id::specificity::ToSpecificity;
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Specificity(u16, u16, u16, u16);

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Specificity {
    /// Computes the sum of both specificities.
    #[inline]
    fn sum(self, other: Self) -> Self {
        let Specificity(a1, b1, c1, l1) = self;
        let Specificity(a2, b2, c2, l2) = other;
        Self(
            a1.saturating_add(a2),
            b1.saturating_add(b2),
            c1.saturating_add(c2),
            l1.saturating_add(l2),
        )
    }

    /// Computes the minimum of both specificities.
    #[inline]
    fn min(self, other: Self) -> Self {
        cmp::min(self, other)
    }

    /// Computes the minimum of both specificities, summing their lengths.
    #[inline]
    fn min_sum_len(self, other: Self) -> Self {
        let mut spec = cmp::min(self, other);
        spec.3 = self.3.saturating_add(other.3);
        spec
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<T> for Specificity
where
    T: ToSpecificity,
{
    /// Creates a specificity from a value.
    #[inline]
    fn from(value: T) -> Self {
        value.to_specificity()
    }
}

impl From<(u16, u16, u16, u16)> for Specificity {
    /// Creates a specificity from a tuple.
    #[inline]
    fn from((a, b, c, l): (u16, u16, u16, u16)) -> Self {
        Self(a, b, c, l)
    }
}

// ----------------------------------------------------------------------------

impl PartialOrd for Specificity {
    /// Orders two specificities.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::ToSpecificity;
    ///
    /// // Create and compare selectors by specificity
    /// let a = selector!(location = "**/*.md")?;
    /// let b = selector!(location = "*.md")?;
    /// assert!(a.to_specificity() < b.to_specificity());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Specificity {
    /// Orders two specificities.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::ToSpecificity;
    ///
    /// // Create and compare selectors by specificity
    /// let a = selector!(location = "**/*.md")?;
    /// let b = selector!(location = "*.md")?;
    /// assert!(a.to_specificity() < b.to_specificity());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let Specificity(a1, b1, c1, l1) = self;
        let Specificity(a2, b2, c2, l2) = other;

        // An all-zero specificity is the least specific and must always be the
        // first in order, so check if this applies to any of the specificities
        if a1 | b1 | c1 | l1 == 0 {
            return Ordering::Less;
        }
        if a2 | b2 | c2 | l2 == 0 {
            return Ordering::Greater;
        }

        // Otherwise, compare each component, where `c` is reversed since fewer
        // double-wildcards is more specific than more double-wildcards
        a1.cmp(a2)
            .then_with(|| b1.cmp(b2))
            .then_with(|| c2.cmp(c1))
            .then_with(|| l1.cmp(l2))
    }
}
