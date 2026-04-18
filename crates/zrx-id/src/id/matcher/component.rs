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

//! Component.

use globset::GlobSet;
use std::path::Path;

use super::matches::Matches;

mod builder;

pub use builder::Builder;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Component.
#[derive(Clone, Debug, Default)]
pub struct Component {
    /// Glob set.
    globset: GlobSet,
    /// Positions of patterns.
    mapping: Box<[usize]>,
    /// Positions of empty patterns.
    matches: Matches,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Component {
    /// Returns a match set with indices of all matching patterns.
    ///
    /// Empty patterns are considered wildcards and thus equivalent to `**`,
    /// which means they're always included in the match set. Additionally, all
    /// patterns matching the given path are included, reconstructed from the
    /// internal mapping.
    pub fn matches<S>(&self, path: S) -> Matches
    where
        S: AsRef<Path>,
    {
        let mut matches = self.matches.clone();
        for index in self.globset.matches(path) {
            matches.add(self.mapping[index]);
        }

        // Return matches
        matches
    }
}
