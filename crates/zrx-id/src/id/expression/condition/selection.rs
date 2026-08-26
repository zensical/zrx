// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Condition selection.

use crate::id::expression::Operator;
use crate::id::matcher::Matches;

use super::Condition;
use super::instruction::Instruction;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Condition {
    /// Returns a match set with indices of all positive terms.
    ///
    /// This method evaluates the terms in the condition's expression using a
    /// stack-based approach, where each instruction is processed in reverse
    /// order. The resulting match set contains the indices of all terms that
    /// are positive, i.e., those that are not negated by [`Operator::Not`].
    #[must_use]
    pub fn selection(&self) -> Matches {
        let mut stack = 1u64;

        // Evaluate instructions in reverse postfix notation
        let mut matches = Matches::new();
        for instruction in self.instructions.iter().rev() {
            match instruction {
                // Compare the terms against the match set, and if the current
                // polarity is positive, add them to the selection
                Instruction::Compare(_, terms) => {
                    if stack & 1 != 0 {
                        matches.union(terms);
                    }
                    stack >>= 1;
                }
                // Combine prior results using the specified operator and
                // arity, and update the stack with the new result
                Instruction::Combine(operator, arity) => {
                    let mask = (1 << arity) - 1;
                    let last = stack & 1 != 0;

                    // Shift the stack to remove the prior results, and push
                    // the new result with the updated polarity
                    stack >>= 1;
                    stack = (stack << arity)
                        | (u64::from(match operator {
                            Operator::Not => !last,
                            Operator::Any | Operator::All => last,
                        }) * mask);
                }
            }
        }

        // Return match set
        matches
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod selection {
        use crate::id::expression::condition::Condition;
        use crate::id::expression::{Expression, Result};
        use crate::id::matcher::Matches;
        use crate::selector;

        #[test]
        fn handles_any() -> Result {
            let expr = Expression::any(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let condition = Condition::builder(expr).build();
            assert_eq!(
                condition.selection(), // fmt
                Matches::from_iter([0, 1])
            );
            Ok(())
        }

        #[test]
        fn handles_all() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(location = "**/*.md")?)?
                    .with(selector!(provider = "file")?)
            })?;
            let condition = Condition::builder(expr).build();
            assert_eq!(
                condition.selection(), // fmt
                Matches::from_iter([0, 1])
            );
            Ok(())
        }

        #[test]
        fn handles_not() -> Result {
            let expr = Expression::not(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let condition = Condition::builder(expr).build();
            assert_eq!(
                condition.selection(), // fmt
                Matches::from_iter([])
            );
            Ok(())
        }

        #[test]
        fn handles_all_any() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expression| {
                        expression
                            .with(selector!(location = "**/*.jpg")?)?
                            .with(selector!(location = "**/*.png")?)
                    }))
            })?;
            let condition = Condition::builder(expr).build();
            assert_eq!(
                condition.selection(), // fmt
                Matches::from_iter([0, 1, 2])
            );
            Ok(())
        }

        #[test]
        fn handles_all_any_not() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(context = "docs")?)?.with(
                            Expression::not(|expr| {
                                expr.with(selector!(location = "**/*.jpg")?)?
                                    .with(selector!(location = "**/*.png")?)
                            }),
                        )
                    }))
            })?;
            let condition = Condition::builder(expr).build();
            assert_eq!(
                condition.selection(), //fmt
                Matches::from_iter([0, 1])
            );
            Ok(())
        }
    }
}
