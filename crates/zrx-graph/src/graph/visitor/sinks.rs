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

//! Iterator over sinks.

use crate::graph::topology::Adjacency;
use crate::graph::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over sinks.
pub struct Sinks<'a> {
    /// Outgoing edges.
    outgoing: &'a Adjacency,
    /// Current index.
    index: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates an iterator over the sinks.
    ///
    /// This method returns an iterator over the sink node indices of the
    /// graph, which are the nodes with no outgoing edges.
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
    /// // Create iterator over sinks
    /// for node in graph.sinks() {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn sinks(&self) -> Sinks<'_> {
        Sinks {
            outgoing: self.topology.outgoing(),
            index: 0,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for Sinks<'_> {
    type Item = usize;

    /// Returns the next sink.
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
    /// // Create iterator over sinks
    /// let mut sinks = graph.sinks();
    /// while let Some(node) = sinks.next() {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.outgoing.len() {
            let node = self.index;
            self.index += 1;

            // Emit the node if it has no outgoing edges
            if self.outgoing[node].is_empty() {
                return Some(node);
            }
        }

        // No more sinks to return
        None
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod sinks {
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
                graph.sinks().collect::<Vec<_>>(), // fmt
                vec![8]
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
                graph.sinks().collect::<Vec<_>>(), // fmt
                vec![8]
            );
        }
    }
}
