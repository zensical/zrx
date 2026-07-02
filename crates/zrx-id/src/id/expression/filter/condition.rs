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

//! Condition.

use crate::id::expression::{Operator, Term};
use crate::id::matcher::Matches;

mod builder;
mod group;
mod instruction;

use instruction::Instruction;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Condition.
///
/// Conditions are compiled and optimized expressions, converted into postfix
/// notation - also known as reverse polish notation (RPN) - for very efficient
/// and fast matching against a set of extracted terms. Conditions are an
/// internal construct and not exported via the public interface.
#[derive(Debug)]
pub struct Condition {
    /// Instructions in postfix notation.
    instructions: Box<[Instruction]>,
    /// Extracted terms.
    terms: Box<[Term]>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Condition {
    /// Returns whether the condition is satisfied by the given match set.
    ///
    /// This method evaluates the underlying instructions in postfix notation -
    /// also known as reverse polish notation (RPN) – against the provided set
    /// of matches. It leverages a bitwise stack to keep track of intermediate
    /// results, allowing for efficient evaluation of logical operators.
    ///
    /// Note that this method assumes that there're never more than 64 terms
    /// on the stack. Although this might theoretically happen, it practically
    /// never should, since conditions are going through optimization, which
    /// combines all term operands into a single instance of [`Matches`].
    #[must_use]
    pub fn satisfies(&self, matches: &Matches) -> bool {
        let mut stack = 0u64;

        // Evaluate instructions in postfix notation
        for instruction in &self.instructions {
            match instruction {
                // Compare terms against matches according to the semantics of
                // the containing operator, which differs between operators
                Instruction::Compare(operator, terms) => {
                    stack = (stack << 1)
                        | u64::from(match operator {
                            Operator::Any => matches.has_any(terms),
                            Operator::All => matches.has_all(terms),
                            Operator::Not => matches.has_any(terms),
                        });
                }
                // Combine prior results according to the operator semantics,
                // consuming the relevant number of bits from the stack
                Instruction::Combine(operator, arity) => {
                    let mask = (1 << arity) - 1;
                    let last = stack & mask;

                    // Remove the consumed bits from the stack, and push the
                    // result of the operation back onto the stack
                    stack >>= arity;
                    stack = (stack << 1)
                        | u64::from(match operator {
                            Operator::Any => last != 0,
                            Operator::All => last == mask,
                            Operator::Not => last == 0,
                        });
                }
            }
        }

        // At the end, there must be exactly one value left on the stack,
        // representing the result of the entire condition evaluation
        stack == 1
    }
}

#[allow(clippy::must_use_candidate)]
impl Condition {
    /// Returns the instructions in postfix notation.
    #[inline]
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// Returns the extracted terms.
    #[inline]
    pub fn terms(&self) -> &[Term] {
        &self.terms
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod satisfies {
        use crate::id::expression::filter::Condition;
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
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), true),
                (Matches::from_iter([1]), true),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 1, 2]), true),
                (Matches::from_iter([2]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_any_optimized() -> Result {
            let expr = Expression::any(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let condition = Condition::builder(expr).optimize().build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), true),
                (Matches::from_iter([1]), true),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 1, 2]), true),
                (Matches::from_iter([2]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_all() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(location = "**/*.md")?)?
                    .with(selector!(provider = "file")?)
            })?;
            let condition = Condition::builder(expr).build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), false),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 1, 2]), true),
                (Matches::from_iter([2]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_all_optimized() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(location = "**/*.md")?)?
                    .with(selector!(provider = "file")?)
            })?;
            let condition = Condition::builder(expr).optimize().build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), false),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 1, 2]), true),
                (Matches::from_iter([2]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_not() -> Result {
            let expr = Expression::not(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let condition = Condition::builder(expr).build();
            for (matches, check) in [
                (Matches::from_iter([]), true),
                (Matches::from_iter([0]), false),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), false),
                (Matches::from_iter([0, 1, 2]), false),
                (Matches::from_iter([2]), true),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_not_optimized() -> Result {
            let expr = Expression::not(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let condition = Condition::builder(expr).optimize().build();
            for (matches, check) in [
                (Matches::from_iter([]), true),
                (Matches::from_iter([0]), false),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), false),
                (Matches::from_iter([0, 1, 2]), false),
                (Matches::from_iter([2]), true),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_all_any() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(location = "**/*.jpg")?)?
                            .with(selector!(location = "**/*.png")?)
                    }))
            })?;
            let condition = Condition::builder(expr).build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), false),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 2]), true),
                (Matches::from_iter([0, 1, 3]), true),
                (Matches::from_iter([1, 2]), false),
                (Matches::from_iter([3]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_all_any_optimized() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(location = "**/*.jpg")?)?
                            .with(selector!(location = "**/*.png")?)
                    }))
            })?;
            let condition = Condition::builder(expr).optimize().build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), false),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 2]), true),
                (Matches::from_iter([0, 1, 3]), true),
                (Matches::from_iter([1, 2]), false),
                (Matches::from_iter([3]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_all_any_not() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(context = "docs")?)? // fmt
                            .with(Expression::not(|expr| {
                                expr.with(selector!(location = "**/*.jpg")?)?
                                    .with(selector!(location = "**/*.png")?)
                            }),
                        )
                    }))
            })?;
            let condition = Condition::builder(expr).build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), true),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 2]), false),
                (Matches::from_iter([0, 3]), false),
                (Matches::from_iter([0, 1, 2]), true),
                (Matches::from_iter([0, 1, 3]), true),
                (Matches::from_iter([0, 4]), true),
                (Matches::from_iter([4]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }

        #[test]
        fn handles_all_any_not_optimized() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(context = "docs")?)? // fmt
                            .with(Expression::not(|expr| {
                                expr.with(selector!(location = "**/*.jpg")?)?
                                    .with(selector!(location = "**/*.png")?)
                            }),
                        )
                    }))
            })?;
            let condition = Condition::builder(expr).optimize().build();
            for (matches, check) in [
                (Matches::from_iter([]), false),
                (Matches::from_iter([0]), true),
                (Matches::from_iter([1]), false),
                (Matches::from_iter([0, 1]), true),
                (Matches::from_iter([0, 2]), false),
                (Matches::from_iter([0, 3]), false),
                (Matches::from_iter([0, 1, 2]), true),
                (Matches::from_iter([0, 1, 3]), true),
                (Matches::from_iter([0, 4]), true),
                (Matches::from_iter([4]), false),
            ] {
                assert_eq!(condition.satisfies(&matches), check);
            }
            Ok(())
        }
    }
}
