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

//! With operator.

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
    /// Retrieve a reference to a node's data.
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
    /// // Create graph from builder and retrieve nodes
    /// let graph = builder.build();
    /// for node in &graph {
    ///     graph.with(node, |name, _| {
    ///         println!("{name:?}");
    ///     });
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with<F, R>(&self, node: usize, f: F) -> R
    where
        F: FnOnce(&T, Adjacent) -> R,
    {
        let incoming = self.topology.incoming();
        let outgoing = self.topology.outgoing();
        f(
            &self.data[node],
            Adjacent {
                incoming: &incoming[node],
                outgoing: &outgoing[node],
            },
        )
    }

    /// Retrieve a mutable reference to a node's data.
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
    /// for node in &graph {
    ///     graph.with_mut(node, |name, _| {
    ///         println!("{name:?}");
    ///     });
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with_mut<F, R>(&mut self, node: usize, f: F) -> R
    where
        F: FnOnce(&mut T, Adjacent) -> R,
    {
        let incoming = self.topology.incoming();
        let outgoing = self.topology.outgoing();
        f(
            &mut self.data[node],
            Adjacent {
                incoming: &incoming[node],
                outgoing: &outgoing[node],
            },
        )
    }
}
