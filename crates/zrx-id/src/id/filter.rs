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

//! Filter.

use slab::Slab;

use super::matcher::Matcher;

mod builder;
mod condition;
mod error;
pub mod expression;
mod iter;

use condition::Condition;
pub use error::{Error, Result};
pub use expression::{Expression, IntoExpression};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Filter.
///
/// Filters allow to efficiently match identifiers against a set of expressions,
/// which are composed of conditions and logical operators, and compiled into a
/// set of optimized instructions for matching. This allows to evaluate complex
/// logical expressions against identifiers extremely fast.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::{selector, Expression, Filter, Id};
///
/// // Create filter builder and insert expression
/// let mut builder = Filter::builder();
/// builder.insert(Expression::any(|expr| {
///     expr.with(selector!(location = "**/*.md")?)?
///         .with(selector!(provider = "file")?)
/// })?);
///
/// // Create filter from builder
/// let filter = builder.build()?;
///
/// // Create identifier and match expressions
/// let id: Id = "zri:file:::docs:index.md:".parse()?;
/// for index in filter.matches(&id)? {
///     println!("{index:?}");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct Filter {
    /// Condition set, built from expressions.
    conditions: Slab<Condition>,
    /// Condition indices of negations.
    negations: Vec<u32>,
    /// Condition term mappings.
    mapping: Vec<u32>,
    /// Extracted terms.
    matcher: Matcher,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

#[allow(clippy::must_use_candidate)]
impl Filter {
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
