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

//! Component builder.

use globset::{Glob, GlobSetBuilder};

use crate::id::matcher::{Matches, Result};

use super::Component;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Component builder.
///
/// This data type combines [`GlobSetBuilder`] with [`Matches`] to efficiently
/// build a [`Component`] that can match optional patterns extremely fast.
///
/// Before, we used `**` to denote optional patterns, leading to an explosion
/// of states in the resulting deterministic finite automaton (DFA) when many
/// optional patterns were used. Additionally, runtime performance was 1,000x
/// slower, which is unacceptable.
#[derive(Clone, Debug)]
pub struct Builder {
    /// Glob set builder.
    globset: GlobSetBuilder,
    /// Positions of patterns.
    mapping: Vec<usize>,
    /// Positions of empty patterns.
    matches: Matches,
    /// Total number of patterns.
    total: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Builder {
    /// Adds a pattern to the component.
    ///
    /// If the pattern is [`Some`], it is added to the [`GlobSetBuilder`]. If
    /// it's [`None`], the position is added to the match set as stored inside
    /// of [`Matches`]. This allows for extremely fast matching, reducing the
    /// number of states in the resulting deterministic finite automaton (DFA).
    pub fn add(&mut self, pattern: Option<Glob>) {
        if let Some(pattern) = pattern {
            self.globset.add(pattern);
            self.mapping.push(self.total);
        } else {
            self.matches.insert(self.total);
        }
        self.total += 1;
    }

    /// Builds the matcher component.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Glob`][] if the [`GlobSet`][] associated
    /// with a component cannot be successfully built.
    ///
    /// [`Error::Glob`]: crate::id::matcher::error::Error::Glob
    ///
    /// [`GlobSet`]: globset::GlobSet
    pub fn build(self) -> Result<Component> {
        Ok(Component {
            globset: self.globset.build()?,
            mapping: self.mapping.into_boxed_slice(),
            matches: self.matches,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Default for Builder {
    /// Creates a component builder.
    #[inline]
    fn default() -> Self {
        Builder {
            globset: GlobSetBuilder::new(),
            mapping: Vec::default(),
            matches: Matches::default(),
            total: 0,
        }
    }
}
