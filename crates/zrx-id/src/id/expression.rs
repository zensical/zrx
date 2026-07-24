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

//! Expression.

use std::vec::IntoIter;

use zrx_scheduler::Value;

mod builder;
mod condition;
mod error;
pub mod filter;
mod operand;

pub use builder::Builder;
pub use error::{Error, Result};
pub use filter::Filter;
pub use operand::{Operand, Operator, Term};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Expression.
///
/// Expressions allow to build trees of [`Id`][] and [`Selector`][] instances
/// combined with logical operators, enabling complex matching and filtering.
///
/// The following operators are supported:
///
/// - [`Expression::any`]: Logical `OR` - any operand must match.
/// - [`Expression::all`]: Logical `AND` - all operands must match.
/// - [`Expression::not`]: Logical `NOT` - no operand must match.
///
/// [`Id`]: crate::id::Id
/// [`Selector`]: crate::id::selector::Selector
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::{selector, Expression};
///
/// // Create expression
/// let expr = Expression::all(|expr| {
///     expr.with(selector!(location = "**/*.md")?)?
///         .with(Expression::not(|expr| {
///             expr.with(selector!(provider = "file")?)
///         })
///     )
/// })?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expression {
    /// Expression operator.
    operator: Operator,
    /// Expression operands.
    operands: Vec<Operand>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

#[allow(clippy::must_use_candidate)]
impl Expression {
    /// Returns the operator.
    #[inline]
    pub fn operator(&self) -> Operator {
        self.operator
    }

    /// Returns a reference to the operands.
    #[inline]
    pub fn operands(&self) -> &[Operand] {
        &self.operands
    }

    /// Returns the number of operands.
    #[inline]
    pub fn len(&self) -> usize {
        self.operands.len()
    }

    /// Returns whether there are any operands.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.operands.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for Expression {}

// ----------------------------------------------------------------------------

impl<T> From<T> for Expression
where
    T: Into<Term>,
{
    /// Creates an expression from a term.
    #[inline]
    fn from(term: T) -> Self {
        let term = Operand::from(term.into());
        Expression {
            operator: Operator::Any,
            operands: Vec::from([term]),
        }
    }
}

// ----------------------------------------------------------------------------

impl IntoIterator for Expression {
    type Item = Operand;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates a consuming iterator over the expression.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Expression};
    ///
    /// // Create expression
    /// let expr = Expression::any(|expr| {
    ///     expr.with(selector!(location = "**/*.jpg")?)?
    ///         .with(selector!(location = "**/*.png")?)
    /// })?;
    ///
    /// // Create iterator over expression
    /// for operand in expr {
    ///     println!("{operand:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.operands.into_iter()
    }
}

// ----------------------------------------------------------------------------

impl Default for Expression {
    /// Creates an expression that matches everything.
    ///
    /// While it may seem counterintuitive to have the default expression match
    /// everything, it is designed this way to align with the concept of vacuous
    /// truth in logic. An expression with no operands is considered to be true,
    /// as there are no conditions to violate it.
    ///
    /// In our implementation, we use an empty expression with a logical `NOT`
    /// operator to represent this concept, to be distinguishable as a marker.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Expression;
    ///
    /// // Create empty expression
    /// let expr = Expression::default();
    /// assert!(expr.is_empty());
    /// ```
    #[inline]
    fn default() -> Self {
        Self {
            operator: Operator::Not,
            operands: Vec::new(),
        }
    }
}
