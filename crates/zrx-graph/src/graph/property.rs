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

//! Graph properties.

use super::Graph;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Returns whether the given node is a source.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Topology};
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
    /// // Ensure nodes are sources (or not)
    /// assert_eq!(graph.is_source(a), true);
    /// assert_eq!(graph.is_source(b), false);
    /// assert_eq!(graph.is_source(c), false);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn is_source(&self, node: usize) -> bool {
        let incoming = self.topology.incoming();
        incoming[node].is_empty()
    }

    /// Returns whether the given node is a sink.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Topology};
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
    /// // Ensure nodes are sinks (or not)
    /// assert_eq!(graph.is_sink(a), false);
    /// assert_eq!(graph.is_sink(b), false);
    /// assert_eq!(graph.is_sink(c), true);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn is_sink(&self, node: usize) -> bool {
        let outgoing = self.topology.outgoing();
        outgoing[node].is_empty()
    }

    /// Returns whether the source node is an ancestor of the target node.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Topology};
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
    /// // Ensure nodes are ancestors (or not)
    /// assert_eq!(graph.is_ancestor(a, b), true);
    /// assert_eq!(graph.is_ancestor(a, c), true);
    /// assert_eq!(graph.is_ancestor(b, a), false);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn is_ancestor(&self, source: usize, target: usize) -> bool {
        let distance = self.topology.distance();
        distance[source][target] != u8::MAX
    }

    /// Returns whether the source node is a descendant of the target node.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Topology};
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
    /// // Ensure nodes are descendants (or not)
    /// assert_eq!(graph.is_descendant(b, a), true);
    /// assert_eq!(graph.is_descendant(c, a), true);
    /// assert_eq!(graph.is_descendant(a, b), false);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn is_descendant(&self, source: usize, target: usize) -> bool {
        let distance = self.topology.distance();
        distance[target][source] != u8::MAX
    }

    /// Returns whether the source node is a predecessor of the target node.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Topology};
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
    /// // Ensure nodes are predecessors (or not)
    /// assert_eq!(graph.is_predecessor(a, b), true);
    /// assert_eq!(graph.is_predecessor(a, c), false);
    /// assert_eq!(graph.is_predecessor(b, a), false);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn is_predecessor(&self, source: usize, target: usize) -> bool {
        let distance = self.topology.distance();
        distance[source][target] == 1
    }

    /// Returns whether the source node is a successor of the target node.
    ///
    /// # Panics
    ///
    /// Panics if the node does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Topology};
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
    /// // Ensure nodes are successors (or not)
    /// assert_eq!(graph.is_successor(b, a), true);
    /// assert_eq!(graph.is_successor(c, a), false);
    /// assert_eq!(graph.is_successor(a, b), false);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn is_successor(&self, source: usize, target: usize) -> bool {
        let distance = self.topology.distance();
        distance[target][source] == 1
    }
}
