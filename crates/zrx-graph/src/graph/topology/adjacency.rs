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

//! Adjacency list.

use std::ops::{Index, Range};

use super::{Builder, Edge};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Adjacency list.
///
/// Since we only work with immutable graphs, we can model adjacencies of nodes
/// using the Compressed Sparse Row (CSR) format, which minimizes the necessary
/// storage needed. This is ideal for our use case, since it offers enumeration
/// of adjacent nodes in O(1), i.e., constant time.
///
/// We also compute the in- or out-degree for each node, depending on whether
/// an adjacency list for the incoming or outgoing edges is constructed, as
/// that's what we need when traversing the graph, and computing them once to
/// clone them saves a lot of time.
#[derive(Debug)]
pub struct Adjacency {
    /// Row pointer.
    rows: Vec<usize>,
    /// Column indices.
    columns: Vec<usize>,
    /// In- or out-degrees.
    degrees: Vec<u8>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Adjacency {
    /// Creates an adjacency list for outgoing edges.
    ///
    /// This method constructs an adjacency list from the graph builder, where
    /// each entry represents a node and the values represent the nodes that are
    /// reachable from that node via outgoing edges. If you need the adjacency
    /// list for all incoming edges, use [`Adjacency::incoming`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create adjacency list
    /// let adj = Adjacency::outgoing(&builder);
    /// assert_eq!(&adj[a], &[b]);
    /// assert_eq!(&adj[b], &[c]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn outgoing<T, W>(builder: &Builder<T, W>) -> Self
    where
        W: Clone,
    {
        Adjacency::new(builder.len(), builder.edges().to_vec())
    }

    /// Creates an adjacency list for incoming edges.
    ///
    /// This method constructs an adjacency list from the graph builder, where
    /// each entry represents a node and the values represent the nodes that are
    /// reachable from that node via incoming edges. If you need the adjacency
    /// list for all outgoing edges, use [`Adjacency::outgoing`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create adjacency list
    /// let adj = Adjacency::incoming(&builder);
    /// assert_eq!(&adj[b], &[a]);
    /// assert_eq!(&adj[c], &[b]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn incoming<T, W>(builder: &Builder<T, W>) -> Self
    where
        W: Clone,
    {
        let iter = builder.edges().iter().map(|edge| Edge {
            source: edge.target,
            target: edge.source,
            weight: edge.weight.clone(),
        });
        Adjacency::new(builder.len(), iter.collect())
    }

    /// Creates an adjacency list.
    ///
    /// This function does not take a full graph as input because we only need
    /// the actual edges to construct the adjacency list, as well as the number
    /// of nodes. This allows us to create adjacency lists for both directions
    /// without needing to duplicate code, as we can just invert the graph.
    ///
    /// # Panics
    ///
    /// Panics, if the number of edges exceeds the maximum value that can be
    /// represented as a `u8`, which is 255. This is an invariant that we expect
    /// to hold, as we do not expect any node to have more than 255 incoming or
    /// outgoing edges. While the number of incoming edges equals the number of
    /// arguments of a function, the number of outgoing edges equals the number
    /// of functions of which a node is an argument. The latter is more likely
    /// to be violated, while still very unlikely to happen in practice. We can
    /// lift this invariant if we run into this problem in the future.
    fn new<W>(nodes: usize, mut edges: Vec<Edge<W>>) -> Self {
        let mut rows = vec![0; nodes + 1];
        let mut columns = Vec::new();

        // First, we need to sort the edges according to the source node, which
        // allows us to construct the adjacency list in linear time. Since we
        // expect that data is (almost) sorted, we use insertion sort, which is
        // stable and runs in O(n + k), where k is the number of inversions.
        insertion_sort(edges.as_mut_slice(), |edge| edge.source);

        // Process edges and add pointers for all rows until we reach the source
        // node of the current edge, representing rows without outgoing edges
        let mut r = 0;
        for edge in edges {
            while r <= edge.source {
                rows[r] = columns.len();
                r += 1;
            }
            columns.push(edge.target);
        }

        // Fill in the remaining row pointers
        while r <= nodes {
            rows[r] = columns.len();
            r += 1;
        }

        // Compute in- or out-degrees
        let mut degrees = vec![0u8; nodes];
        for r in 0..nodes {
            let degree = rows[r + 1] - rows[r];
            degrees[r] = u8::try_from(degree).expect("invariant");
        }

        // Return adjacency list
        Adjacency { rows, columns, degrees }
    }

    /// Creates an iterator over the adjacency list.
    ///
    /// This iterator emits the node indices, which is exactly the same as
    /// iterating over the adjacency list using `0..self.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create iterator over adjacency list
    /// let adj = Adjacency::outgoing(&builder);
    /// for node in adj.iter() {
    ///     println!("{node:?} -> {:?}", &adj[node]);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Range<usize> {
        0..self.len()
    }
}

#[allow(clippy::must_use_candidate)]
impl Adjacency {
    /// Returns a reference to the in- or out-degrees.
    #[inline]
    pub fn degrees(&self) -> &[u8] {
        &self.degrees
    }

    /// Returns the number of rows.
    #[inline]
    pub fn len(&self) -> usize {
        self.rows.len() - 1
    }

    /// Returns whether there are any rows.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Index<usize> for Adjacency {
    type Output = [usize];

    /// Returns the column indices for the given row index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create adjacency list
    /// let adj = Adjacency::outgoing(&builder);
    /// assert_eq!(&adj[a], &[b]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let start = self.rows[index];
        let end = self.rows[index + 1];
        &self.columns[start..end]
    }
}

// ----------------------------------------------------------------------------

impl IntoIterator for &Adjacency {
    type Item = usize;
    type IntoIter = Range<usize>;

    /// Creates an iterator over the adjacency list.
    ///
    /// This iterator emits the node indices, which is exactly the same as
    /// iterating over the adjacency list using `0..self.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create iterator over adjacency list
    /// let adj = Adjacency::outgoing(&builder);
    /// for node in &adj {
    ///     println!("{node:?} -> {:?}", &adj[node]);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Sorts the edges in-place using insertion sort based on a key function, which
/// is more efficient than Rust's sort implementation for (almost) sorted lists.
fn insertion_sort<W, F>(edges: &mut [Edge<W>], f: F)
where
    F: Fn(&Edge<W>) -> usize,
{
    for i in 1..edges.len() {
        let mut j = i;
        while j > 0 && f(&edges[j - 1]) > f(&edges[j]) {
            edges.swap(j - 1, j);
            j -= 1;
        }
    }
}
