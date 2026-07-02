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

//! Filter builder.

use slab::Slab;

use crate::id::expression::{Expression, Operator, Term};
use crate::id::matcher::Matcher;

use super::condition::Condition;
use super::error::Result;
use super::Filter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Filter builder.
///
/// This data type uses a [`Slab`] to store conditions efficiently, which makes
/// it possible to keep indices stable when adding or removing expressions. It
/// allows users to modify a [`Filter`] dynamically, and rebuild it on-the-fly
/// after all modifications were made.
#[derive(Debug, Default)]
pub struct Builder {
    /// Condition set.
    conditions: Slab<Condition>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Filter {
    /// Creates a filter builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::expression::Filter;
    ///
    /// // Create filter builder
    /// let mut builder = Filter::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder() -> Builder {
        Builder::default()
    }

    /// Creates a filter builder from the filter.
    ///
    /// This method allows to modify an existing [`Filter`] by converting it
    /// back into a filter builder to insert or remove expressions.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::expression::Filter;
    ///
    /// // Create filter
    /// let filter = Filter::default();
    ///
    /// // Create filter builder
    /// let mut builder = filter.into_builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn into_builder(self) -> Builder {
        Builder { conditions: self.conditions }
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Inserts an expression into the filter and returns its index.
    ///
    /// This method adds an [`Expression`] to the filter builder, and returns
    /// the index of the inserted condition, which can be used to remove it.
    ///
    /// Note that the expression is immediately converted into a [`Condition`]
    /// for performance reasons, which means it cannot be recovered. If we'd
    /// store expressions directly, removing or inserting new expressions into
    /// the filter would mandate recompilation of all expressions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::expression::Filter;
    /// use zrx_id::{selector, Expression};
    ///
    /// // Create filter builder and insert expression
    /// let mut builder = Filter::builder();
    /// builder.insert(Expression::any(|expr| {
    ///     expr.with(selector!(location = "**/*.jpg")?)?
    ///         .with(selector!(location = "**/*.png")?)
    /// })?);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn insert<T>(&mut self, expr: T) -> usize
    where
        T: Into<Expression>,
    {
        let builder = Condition::builder(expr);
        self.conditions.insert(builder.optimize().build())
    }

    /// Removes an expression from the filter.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::expression::Filter;
    /// use zrx_id::{selector, Expression};
    ///
    /// // Create filter builder and insert expression
    /// let mut builder = Filter::builder();
    /// builder.insert(Expression::any(|expr| {
    ///     expr.with(selector!(location = "**/*.jpg")?)?
    ///         .with(selector!(location = "**/*.png")?)
    /// })?);
    ///
    /// // Remove expression
    /// builder.remove(0);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn remove(&mut self, index: usize) {
        self.conditions.remove(index);
    }

    /// Builds the filter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Matcher`][] if the underlying matcher can't be built.
    ///
    /// [`Error::Matcher`]: crate::id::expression::filter::Error::Matcher
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::expression::Filter;
    /// use zrx_id::{selector, Expression};
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Filter> {
        let mut builder = Matcher::builder();

        // Initialize term mappings and negations
        let mut mapping = Vec::with_capacity(self.conditions.len());
        let mut negations = Vec::new();

        // Add all terms of each condition to the mapping and matcher
        for (index, condition) in &self.conditions {
            for term in condition.terms() {
                mapping.push(u32::try_from(index)?);
                match term {
                    Term::Id(id) => builder.add(id)?,
                    Term::Selector(selector) => builder.add(selector)?,
                };
            }

            // If the current condition contains a negation, we add its index
            // to the list of negations, so it's always checked when matching
            let mut iter = condition.instructions().iter();
            if iter.any(|instruction| instruction.operator() == Operator::Not) {
                negations.push(u32::try_from(index)?);
            }
        }

        // Build matcher and return filter
        Ok(Filter {
            conditions: self.conditions,
            negations,
            mapping,
            matcher: builder.build()?,
        })
    }
}

#[allow(clippy::must_use_candidate)]
impl Builder {
    /// Returns the number of expressions.
    #[inline]
    pub fn len(&self) -> usize {
        self.conditions.len()
    }

    /// Returns whether there are any expressions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }
}
