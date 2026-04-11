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

//! Frontier set.

use zrx_graph::traversal::Result;
use zrx_graph::Traversal;

use crate::scheduler::signal::{Id, Scope};

mod set;

pub use set::Frontiers;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Frontier.
#[derive(Debug)]
pub struct Frontier<I> {
    /// Frontier scope.
    scope: Scope<I>,
    /// Traversal.
    traversal: Traversal,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Frontier<I> {
    /// Creates a frontier.
    #[must_use]
    pub fn new(scope: Scope<I>, traversal: Traversal) -> Self {
        Self { scope, traversal }
    }

    /// Returns the next visitable node.
    #[inline]
    pub fn take(&mut self) -> Option<usize> {
        self.traversal.take()
    }

    /// Marks the given node as visited.
    #[inline]
    pub fn complete(&mut self, node: usize) -> Result {
        self.traversal.complete(node)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Frontier<I>
where
    I: Id,
{
    /// Returns a reference to the scope.
    #[inline]
    pub fn scope(&self) -> &Scope<I> {
        &self.scope
    }

    // /// Returns the number of visitable nodes.
    // #[inline]
    // pub fn len(&self) -> usize {
    //     self.traversal.len()
    // }

    /// Returns whether there are any visitable nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.traversal.is_empty()
    }
}
