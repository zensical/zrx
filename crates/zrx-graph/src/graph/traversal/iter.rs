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

//! Iterator over a topological traversal.

use super::Traversal;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over a topological traversal.
///
/// This iterator consumes a [`Traversal`], emitting nodes in topological order.
/// It offers a simplified API for synchronous iteration if nodes don't need to
/// be deliberately completed, but can be considered done once the iterator
/// has emitted them.
#[derive(Debug)]
pub struct IntoIter {
    /// Traversal.
    traversal: Traversal,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for IntoIter {
    type Item = usize;

    /// Returns the next node.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let node = self.traversal.take()?;
        self.traversal.complete(node).expect("invariant");
        Some(node)
    }

    /// Returns the bounds of the traversal.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.traversal.len(), None)
    }
}

// ----------------------------------------------------------------------------

impl IntoIterator for Traversal {
    type Item = usize;
    type IntoIter = IntoIter;

    /// Creates an iterator over a topological traversal.
    ///
    /// This consumes the traversal and produces an iterator that automatically
    /// completes each node after emitting it, allowing for convenient use in
    /// for loops and iterator chains.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::Graph;
    ///
    /// // Create graph builder and add nodes
    /// let mut builder = Graph::builder();
    /// let a = builder.add_node("a");
    /// let b = builder.add_node("b");
    /// let c = builder.add_node("c");
    ///
    /// // Create edges between nodes
    /// builder.add_edge(a, b, 0)?;
    /// builder.add_edge(b, c, 0)?;
    ///
    /// // Create graph from builder
    /// let graph = builder.build();
    ///
    /// // Create iterator over topological traversal
    /// for node in graph.traverse([a]) {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter { traversal: self }
    }
}
