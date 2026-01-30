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

//! Topology.

use std::cell::OnceCell;
use std::rc::Rc;

use super::builder::Builder;

mod adjacency;
mod distance;

pub use adjacency::Adjacency;
pub use distance::Distance;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Topology.
///
/// This data type represents the topology of a graph, which allows to find the
/// outgoing and incoming edges for each node in linear time by using efficient
/// adjacency lists. Note that our implementation does not support edge weights,
/// as they're not necessary for scheduling purposes, and they would only add
/// unnecessary complexity and overhead. The topology also contains the lazily
/// computed [`Distance`] matrix that allows to find the shortest path between
/// two nodes in the graph, or determine whether they're reachable at all.
///
/// The graph topology must be considered immutable, as [`Adjacency`] lists
/// can't be mutated anyway, and represents the conversion of a graph into an
/// executable form. In order to modify the graph, convert it back into a
/// builder using the [`Graph::into_builder`][] method.
///
/// The [`Topology`] data type is just a wrapper around [`TopologyInner`] with
/// an [`Rc`], so it can be shared between the [`Graph`][] and [`Traversal`][]
/// structures without the need for lifetime annotations, which would render
/// incremental and asynchronous traversals of graphs more complex.
///
/// [`Graph`]: crate::graph::Graph
/// [`Graph::into_builder`]: crate::graph::Graph::into_builder
/// [`Traversal`]: crate::graph::traversal::Traversal
#[derive(Clone, Debug)]
pub struct Topology(Rc<TopologyInner>);

// ----------------------------------------------------------------------------

/// Topology inner state.
#[derive(Debug)]
struct TopologyInner {
    /// Outgoing edges.
    outgoing: Adjacency,
    /// Incoming edges.
    incoming: Adjacency,
    /// Distance matrix (computed on first access).
    distance: OnceCell<Distance>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

#[allow(clippy::must_use_candidate)]
impl Topology {
    /// Returns a reference to the outgoing edges.
    #[inline]
    pub fn outgoing(&self) -> &Adjacency {
        &self.0.outgoing
    }

    /// Returns a reference to the incoming edges.
    #[inline]
    pub fn incoming(&self) -> &Adjacency {
        &self.0.incoming
    }

    /// Returns a reference to the distance matrix.
    #[inline]
    pub fn distance(&self) -> &Distance {
        self.0.distance.get_or_init(|| {
            // Compute distance matrix on first access, since this incurs cost
            // of O(n³) because of the usage of the Floyd-Warshall algorithm.
            Distance::new(&self.0.outgoing)
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<&Builder<T>> for Topology {
    /// Creates a topology of the given graph.
    ///
    /// This method constructs a topology from a graph builder, one of the key
    /// components of an executable [`Graph`][]. Thus, it's usually not needed
    /// to create a topology manually, as it's automatically created when the
    /// graph is built using the [`Builder::build`] method.
    ///
    /// [`Graph`]: crate::graph::Graph
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
    ///
    /// // Create topology
    /// let topology = Topology::from(&builder);
    /// # Ok(())
    /// # }
    /// ```
    fn from(builder: &Builder<T>) -> Self {
        Self(Rc::new(TopologyInner {
            outgoing: Adjacency::outgoing(builder),
            incoming: Adjacency::incoming(builder),
            distance: OnceCell::new(),
        }))
    }
}
