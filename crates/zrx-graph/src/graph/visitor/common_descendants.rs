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

//! Iterator over common descendants of a set of nodes.

use std::collections::BTreeSet;

use crate::graph::topology::Distance;
use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over common descendants of a set of nodes.
pub struct CommonDescendants<'a> {
    /// Distance matrix.
    distance: &'a Distance,
    /// Set of common descendants.
    descendants: BTreeSet<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates an iterator over the common descendants of the set of nodes.
    ///
    /// # Panics
    ///
    /// Panics if a node does not exist, as this indicates that there's a bug
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
    /// builder.add_edge(a, c, 0)?;
    ///
    /// // Create graph from builder
    /// let graph = builder.build();
    ///
    /// // Create iterator over common descendants
    /// for nodes in graph.common_descendants([b, c]) {
    ///     println!("{nodes:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn common_descendants<N>(&self, nodes: N) -> CommonDescendants<'_>
    where
        N: AsRef<[usize]>,
    {
        let distance = self.topology.distance();
        let nodes = nodes.as_ref();

        // Compute common descendants by ensuring that each node in the given
        // set of nodes is reachable from the current node being considered
        let mut descendants = BTreeSet::default();
        for descendant in self {
            if nodes.iter().all(|&node| {
                node != descendant && distance[node][descendant] != u8::MAX
            }) {
                descendants.insert(descendant);
            }
        }

        // Create and return iterator
        CommonDescendants {
            distance: self.topology.distance(),
            descendants,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for CommonDescendants<'_> {
    type Item = Vec<usize>;

    /// Returns the next layer of common descendants.
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
    /// builder.add_edge(a, c, 0)?;
    /// builder.add_edge(b, c, 0)?;
    ///
    /// // Create graph from builder
    /// let graph = builder.build();
    ///
    /// // Create iterator over common descendants
    /// let mut descendants = graph.common_descendants([a, b]);
    /// while let Some(nodes) = descendants.next() {
    ///     println!("{nodes:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        if self.descendants.is_empty() {
            return None;
        }

        // Compute the next layer of common descendants - all nodes that are not
        // descendants of any other remaining common descendant. This process is
        // commonly referred to as peeling, where we iteratively remove layers
        // from the set of common descendants.
        let mut layer = Vec::new();
        for &descendant in &self.descendants {
            if !self.descendants.iter().any(|&node| {
                descendant != node && self.distance[node][descendant] != u8::MAX
            }) {
                layer.push(descendant);
            }
        }

        // Remove all nodes in the layer from the set of common descendants,
        // and return the layer if it's not empty. Otherwise, we're done.
        self.descendants.retain(|node| !layer.contains(node));
        (!layer.is_empty()).then_some(layer)
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod common_descendants {
        use crate::graph;

        #[test]
        fn handles_graph() {
            let graph = graph! {
                "a" => "d",
                "b" => "d", "b" => "e",
                "c" => "f", "c" => "g",
                "d" => "f", "d" => "g",
                "e" => "g",
            };
            assert_eq!(
                graph.common_descendants([0, 2]).collect::<Vec<_>>(),
                vec![vec![1], vec![5, 6]]
            );
        }

        #[test]
        fn handles_multi_graph() {
            let graph = graph! {
                "a" => "d",
                "b" => "d", "b" => "e", "b" => "e",
                "c" => "f", "c" => "g",
                "d" => "f", "d" => "g",
                "e" => "g",
            };
            assert_eq!(
                graph.common_descendants([0, 2]).collect::<Vec<_>>(),
                vec![vec![1], vec![5, 6]]
            );
        }
    }
}
