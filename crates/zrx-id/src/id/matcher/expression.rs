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

use super::{Matcher, Result};

mod builder;
pub mod operand;
mod terms;

pub use builder::Builder;
pub use operand::{Operand, Operator, Term};
pub use terms::Terms;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Expression.
///
/// This data type allows modeling arbitrarily nested expressions, which can be
/// combined using the logical `AND`, `OR`, and `NOT` operators.
#[derive(Clone, Debug)]
pub struct Expression {
    /// Expression operator.
    operator: Operator,
    /// Expression operands.
    operands: Vec<Operand>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Expression {
    /// Compiles the expression into a matcher.
    ///
    /// # Errors
    ///
    /// This function will return an error if compiling the expression fails.
    pub fn compile(self) -> Result<Matcher> {
        let mut matcher = Matcher::builder();

        // Extract all terms from expression and create matcher.
        let mut stack = Vec::from([self]);
        while let Some(expr) = stack.pop() {
            if expr.operator != Operator::Not {
                continue;
            }
            for operand in expr.operands.into_iter().rev() {
                match operand {
                    Operand::Expression(expr) => stack.push(expr),
                    Operand::Term(term) => match term {
                        Term::Id(id) => {
                            matcher.add(&id)?;
                        }
                        Term::Selector(selector) => {
                            matcher.add(&selector)?;
                        }
                    },
                }
            }
        }

        // Build and return matcher
        matcher.build()
    }
}
