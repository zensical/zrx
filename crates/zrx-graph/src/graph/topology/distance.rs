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

//! Distance matrix.

use std::ops::Index;

use super::adjacency::Adjacency;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Distance matrix.
///
/// This data type stores the all-pairs shortest path distances for all nodes
/// in a directed acyclic graph (DAG). It's computed through the Floyd-Warshall
/// algorithm, and allows for efficient retrieval of distances between any two
/// nodes, which is essential for many graph algorithms.
#[derive(Debug)]
pub struct Distance {
    /// Row number.
    rows: usize,
    /// Column values.
    columns: Vec<u8>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Distance {
    /// Creates a distance matrix.
    ///
    /// This method is called by [`Topology::new`][], and is not intended to be
    /// used on its own, since an adjacency list is needed to create the matrix.
    /// Computation is expensive, which is why [`Topology`] will defer creation
    /// via [`OnceCell`], so it's only computed when first accessed.
    ///
    /// [`OnceCell`]: std::cell::OnceCell
    /// [`Topology`]: crate::graph::topology::Topology
    /// [`Topology::new`]: crate::graph::topology::Topology::new
    #[must_use]
    pub fn new(adj: &Adjacency) -> Self {
        let nodes = adj.len();
        let mut data = vec![u8::MAX; nodes * nodes];

        // Initialize the distance for all nodes to themselves to 0
        for source in adj {
            data[source * nodes + source] = 0;

            // Initialize the distances for all directed edges to 1
            for &target in &adj[source] {
                data[source * nodes + target] = 1;
            }
        }

        // Create distance matrix and compute all-pairs shortest paths
        let mut dist = Self { rows: nodes, columns: data };
        floyd_warshall(&mut dist);
        dist
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Index<usize> for Distance {
    type Output = [u8];

    /// Returns the column values for the given row.
    ///
    /// This method returns a slice representing the distances from the node as
    /// identified by the given index to all other nodes in the graph. Distances
    /// are represented as the number of edges on the shortest path between the
    /// nodes. For all unreachable nodes, the distance equals [`u8::MAX`].
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
    /// use zrx_graph::topology::Topology;
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
    /// // Create topology
    /// let topology = Topology::new(&builder);
    ///
    /// // Obtain distance matrix
    /// let dist = topology.distance();
    /// assert_eq!(dist[a][c], 2);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        let start = index * self.rows;
        let end = start + self.rows;
        &self.columns[start..end]
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Executes the Floyd-Warshall algorithm to compute all-pairs shortest paths,
/// updating the distance matrix in-place. While Floyd-Warshall is not the most
/// efficient algorithm for sparse graphs, it is simple and effective enough to
/// implement, and should be sufficient for our case.
fn floyd_warshall(dist: &mut Distance) {
    let n = dist.rows;
    for k in 0..n {
        for i in 0..n {
            // Obtain distance from i to k, then check whether the path is
            // marked as reachable, and obtain all distances from k to j
            let i_to_k = dist.columns[i * n + k];
            if i_to_k != u8::MAX {
                for j in 0..n {
                    // If j is reachable from k, compute the distance from
                    // i to j via k, and update the distance matrix
                    let k_to_j = dist.columns[k * n + j];
                    if k_to_j != u8::MAX {
                        let value = i_to_k + k_to_j;

                        // Update the distance matrix
                        let i_to_j = &mut dist.columns[i * n + j];
                        if *i_to_j > value {
                            *i_to_j = value;
                        }
                    }
                }
            }
        }
    }
}
