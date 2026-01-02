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

//! Iterator over terms.

use super::operand::Operand;
use super::{Expression, Term};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over terms.
pub struct Terms<'a> {
    /// Stack for depth-first search.
    stack: Vec<&'a Expression>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Expression {
    /// Creates an iterator over the terms of an expression.
    #[inline]
    #[must_use]
    pub fn terms(&self) -> Terms<'_> {
        Terms { stack: Vec::from([self]) }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> Iterator for Terms<'a> {
    type Item = &'a Term;

    /// Returns the next term.
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(expr) = self.stack.pop() {
            for operand in expr.operands.iter().rev() {
                match operand {
                    Operand::Expression(expr) => self.stack.push(expr),
                    Operand::Term(term) => return Some(term),
                }
            }
        }

        // No more terms to return
        None
    }
}
