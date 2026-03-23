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

use slab::Iter;

use super::condition::Condition;
use super::expression::Term;
use super::Filter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over terms.
pub struct Terms<'a> {
    /// Condition set, built from expressions.
    conditions: Iter<'a, Condition>,
    /// Current set of extracted terms.
    inner: &'a [Term],
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Filter {
    /// Creates an iterator over the terms.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Expression, Filter};
    ///
    /// // Create filter builder and insert expression
    /// let mut builder = Filter::builder();
    /// builder.insert(Expression::any(|expr| {
    ///     expr.with(selector!(location = "**/*.jpg")?)?
    ///         .with(selector!(location = "**/*.png")?)
    /// })?);
    ///
    /// // Create filter from builder
    /// let filter = builder.build()?;
    ///
    /// // Create iterator over terms
    /// for term in filter.terms() {
    ///     println!("{term:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn terms(&self) -> Terms<'_> {
        Terms {
            conditions: self.conditions.iter(),
            inner: &[],
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> Iterator for Terms<'a> {
    type Item = &'a Term;

    /// Returns the next term.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Check if we have terms left in the current condition
            if let Some(term) = self.inner.first() {
                self.inner = &self.inner[1..];
                return Some(term);
            }

            // Fetch next condition and extract its terms
            let (_, condition) = self.conditions.next()?;
            self.inner = condition.terms();
        }
    }
}
