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

use crate::matcher::expression::operand::TryIntoOperand;

use super::operand::{Operand, Operator, Result};
use super::Expression;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Expression builder.
#[derive(Clone, Debug)]
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
    /// This function will return an error if building the expression fails.
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
    /// This function will return an error if building the expression fails.
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
    /// This function will return an error if building the expression fails.
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
    /// This function will return an error if operand conversion fails.
    pub fn with<O>(mut self, operand: O) -> Result<Self>
    where
        O: TryIntoOperand,
    {
        self.operands.push(operand.try_into_operand()?);
        Ok(self)
    }

    /// Builds the expression.
    #[must_use]
    pub fn build(self) -> Expression {
        Expression {
            operator: self.operator,
            operands: self.operands,
        }
    }
}
