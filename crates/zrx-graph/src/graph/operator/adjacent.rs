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

//! Adjacent operator.

use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Adjacent nodes.
pub struct Adjacent<'a> {
    /// Incoming edges.
    pub incoming: &'a [usize],
    /// Outgoing edges.
    pub outgoing: &'a [usize],
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Retrieve a reference to a node and its adjacent nodes.
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
    ///
    /// // Create graph from builder and retrieve node
    /// let graph = builder.build();
    ///
    /// // Obtain reference to node and adjacent nodes
    /// let (data, adj) = graph.adjacent(a);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn adjacent(&self, node: usize) -> (&'_ T, Adjacent<'_>) {
        let incoming = self.topology.incoming();
        let outgoing = self.topology.outgoing();
        (
            &self.data[node],
            Adjacent {
                incoming: &incoming[node],
                outgoing: &outgoing[node],
            },
        )
    }

    /// Retrieve a mutable reference to a node and its adjacent nodes.
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
    ///
    /// // Create graph from builder and retrieve node
    /// let mut graph = builder.build();
    ///
    /// // Obtain mutable reference to node and adjacent nodes
    /// let (data, adj) = graph.adjacent_mut(a);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn adjacent_mut(&mut self, node: usize) -> (&'_ mut T, Adjacent<'_>) {
        let incoming = self.topology.incoming();
        let outgoing = self.topology.outgoing();
        (
            &mut self.data[node],
            Adjacent {
                incoming: &incoming[node],
                outgoing: &outgoing[node],
            },
        )
    }
}
