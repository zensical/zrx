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

//! Condition builder.

use crate::id::filter::expression::{Operand, Operator, Term};
use crate::id::filter::Expression;
use crate::id::matcher::Matches;

use super::group::Group;
use super::{Condition, Instruction};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Condition builder.
#[derive(Debug)]
pub struct Builder {
    /// Condition group.
    group: Group,
    /// Extracted terms.
    terms: Vec<Term>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Condition {
    /// Creates a condition builder from an expression.
    ///
    /// This method compiles an [`Expression`] into a [`Builder`], transforming
    /// it to a [`Group`] - a tree of logical operators and term indices – and
    /// extracting all terms along the way. The resulting builder can then be
    /// transformed into a [`Condition`], which is used in a [`Filter`][].
    ///
    /// [`Filter`]: crate::id::filter::Filter
    #[inline]
    #[must_use]
    pub fn builder<T>(expr: T) -> Builder
    where
        T: Into<Expression>,
    {
        let mut terms = Vec::new();
        Builder {
            group: compile(expr.into(), &mut terms),
            terms,
        }
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Builds the condition.
    pub fn build(self) -> Condition {
        let mut input = Vec::from([(self.group, None)]);
        let mut stack = Vec::new();

        // Process stack with condition groups until empty, transforming groups
        // into instructions in reverse postfix notation. Note that we have to
        // keep track of the operator for term groups as well, as we need it
        // for the compare instructions we're about to create.
        while let Some((group, operator)) = input.pop() {
            match group {
                // Emit combine instruction, and put all operands onto the
                // stack with the same operator for term processing
                Group::Operator(operator, operands) => {
                    stack.push(Instruction::Combine(operator, operands.len()));
                    for operand in operands {
                        input.push((operand, Some(operator)));
                    }
                }
                // Emit compare instruction to compare terms against matches,
                // and fall back to the logical `OR` operator if none is given,
                // which is the expected behavior when the condition consists
                // of a single set of terms without an operator.
                Group::Terms(terms) => {
                    stack.push(Instruction::Compare(
                        operator.unwrap_or(Operator::Any),
                        terms,
                    ));
                }
            }
        }

        // Reverse instructions for reverse polish notation - we could also
        // use a post-order traversal, but that's more complex to manage
        stack.reverse();

        // Return condition with instructions and extracted terms
        Condition {
            instructions: stack.into_boxed_slice(),
            terms: self.terms.into_boxed_slice(),
        }
    }

    /// Optimizes the condition builder.
    ///
    /// It's important to optimize the condition before building it, as this
    /// will significantly reduce the number of instructions.
    #[inline]
    #[must_use]
    pub fn optimize(self) -> Self {
        Self {
            group: optimize(self.group),
            terms: self.terms,
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Recursively compiles an expression into a condition group, and collects
/// all terms along the way. Note that the terms are stored in post-order, so
/// the returned condition group can reference them by index. This is essential
/// for efficient storage and evaluation.
fn compile(expr: Expression, terms: &mut Vec<Term>) -> Group {
    let operator = expr.operator();

    // Extract terms and compile operands recursively
    let iter = expr.into_iter().map(|operand| match operand {
        Operand::Expression(expr) => compile(expr, terms),
        Operand::Term(term) => {
            let index = terms.len();
            terms.push(term);
            Group::Terms(Matches::from_iter([index]))
        }
    });

    // Collect into operator group
    Group::Operator(operator, iter.collect())
}

// ----------------------------------------------------------------------------

/// Optimizes a condition group, trying to combine operators and terms without
/// changing the semantics - note that this happens in several stages.
fn optimize(group: Group) -> Group {
    let group = group.map(optimize_operators);
    let group = group.map(optimize_terms);

    // Try to hoist top-level logical `OR` operator
    optimize_hoistable(group)
}

/// Optimizes nested operators through hoisting if and only if they're of the
/// same type - note that this does not apply to the logical `NOT` operator
fn optimize_operators(group: Group) -> Group {
    let (operator, operands) = match group {
        Group::Operator(Operator::Not, ..) | Group::Terms(..) => return group,
        Group::Operator(operator, operands) => (operator, operands),
    };

    // Hoist inner operators of the same type
    let iter = operands.into_iter().flat_map(|operand| match operand {
        Group::Operator(inner, operands) if inner == operator => operands,
        other => Vec::from([other]),
    });

    // Collect into operator group
    Group::Operator(operator, iter.collect())
}

/// Optimizes adjacent terms that are operands of the current group, combining
/// them into a single match set for efficient and optimized parallel matching.
fn optimize_terms(group: Group) -> Group {
    let Group::Operator(operator, operands) = group else {
        return group;
    };

    // Combine adjacent term groups in reverse
    let mut optimized = Vec::new();
    let mut opt: Option<Matches> = None;
    for operand in operands.into_iter().rev() {
        match operand {
            other @ Group::Operator(..) => optimized.push(other),
            Group::Terms(terms) => {
                if let Some(ref mut matches) = opt {
                    matches.union(&terms);
                } else {
                    opt = Some(terms);
                }
            }
        }
    }

    // Add combined terms if any
    if let Some(matches) = opt {
        optimized.push(Group::Terms(matches));
    }

    // Reverse the optimized terms and return operator group
    optimized.reverse();
    Group::Operator(operator, optimized)
}

/// Optimizes hoistable top-level groups, e.g., a logical `OR` operator with a
/// single operand. This is only applied at the top-level of the condition.
fn optimize_hoistable(group: Group) -> Group {
    match group {
        Group::Operator(Operator::Any, mut operands) if operands.len() == 1 => {
            operands.pop().expect("invariant")
        }
        other => other,
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod builder {
        use crate::id::filter::condition::group::Group;
        use crate::id::filter::expression::{Operator, Result, Term};
        use crate::id::filter::{Condition, Expression};
        use crate::selector;

        #[test]
        fn handles_expression() -> Result {
            let expr = Expression::any(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let builder = Condition::builder(expr);
            assert_eq!(
                builder.terms,
                [
                    Term::from(selector!(location = "**/*.jpg")?),
                    Term::from(selector!(location = "**/*.png")?),
                ]
            );
            match builder.group {
                Group::Terms(_) => panic!("expected operator"),
                Group::Operator(operator, operands) => {
                    assert_eq!(operator, Operator::Any);
                    assert_eq!(operands.len(), 2);
                }
            }
            Ok(())
        }

        #[test]
        fn handles_nested_expression() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(location = "**/*.md")?)? // fmt
                    .with(Expression::not(|expr| {
                        expr.with(selector!(provider = "file")?)
                    }))
            })?;
            let builder = Condition::builder(expr);
            assert_eq!(
                builder.terms,
                [
                    Term::from(selector!(location = "**/*.md")?),
                    Term::from(selector!(provider = "file")?),
                ]
            );
            match builder.group {
                Group::Terms(_) => panic!("expected operator"),
                Group::Operator(operator, operands) => {
                    assert_eq!(operator, Operator::All);
                    assert_eq!(operands.len(), 2);
                    match &operands[1] {
                        Group::Terms(_) => panic!("expected operator"),
                        Group::Operator(operator, operands) => {
                            assert_eq!(*operator, Operator::Not);
                            assert_eq!(operands.len(), 1);
                        }
                    }
                }
            }
            Ok(())
        }
    }

    mod optimize {
        use crate::id::filter::condition::group::Group;
        use crate::id::filter::expression::{Operator, Result};
        use crate::id::filter::{Condition, Expression};
        use crate::id::matcher::Matches;
        use crate::selector;

        #[test]
        fn handles_any() -> Result {
            let expr = Expression::any(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group, // fmt
                Group::Terms(Matches::from_iter([0, 1]))
            );
            Ok(())
        }

        #[test]
        fn handles_any_any() -> Result {
            let expr = Expression::any(|expr| {
                expr.with(Expression::any(|expr| {
                    expr.with(selector!(location = "**/*.jpg")?)?
                        .with(selector!(location = "**/*.png")?)
                })?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group, // fmt
                Group::Terms(Matches::from_iter([0, 1]))
            );
            Ok(())
        }

        #[test]
        fn handles_any_any_mixed() -> Result {
            let expr = Expression::any(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)? // fmt
                    .with(Expression::any(|expr| {
                        expr.with(selector!(location = "**/*.png")?)
                    })?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group, // fmt
                Group::Terms(Matches::from_iter([0, 1]))
            );
            Ok(())
        }

        #[test]
        fn handles_all_all() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(Expression::all(|expr| {
                    expr.with(selector!(location = "**/*.jpg")?)?
                        .with(selector!(location = "**/*.png")?)
                })?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group,
                Group::Operator(
                    Operator::All,
                    vec![Group::Terms(Matches::from_iter([0, 1]))]
                )
            );
            Ok(())
        }

        #[test]
        fn handles_all_all_mixed() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)? // fmt
                    .with(Expression::all(|expr| {
                        expr.with(selector!(location = "**/*.png")?)
                    })?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group,
                Group::Operator(
                    Operator::All,
                    vec![Group::Terms(Matches::from_iter([0, 1]))]
                )
            );
            Ok(())
        }

        #[test]
        fn handles_not_not() -> Result {
            let expr = Expression::not(|expr| {
                expr.with(Expression::not(|expr| {
                    expr.with(selector!(location = "**/*.jpg")?)?
                        .with(selector!(location = "**/*.png")?)
                })?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group,
                Group::Operator(
                    Operator::Not,
                    vec![Group::Operator(
                        Operator::Not,
                        vec![Group::Terms(Matches::from_iter([0, 1]))]
                    )]
                )
            );
            Ok(())
        }

        #[test]
        fn handles_not_not_mixed() -> Result {
            let expr = Expression::not(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)? // fmt
                    .with(Expression::not(|expr| {
                        expr.with(selector!(location = "**/*.png")?)
                    })?)
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group,
                Group::Operator(
                    Operator::Not,
                    vec![
                        Group::Terms(Matches::from_iter([0])),
                        Group::Operator(
                            Operator::Not,
                            vec![Group::Terms(Matches::from_iter([1]))]
                        )
                    ]
                )
            );
            Ok(())
        }

        #[test]
        fn handles_all_any() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(Expression::any(|expr| {
                    expr.with(selector!(location = "**/*.jpg")?)?
                        .with(selector!(location = "**/*.png")?)
                }))
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group,
                Group::Operator(
                    Operator::All,
                    vec![Group::Operator(
                        Operator::Any,
                        vec![Group::Terms(Matches::from_iter([0, 1]))]
                    )]
                )
            );
            Ok(())
        }

        #[test]
        fn handles_all_any_mixed() -> Result {
            let expr = Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(location = "**/*.jpg")?)?
                            .with(selector!(location = "**/*.png")?)
                    }))
            })?;
            let builder = Condition::builder(expr).optimize();
            assert_eq!(
                builder.group,
                Group::Operator(
                    Operator::All,
                    vec![
                        Group::Terms(Matches::from_iter([0])),
                        Group::Operator(
                            Operator::Any,
                            vec![Group::Terms(Matches::from_iter([1, 2]))]
                        )
                    ]
                )
            );
            Ok(())
        }
    }
}
