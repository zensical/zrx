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

//! Topological traversal.

use std::collections::VecDeque;

use super::error::{Error, Result};
use super::topology::Topology;

mod iter;

pub use iter::IntoIter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Topological traversal.
///
/// This data type manages a topological traversal of a directed acyclic graph
/// (DAG). It allows visiting nodes in a way that respects their dependencies,
/// meaning that a node can only be visited after all of its dependencies have
/// been visited. Visitable nodes can be obtained with [`Traversal::take`].
///
/// Note that the traversal itself doesn't know whether it's complete or not,
/// as it only tracks visitable nodes depending on what has been reported back
/// to [`Traversal::complete`]. This is because we also need to support partial
/// traversals that can be resumed, which must be managed by the caller. In case
/// a traversal starts at an intermediate node, only the nodes and dependencies
/// reachable from this node are considered, which is necessary for implementing
/// subgraph traversals that are self-contained, allowing for the creation of
/// frontiers at any point in the graph.
#[derive(Clone, Debug)]
pub struct Traversal {
    /// Graph topology.
    topology: Topology,
    /// Dependency counts.
    dependencies: Vec<u8>,
    /// Visitable nodes.
    visitable: VecDeque<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Traversal {
    /// Creates a topological traversal.
    ///
    /// The given initial nodes are immediately marked as visitable, and thus
    /// returned by [`Traversal::take`], so the caller must make sure they can
    /// be processed. Note that the canonical way to create a [`Traversal`] is
    /// to invoke the [`Graph::traverse`][] method.
    ///
    /// [`Graph::traverse`]: crate::graph::Graph::traverse
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::{Graph, Traversal};
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
    /// // Create topological traversal
    /// let traversal = Traversal::new(graph.topology(), [a]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new<I>(topology: &Topology, initial: I) -> Self
    where
        I: AsRef<[usize]>,
    {
        let mut visitable: VecDeque<_> =
            initial.as_ref().iter().copied().collect();

        // Obtain incoming edges and distance matrix
        let incoming = topology.incoming();
        let distance = topology.distance();

        // When doing a topological traversal, we only visit a node once all of
        // its dependencies have been visited. This means that we need to track
        // the number of dependencies for each node, which is the number of
        // incoming edges for that node.
        let mut dependencies = incoming.degrees().to_vec();
        for node in incoming {
            // We must adjust the dependency count for each node for all of its
            // dependencies that are not reachable from the initial nodes
            for &dependency in &incoming[node] {
                let mut iter = visitable.iter();
                if !iter.any(|&n| distance[n][dependency] != u8::MAX) {
                    dependencies[node] -= 1;
                }
            }
        }

        // Retain only the visitable nodes whose dependencies are satisfied,
        // as we will discover the other initial nodes during traversal
        visitable.retain(|&node| dependencies[node] == 0);
        Self {
            topology: topology.clone(),
            dependencies,
            visitable,
        }
    }

    /// Returns the next visitable node.
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
    /// // Create topological traversal
    /// let mut traversal = graph.traverse([a]);
    /// while let Some(node) = traversal.take() {
    ///     println!("{node:?}");
    ///     traversal.complete(node)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn take(&mut self) -> Option<usize> {
        self.visitable.pop_front()
    }

    /// Marks the given node as visited.
    ///
    /// This method marks a node as visited as part of a traversal, which might
    /// allow visiting dependent nodes when all of their dependencies have been
    /// satisfied. After marking a node as visited, the next nodes that can be
    /// visited can be obtained using the [`Traversal::take`] method.
    ///
    /// # Errors
    ///
    /// In case the node has already been marked as visited, [`Error::Found`]
    /// is returned. This is likely an error in the traversal business logic.
    ///
    /// # Panics
    ///
    /// Panics if a node does not exist, as this indicates that there's a bug
    /// in the code that creates or uses the traversal. While the [`Builder`][]
    /// is designed to be fallible to ensure the structure is valid, methods
    /// that operate on [`Graph`][] panic on violated invariants.
    ///
    /// [`Builder`]: crate::graph::Builder
    /// [`Graph`]: crate::graph::Graph
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
    /// // Create topological traversal
    /// let mut traversal = graph.traverse([a]);
    /// while let Some(node) = traversal.take() {
    ///     println!("{node:?}");
    ///     traversal.complete(node)?;
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn complete(&mut self, node: usize) -> Result {
        if self.dependencies[node] == u8::MAX {
            return Err(Error::Found(node));
        }

        // Mark node as visited - we can just use the maximum value of `u8` as
        // a marker, as we don't expect more than 255 dependencies for any node
        self.dependencies[node] = u8::MAX;

        // Obtain adjacency list of outgoing edges, and decrement the number
        // of unresolved dependencies for each dependent by one. When the number
        // of dependencies for a dependent reaches zero, it can be visited, so
        // we add it to the queue of visitable nodes.
        let outgoing = self.topology.outgoing();
        for &dependent in &outgoing[node] {
            self.dependencies[dependent] -= 1;

            // We satisfied all dependencies, so the dependent can be visited
            if self.dependencies[dependent] == 0 {
                self.visitable.push_back(dependent);
            }
        }

        // No errors occurred.
        Ok(())
    }
}

#[allow(clippy::must_use_candidate)]
impl Traversal {
    /// Returns the graph topology.
    #[inline]
    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Returns the number of visitable nodes.
    #[inline]
    pub fn len(&self) -> usize {
        self.visitable.len()
    }

    /// Returns whether there are any visitable nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.visitable.is_empty()
    }
}
