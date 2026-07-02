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

use crate::id::expression::filter;

mod binding;
mod builder;
mod candidates;
mod condition;
mod error;
mod layout;

use binding::Binding;
pub use builder::Builder;
pub use candidates::Candidates;
use condition::Condition;
pub use error::{Error, Result};
use layout::Layout;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Filter.
///
/// Filters efficiently match ordered identifier sequences against a set of
/// compiled sequence conditions. Each [`Filter`] manages an inner expression
/// filter that identifies matching constrained expressions first, and a
/// positional constraint set that validates whether those candidate matches
/// satisfy a compiled sequence condition. Thus, the inner expression filter
/// can be thought of as the first stage, eliminating non-matching constrained
/// expressions, while the positional constraint set is the second stage, which
/// checks whether the remaining candidate matches are actually satisfiable.
///
/// Each [`Filter`] manages three cooperating compiled structures:
///
/// - [`Filter::filter`]: Contains an inner expression filter that matches the
///   constrained sequence slots against individual identifiers.
///
/// - [`Filter::bindings`]: Contains one item per constrained slot in the inner
///   expression filter, which maps candidate indices back to conditions.
///
/// - [`Filter::layout`]: Contains a condition layout that records where each
///   condition's slots live used to validate ordering and gap constraints.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::sequence::Filter;
/// use zrx_id::{selector, Id, Sequence};
///
/// // Create filter builder and insert sequence
/// let mut builder = Filter::builder();
/// builder.insert(Sequence::suffix(
///     selector!(location = "**/*.md")?,
/// ));
///
/// // Create filter from builder
/// let filter = builder.build()?;
///
/// // Create identifiers and obtain candidate sequences
/// let id: Id = "zri:file:::docs:index.md:".parse()?;
/// for index in filter.candidates([&id])? {
///     println!("{index:?}");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct Filter {
    /// Expression filter.
    filter: filter::Filter,
    /// Expression filter bindings.
    bindings: Box<[Binding]>,
    /// Condition set, built from sequences.
    conditions: Slab<Condition>,
    /// Layout.
    layout: Layout,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

#[allow(clippy::must_use_candidate)]
impl Filter {
    /// Returns the number of sequences.
    #[inline]
    pub fn len(&self) -> usize {
        self.conditions.len()
    }

    /// Returns whether there are any sequences.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }
}
