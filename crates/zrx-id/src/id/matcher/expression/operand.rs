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

//! Operand.

use std::fmt;

use super::Expression;

mod convert;
mod error;
mod term;

pub use convert::TryIntoOperand;
pub use error::{Error, Result};
pub use term::Term;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operator {
    /// Logical `AND`.
    Any,
    /// Logical `OR`.
    All,
    /// Logical `NOT`.
    Not,
}

/// Operand.
#[derive(Clone)]
pub enum Operand {
    /// Expression.
    Expression(Expression),
    /// Term.
    Term(Term),
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<Expression> for Operand {
    /// Creates an operand from the given expression.
    #[inline]
    fn from(expr: Expression) -> Self {
        Self::Expression(expr)
    }
}

impl<T> From<T> for Operand
where
    T: Into<Term>,
{
    /// Creates an operand from the given term.
    #[inline]
    fn from(term: T) -> Self {
        Self::Term(term.into())
    }
}

// ----------------------------------------------------------------------------

impl fmt::Debug for Operand {
    /// Formats the operand for debugging.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Expression(expr) => expr.fmt(f),
            Self::Term(term) => term.fmt(f),
        }
    }
}
