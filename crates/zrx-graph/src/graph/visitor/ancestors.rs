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

//! Iterator over ancestors of a node.

use ahash::HashSet;

use crate::graph::topology::Adjacency;
use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over ancestors of a node.
pub struct Ancestors<'a> {
    /// Incoming edges.
    incoming: &'a Adjacency,
    /// Stack for depth-first search.
    stack: Vec<usize>,
    /// Set of visited nodes.
    visited: HashSet<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates an iterator over the ancestors of the given node.
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
    /// // Create iterator over ancestors
    /// for node in graph.ancestors(c) {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn ancestors(&self, node: usize) -> Ancestors<'_> {
        let mut iter = Ancestors {
            incoming: self.topology.incoming(),
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

impl Iterator for Ancestors<'_> {
    type Item = usize;

    /// Returns the next ancestor.
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
    /// // Create iterator over ancestors
    /// let mut ancestors = graph.ancestors(c);
    /// while let Some(node) = ancestors.next() {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        // Perform a depth-first search to find all ancestors of a node, by
        // exploring them iteratively, not including the node itself
        let node = self.stack.pop()?;
        for &ancestor in self.incoming[node].iter().rev() {
            // If we haven't visited this ancestor yet, we put it on the
            // stack after marking it as visited and return it immediately
            if self.visited.insert(ancestor) {
                self.stack.push(ancestor);
            }
        }

        // Return the next ancestor
        Some(node)
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod ancestors {
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
            for (node, ancestors) in [
                (0, vec![]),
                (1, vec![0]),
                (2, vec![0]),
                (3, vec![1, 0]),
                (4, vec![1, 0]),
                (5, vec![2, 0]),
                (6, vec![3, 1, 0, 4]),
                (7, vec![4, 1, 0, 5, 2]),
                (8, vec![6, 3, 1, 0, 4, 7, 5, 2]),
            ] {
                assert_eq!(
                    graph.ancestors(node).collect::<Vec<_>>(),
                    ancestors
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
            for (node, ancestors) in [
                (0, vec![]),
                (1, vec![0]),
                (2, vec![0]),
                (3, vec![1, 0]),
                (4, vec![1, 0]),
                (5, vec![2, 0]),
                (6, vec![3, 1, 0, 4]),
                (7, vec![4, 1, 0, 5, 2]),
                (8, vec![6, 3, 1, 0, 4, 7, 5, 2]),
            ] {
                assert_eq!(
                    graph.ancestors(node).collect::<Vec<_>>(),
                    ancestors
                );
            }
        }
    }
}
