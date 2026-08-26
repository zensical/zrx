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

use crate::id::expression::filter;
use crate::id::sequence::Sequence;

use super::Filter;
use super::binding::Binding;
use super::condition::Condition;
use super::error::Result;
use super::layout::Layout;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Filter builder.
///
/// This data type uses a [`Slab`] to store conditions efficiently, which makes
/// it possible to keep indices stable when adding or removing sequences. It
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
    /// use zrx_id::sequence::Filter;
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
    /// use zrx_id::sequence::Filter;
    ///
    /// // Create filter
    /// let filter = Filter::default();
    ///
    /// // Create filter builder
    /// let mut builder = filter.into_builder();
    /// ```
    #[must_use]
    pub fn into_builder(self) -> Builder {
        Builder { conditions: self.conditions }
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Inserts a sequence into the filter and returns its index.
    ///
    /// This methods adds a [`Sequence`] to the filter builder, and returns
    /// the index of the inserted condition, which can be used to remove it.
    ///
    /// Note that the sequence is immediately converted into a [`Condition`]
    /// for performance reasons, which means it cannot be recovered. If we'd
    /// store sequences directly, removing or inserting new sequences into
    /// the filter would mandate recompilation of all sequences.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::sequence::Filter;
    /// use zrx_id::{Sequence, selector};
    ///
    /// // Create filter builder and insert sequence
    /// let mut builder = Filter::builder();
    /// builder.insert(Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn insert<T>(&mut self, sequence: T) -> usize
    where
        T: Into<Sequence>,
    {
        let condition = Condition::builder(sequence);
        self.conditions.insert(condition.optimize().build())
    }

    /// Removes a sequence from the filter.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::sequence::Filter;
    /// use zrx_id::{Sequence, selector};
    ///
    /// // Create filter builder and insert sequence
    /// let mut builder = Filter::builder();
    /// builder.insert(Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]));
    ///
    /// // Remove sequence
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
    /// Returns [`Error::Filter`][] if the underlying filter can't be built.
    ///
    /// [`Error::Filter`]: crate::id::sequence::filter::Error::Filter
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::sequence::Filter;
    /// use zrx_id::{Sequence, selector};
    ///
    /// // Create filter builder and insert sequence
    /// let mut builder = Filter::builder();
    /// builder.insert(Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]));
    ///
    /// // Create filter from builder
    /// let filter = builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Filter> {
        let mut filter = filter::Filter::builder();
        let mut layout = Layout::builder(self.conditions.len());

        // Build the reverse binding table, which is keyed by the inner filter
        // candidate index and maps back to the condition index and slot that
        // hit. This is used to reassemble candidate hits into the original
        // sequence conditions and validate ordering and gap constraints.
        let mut bindings = Vec::new();
        for (index, (key, condition)) in self.conditions.iter().enumerate() {
            let expressions = condition.expressions();
            let condition = u32::try_from(index)?;

            // Insert each expression of the current condition into the inner
            // filter. The returned index is the candidate identifier we later
            // receive back whenever that expression matches an identifier
            // during matching using the inner filter.
            let iter = expressions.iter().cloned();
            let filters = iter.map(|expression| filter.insert(expression));
            for (slot, filter) in filters.enumerate() {
                let slot = u8::try_from(slot)?;

                // Since the filter manages expressions within a slab, indices
                // are expected to be stable and dense, so we can safely resize
                // the bindings to accommodate the current filter index. This
                // ensures a compact and efficient representation.
                if bindings.len() <= filter {
                    bindings.resize(filter + 1, Binding { condition, slot: 0 });
                }

                // Associate the condition and slot with the current expression
                // index as returned by the filter for later resolution
                bindings[filter] = Binding { condition, slot };
            }

            // Associate the condition with its number of expressions, so we
            // can later resolve the layout of the condition when matching
            layout.add(key, expressions.len());
        }

        // Build and return filter
        Ok(Filter {
            filter: filter.build()?,
            conditions: self.conditions,
            bindings: bindings.into_boxed_slice(),
            layout: layout.build(),
        })
    }
}

#[allow(clippy::must_use_candidate)]
impl Builder {
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
