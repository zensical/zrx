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

//! Iterator over candidates.

use slab::Slab;

use crate::id::TryToId;
use crate::id::matcher::matches::IntoIter;

use super::Filter;
use super::condition::Condition;
use super::error::Result;
use super::layout::{Item, Positions};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over candidates.
pub struct Candidates<'a> {
    /// Iterator over matches.
    matches: IntoIter,
    /// Condition set, built from sequences.
    conditions: &'a Slab<Condition>,
    /// Layout position set.
    positions: Positions,
    /// Layout items.
    items: &'a [Item],
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Filter {
    /// Returns the indices of sequences that match the identifiers.
    ///
    /// This method compares sequences part of the filter against the given set
    /// of ordered identifiers and returns an iterator over the indices of the
    /// sequences that match. The order of the returned indices corresponds to
    /// the order in which the sequences were added to the filter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`][] if any of the identifiers is invalid.
    ///
    /// [`Error::Id`]: crate::id::sequence::filter::Error::Id
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::sequence::Filter;
    /// use zrx_id::{Id, Sequence, selector};
    ///
    /// // Create filter builder and insert sequence
    /// let mut builder = Filter::builder();
    /// builder.insert(Sequence::suffix(selector!(location = "**/*.md")?));
    ///
    /// // Create filter from builder
    /// let filter = builder.build()?;
    ///
    /// // Create identifier and obtain candidate sequences
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// for index in filter.candidates([&id])? {
    ///     println!("{index:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn candidates<T>(&self, ids: T) -> Result<Candidates<'_>>
    where
        T: IntoIterator,
        T::Item: TryToId,
    {
        let mut matches = self.layout.matches();
        let mut positions = self.layout.positions();

        // Iterate over the input identifiers and query the inner expression
        // filter, mapping the resulting candidate slot indices to conditions
        for id in ids {
            let id = id.try_to_id()?;
            positions.advance();

            // Query the inner expression filter for expressions that match the
            // current identifier and map the resulting candidate slot indices
            // to conditions, marking the corresponding positions in the layout
            // position set.
            let mut current = None;
            for index in self.filter.candidates(id.as_ref())? {
                let binding = &self.bindings[index];

                // Retrieve binding condition and slot index
                let condition = binding.condition as usize;
                let slot = usize::from(binding.slot);

                // Add condition to matches if it hasn't been added yet
                if current != Some(condition) {
                    matches.add(condition);
                    current = Some(condition);
                }

                // Mark the position in the position set
                let item = &self.layout.items[condition];
                positions.mark(item.range.start + slot);
            }
        }

        // Return iterator over candidates
        Ok(Candidates {
            matches: matches.into_iter(),
            conditions: &self.conditions,
            positions,
            items: &self.layout.items,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for Candidates<'_> {
    type Item = usize;

    /// Returns the next candidate.
    fn next(&mut self) -> Option<Self::Item> {
        for index in self.matches.by_ref() {
            let item = &self.items[index];

            // Check whether the current candidate condition is satisfied
            let condition = &self.conditions[item.index];
            if self.positions.satisfies(&item.range, condition) {
                return Some(item.index);
            }
        }

        // No more candidates to return
        None
    }
}
