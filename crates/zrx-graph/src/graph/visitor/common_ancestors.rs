// Copyright (c) 2025 Zensical and contributors

// SPDX-License-Identifier: MIT
// Third-party contributions licensed under DCO

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

//! Iterator over common ancestors of a set of nodes.

use std::collections::BTreeSet;

use crate::graph::topology::Distance;
use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over common ancestors of a set of nodes.
pub struct CommonAncestors<'a> {
    /// Distance matrix.
    distance: &'a Distance,
    /// Set of common ancestors.
    ancestors: BTreeSet<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates an iterator over the common ancestors of the set of nodes.
    ///
    /// This method creates an iterator over the common ancestores of a given
    /// set of nodes, and emits them in layers, starting with the lowest common
    /// ancestors (LCA). In directed acyclic graphs, nodes might have multiple
    /// common ancestors, since there can be multiple paths leading to the nodes
    /// in the provided set. The iterator peels these common ancestors layer by
    /// layer, emitting all sinks among the remaining nodes at each step.
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
    /// // Create iterator over common ancestors
    /// for nodes in graph.common_ancestors([b, c]) {
    ///     println!("{nodes:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn common_ancestors<N>(&self, nodes: N) -> CommonAncestors<'_>
    where
        N: AsRef<[usize]>,
    {
        let distance = self.topology.distance();
        let nodes = nodes.as_ref();

        // Compute common ancestors by ensuring that each node in the given set
        // of nodes is reachable from the current node being considered
        let mut ancestors = BTreeSet::default();
        for ancestor in self {
            if nodes.iter().all(|&node| {
                node != ancestor && distance[ancestor][node] != u8::MAX
            }) {
                ancestors.insert(ancestor);
            }
        }

        // Create and return iterator
        CommonAncestors {
            distance: self.topology.distance(),
            ancestors,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for CommonAncestors<'_> {
    type Item = Vec<usize>;

    /// Returns the next layer of common ancestors.
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
    /// // Create iterator over common ancestors
    /// let mut ancestors = graph.common_ancestors([b, c]);
    /// while let Some(nodes) = ancestors.next() {
    ///     println!("{nodes:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        if self.ancestors.is_empty() {
            return None;
        }

        // Compute the next layer of common ancestors - all nodes that are not
        // ancestors of any other remaining common ancestor. This process is
        // commonly referred to as peeling, where we iteratively remove layers
        // from the set of common ancestors.
        let mut layer = Vec::new();
        for &ancestor in &self.ancestors {
            if !self.ancestors.iter().any(|&node| {
                ancestor != node && self.distance[ancestor][node] != u8::MAX
            }) {
                layer.push(ancestor);
            }
        }

        // Remove all nodes in the layer from the set of common ancestors, and
        // return the layer if it's not empty. Otherwise, we're done.
        self.ancestors.retain(|node| !layer.contains(node));
        (!layer.is_empty()).then_some(layer)
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod common_ancestors {
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
                graph.common_ancestors([5, 6]).collect::<Vec<_>>(),
                vec![vec![1, 4], vec![0, 2]]
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
                graph.common_ancestors([5, 6]).collect::<Vec<_>>(),
                vec![vec![1, 4], vec![0, 2]]
            );
        }
    }
}
