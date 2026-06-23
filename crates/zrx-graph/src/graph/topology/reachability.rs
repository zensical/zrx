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

//! Reachability.

use super::adjacency::Adjacency;

mod distance;

use distance::Distance;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Direct reachability.
#[derive(Debug)]
pub struct Direct;

// ----------------------------------------------------------------------------

/// Transitive reachability.
#[derive(Debug)]
pub struct Transitive {
    /// Distance matrix.
    distance: Distance,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Transitive {
    /// Creates transitive reachability from the given adjacency list.
    #[must_use]
    pub fn new(adj: &Adjacency) -> Self {
        Self { distance: Distance::new(adj) }
    }

    /// Returns whether the target node is reachable from the source node.
    #[inline]
    #[must_use]
    pub fn is_reachable(&self, source: usize, target: usize) -> bool {
        self.distance.is_reachable(source, target)
    }
}
