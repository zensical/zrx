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

//! Condition group.

use crate::id::filter::expression::Operator;
use crate::id::matcher::Matches;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Condition group.
#[derive(Debug, PartialEq, Eq)]
pub enum Group {
    /// Group with operator.
    Operator(Operator, Vec<Group>),
    /// Group with terms.
    Terms(Matches),
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Group {
    /// Transforms a condition group in post-order with the given function.
    ///
    /// This method traverses the condition group in post-order, and maps each
    /// group to a new group by applying the provided function. Operators are
    /// processed after all of their operands have been transformed, so that
    /// groups can be rewritten from the bottom up, e.g., for optimization.
    ///
    /// This implementation deliberately uses a stack over recursion, because
    /// the Rust compiler will run into stack overflows for deep recursion.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn map<F>(self, mut f: F) -> Self
    where
        F: FnMut(Group) -> Group,
    {
        // Initialize stack for input and transformed groups
        let mut input = Vec::from([(self, false, 0)]);
        let mut stack = Vec::new();

        // Process input stack until empty, transforming groups in post-order,
        // moving them onto the stack of transformed groups
        while let Some((group, visited, arity)) = input.pop() {
            match group {
                // 1st visit: push operator onto stack and mark it as visited,
                // then push its operands in reverse onto the stack
                Group::Operator(operator, operands) if !visited => {
                    let group = Group::Operator(operator, Vec::new());
                    input.push((group, true, operands.len()));

                    // Push operands in reverse onto stack
                    for group in operands.into_iter().rev() {
                        input.push((group, false, 0));
                    }
                }
                // 2nd visit: pop exactly as many operands as the operator
                // expects from the stack and transform it with the function
                Group::Operator(operator, ..) => {
                    let operands = stack.split_off(stack.len() - arity);
                    stack.push(f(Group::Operator(operator, operands)));
                }
                // Push terms onto stack for processing
                Group::Terms(_) if !visited => {
                    stack.push(f(group));
                }
                // This should never happen
                Group::Terms(_) => unreachable!(),
            }
        }

        // Return the top-most processed group
        stack.pop().expect("invariant")
    }
}
