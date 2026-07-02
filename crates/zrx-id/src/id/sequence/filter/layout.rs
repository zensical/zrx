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

//! Layout.

use crate::id::matcher::Matches;

use super::Condition;

mod builder;
mod item;
mod positions;

pub use item::Item;
pub use positions::Positions;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Layout.
///
/// Layouts are used to map conditions and their expression slots to a flat
/// list of items, which can be used for efficient filtering of candidates.
#[derive(Debug, Default)]
pub struct Layout {
    /// Layout items.
    pub items: Box<[Item]>,
    /// Layout slots.
    pub slots: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Layout {
    /// Creates initial matching conditions.
    ///
    /// Seeding the initial set of matching conditions is necessary to ensure
    /// that conditions with empty slot ranges are evaluated against the current
    /// identifier sequence. Otherwise, they would be skipped entirely, even
    /// though they can still match based on their sequence structure alone.
    ///
    /// The sequence filter matches in two stages:
    ///
    /// 1. The inner expression filter reports which constrained slots matched
    ///    the identifiers we are checking.
    ///
    /// 2. The outer sequence filter reassembles those hits per condition and
    ///    checks whether gap and ordering constraints are satisfied.
    ///
    /// By adding them up front, we ensure that second 2nd stage evaluates
    /// those conditions against the given sequence of expressions.
    #[must_use]
    pub fn matches(&self) -> Matches {
        let mut matches = Matches::new();
        for (index, item) in self.items.iter().enumerate() {
            if item.range.is_empty() {
                matches.add(index);
            }
        }
        matches
    }
}
