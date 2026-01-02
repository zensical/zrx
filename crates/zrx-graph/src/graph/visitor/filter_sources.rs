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

//! Iterator over sources in a set of nodes.

use crate::graph::topology::Distance;
use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over sources in a set of nodes.
pub struct FilterSources<'a> {
    /// Distance matrix.
    distance: &'a Distance,
    /// Nodes to filter.
    nodes: Vec<usize>,
    /// Current index.
    index: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates an iterator over the sources in the given set of nodes.
    ///
    /// This method creates a view over the provided set of nodes, representing
    /// an embedded subgraph, and emits only those nodes that are sources within
    /// this subgraph. Sources are nodes that do not have incoming edges from
    /// other nodes in the given set.
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
    /// builder.add_edge(b, c, 0)?;
    ///
    /// // Create graph from builder
    /// let graph = builder.build();
    ///
    /// // Create iterator over sources
    /// for node in graph.filter_sources([b, c]) {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn filter_sources<N>(&self, nodes: N) -> FilterSources<'_>
    where
        N: Into<Vec<usize>>,
    {
        FilterSources {
            distance: self.topology.distance(),
            nodes: nodes.into(),
            index: 0,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for FilterSources<'_> {
    type Item = usize;

    /// Returns the next source.
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
    /// // Create iterator over sources
    /// let mut sources = graph.filter_sources([b, c]);
    /// while let Some(node) = sources.next() {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.nodes.len() {
            let node = self.nodes[self.index];
            self.index += 1;

            // Emit the node if it's a source, which is true if it can't be
            // reached from any other node in the set through any path
            if !self.nodes.iter().any(|&ancestor| {
                node != ancestor && self.distance[ancestor][node] != u8::MAX
            }) {
                return Some(node);
            }
        }

        // No more sources to return
        None
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod filter_sources {
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
            assert_eq!(
                graph.filter_sources([1, 3, 4, 5, 7]).collect::<Vec<_>>(),
                vec![1, 5]
            );
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
            assert_eq!(
                graph.filter_sources([2, 4, 5, 6]).collect::<Vec<_>>(),
                vec![2, 4]
            );
        }
    }
}
