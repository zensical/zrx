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

//! Iterator over descendants of a node.

use ahash::HashSet;

use crate::graph::topology::Adjacency;
use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over descendants of a node.
pub struct Descendants<'a> {
    /// Outgoing edges.
    outgoing: &'a Adjacency,
    /// Stack for depth-first search.
    stack: Vec<usize>,
    /// Set of visited nodes.
    visited: HashSet<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates an iterator over the descendants of the given node.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist, as this indicates that there's a bug
    /// in the code that creates or uses the iterator. While the [`Builder`][]
    /// is designed to be fallible to ensure the structure is valid, methods
    /// that operate on [`Graph`] panic on violated invariants.
    ///
    /// [`Builder`]: crate::graph::Builder
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
    /// // Create iterator over descendants
    /// for node in graph.descendants(a) {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn descendants(&self, node: usize) -> Descendants<'_> {
        let mut iter = Descendants {
            outgoing: self.topology.outgoing(),
            stack: Vec::from([node]),
            visited: HashSet::default(),
        };

        // Skip the initial node itself - it's simpler to just skip the initial
        // node, so we can keep the iterator implementation plain and simple
        iter.next();
        iter
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for Descendants<'_> {
    type Item = usize;

    /// Returns the next descendant.
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
    /// // Create iterator over descendants
    /// let mut descendants = graph.descendants(a);
    /// while let Some(node) = descendants.next() {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        // Perform a depth-first search to find all descendants of a node, by
        // exploring them iteratively, not including the node itself
        let node = self.stack.pop()?;
        for &descendant in self.outgoing[node].iter().rev() {
            // If we haven't visited this descendant yet, we put it on the
            // stack after marking it as visited and return it immediately
            if self.visited.insert(descendant) {
                self.stack.push(descendant);
            }
        }

        // Return the next descendant
        Some(node)
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod descendants {
        use crate::graph;

        #[test]
        fn handles_graph() {
            let graph = graph! {
                "a" => "b", "a" => "c",
                "b" => "d", "b" => "e",
                "c" => "f",
                "d" => "g",
                "e" => "g", "e" => "h",
                "f" => "h",
                "g" => "i",
                "h" => "i",
            };
            for (node, descendants) in [
                (0, vec![1, 3, 6, 8, 4, 7, 2, 5]),
                (1, vec![3, 6, 8, 4, 7]),
                (2, vec![5, 7, 8]),
                (3, vec![6, 8]),
                (4, vec![6, 8, 7]),
                (5, vec![7, 8]),
                (6, vec![8]),
                (7, vec![8]),
                (8, vec![]),
            ] {
                assert_eq!(
                    graph.descendants(node).collect::<Vec<_>>(),
                    descendants
                );
            }
        }

        #[test]
        fn handles_multi_graph() {
            let graph = graph! {
                "a" => "b", "a" => "c", "a" => "c",
                "b" => "d", "b" => "e",
                "c" => "f",
                "d" => "g",
                "e" => "g", "e" => "h",
                "f" => "h",
                "g" => "i",
                "h" => "i",
            };
            for (node, descendants) in [
                (0, vec![1, 3, 6, 8, 4, 7, 2, 5]),
                (1, vec![3, 6, 8, 4, 7]),
                (2, vec![5, 7, 8]),
                (3, vec![6, 8]),
                (4, vec![6, 8, 7]),
                (5, vec![7, 8]),
                (6, vec![8]),
                (7, vec![8]),
                (8, vec![]),
            ] {
                assert_eq!(
                    graph.descendants(node).collect::<Vec<_>>(),
                    descendants
                );
            }
        }
    }
}
