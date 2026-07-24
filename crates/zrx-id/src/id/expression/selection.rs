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

//! Selection.

use crate::id::matcher::matches::IntoIter;
use crate::id::selector::Selector;

use super::condition::Condition;
use super::Expression;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Selection.
///
/// Selections are used to extract a positive set of [`Term`][] instances from
/// an [`Expression`], yielding canonical [`Selector`] instances only. They are
/// essential to construct a provider-side [`Filter`][], as providers do not
/// perform [`Expression`] evaluation, only registration and matching.
///
/// [`Filter`]: crate::id::expression::Filter
/// [`Term`]: crate::id::expression::Term
pub struct Selection {
    /// Condition.
    condition: Condition,
    /// Iterator over terms.
    terms: IntoIter,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Expression {
    /// Returns a selection over all positive selectors.
    ///
    /// This method evaluates the terms in the condition's expression using a
    /// stack-based approach, where each instruction is processed in reverse
    /// order. The resulting match set contains the indices of all terms that
    /// are positive, i.e., those that are not negated by [`Operator::Not`][].
    ///
    /// [`Operator::Not`]: crate::id::expression::Operator::Not
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
    /// // Create iterator over selection
    /// for selector in expr.selection() {
    ///     println!("{selector:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn selection(&self) -> Selection {
        let condition = Condition::builder(self.clone()).optimize().build();
        Selection {
            terms: condition.selection().into_iter(),
            condition,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for Selection {
    type Item = Selector;

    /// Returns the next selector.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let terms = self.condition.terms();
        self.terms.next().map(|index| terms[index].clone().into())
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.terms.size_hint()
    }
}
