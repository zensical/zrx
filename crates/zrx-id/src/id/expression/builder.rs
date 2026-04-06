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

//! Expression builder.

use super::error::Result;
use super::operand::{Operand, Operator, TryIntoOperand};
use super::Expression;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Expression builder.
#[derive(Debug)]
pub struct Builder {
    /// Expression operator.
    operator: Operator,
    /// Expression operands.
    operands: Vec<Operand>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Expression {
    /// Creates an expression for which any operand must match.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`][] if any of the operands are invalid.
    ///
    /// [`Error::Id`]: crate::id::expression::Error::Id
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
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn any<F>(f: F) -> Result<Self>
    where
        F: FnOnce(Builder) -> Result<Builder>,
    {
        f(Builder {
            operator: Operator::Any,
            operands: Vec::new(),
        })
        .map(Builder::build)
    }

    /// Creates an expression for which all operands must match.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`][] if any of the operands are invalid.
    ///
    /// [`Error::Id`]: crate::id::expression::Error::Id
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
    ///         .with(selector!(provider = "file")?)
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn all<F>(f: F) -> Result<Self>
    where
        F: FnOnce(Builder) -> Result<Builder>,
    {
        f(Builder {
            operator: Operator::All,
            operands: Vec::new(),
        })
        .map(Builder::build)
    }

    /// Creates an expression for which no operand must match.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`][] if any of the operands are invalid.
    ///
    /// [`Error::Id`]: crate::id::expression::Error::Id
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
    #[inline]
    pub fn not<F>(f: F) -> Result<Self>
    where
        F: FnOnce(Builder) -> Result<Builder>,
    {
        f(Builder {
            operator: Operator::Not,
            operands: Vec::new(),
        })
        .map(Builder::build)
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Adds an operand to the expression.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`][] if any of the operands are invalid.
    ///
    /// [`Error::Id`]: crate::id::expression::Error::Id
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
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with<T>(mut self, operand: T) -> Result<Self>
    where
        T: TryIntoOperand,
    {
        self.operands.push(operand.try_into_operand()?);
        Ok(self)
    }

    /// Builds the expression.
    ///
    /// This method is private, as building is done implicitly through the
    /// construction methods defined as part of [`Expression`].
    #[must_use]
    fn build(self) -> Expression {
        Expression {
            operator: self.operator,
            operands: self.operands,
        }
    }
}
