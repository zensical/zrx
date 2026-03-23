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

use ahash::HashSet;
use std::collections::VecDeque;
use std::mem;

use super::topology::Topology;
use super::Graph;

mod error;
mod into_iter;

pub use error::{Error, Result};
pub use into_iter::IntoIter;

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
    /// Initial nodes.
    initial: Vec<usize>,
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
    /// to invoke the [`Graph::traverse`] method.
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
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
            unique(initial.as_ref().iter()).collect();

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
                let mut iter = initial.as_ref().iter();
                if !iter.any(|&n| distance[n][dependency] != u8::MAX) {
                    dependencies[node] -= 1;
                }
            }
        }

        // Retain only the visitable nodes whose dependencies are satisfied,
        // as we will discover the other initial nodes during traversal
        visitable.retain(|&n| dependencies[n] == 0);
        Self {
            topology: topology.clone(),
            dependencies,
            initial: visitable.iter().copied().collect(),
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
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
    /// If the node has already been marked as visited, [`Error::Completed`] is
    /// returned. This is likely an error in the caller's business logic.
    ///
    /// # Panics
    ///
    /// Panics if a node does not exist, as this indicates that there's a bug
    /// in the code that creates or uses the traversal. While the [`Builder`][]
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
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
            return Err(Error::Completed(node));
        }

        // When the dependency count isn't zero, the traversal converged with
        // another traversal and restarted at a node occurring before the given
        // node. In this case, we return an error, and indicate to the caller
        // that parts of the traversal have to be completed again.
        if self.dependencies[node] != 0 {
            return Err(Error::Converged);
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

        // No errors occurred
        Ok(())
    }

    /// Attempts to converge with the given traversal.
    ///
    /// This method attempts to merge both traversals into a single traversal,
    /// which is possible when they have common descendants, a condition that
    /// is always true for directed acyclic graphs with a single component.
    /// There are several cases to consider when converging two traversals:
    ///
    /// - If traversals start from the same set of source nodes, they already
    ///   converged, so we just restart the traversal at these source nodes.
    ///
    /// - If traversals start from different source nodes, yet both have common
    ///   descendants, we converge at the first layer of common descendants, as
    ///   all descendants of them must be revisited in the combined traversal.
    ///   Ancestors of the common descendants that have already been visited in
    ///   either traversal don't need to be revisited, and thus are carried over
    ///   from both traversals in their current state.
    ///
    /// - If traversals are disjoint, they can't be converged, so we return the
    ///   given traversal back to the caller wrapped in [`Error::Disjoint`].
    ///
    /// # Errors
    ///
    /// If the given traversal is from a different graph, [`Error::Mismatch`]
    /// is returned. Otherwise, if the traversals are disjoint, the traversal
    /// is returned back to the caller wrapped in [`Error::Disjoint`], so the
    /// caller can decide how to proceed.
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
    /// builder.add_edge(a, c)?;
    /// builder.add_edge(b, c)?;
    ///
    /// // Create graph from builder
    /// let graph = builder.build();
    ///
    /// // Create topological traversal
    /// let mut traversal = graph.traverse([a]);
    ///
    /// // Converge with another topological traversal
    /// let mut other = graph.traverse([b]);
    /// assert!(traversal.converge(other).is_ok());
    /// # Ok(())
    /// # }
    /// ```
    pub fn converge(&mut self, other: Self) -> Result {
        if self.topology != other.topology {
            return Err(Error::Mismatch);
        }

        // Compute the initial nodes for the combined traversal, which is the
        // union of both traversals with all redundant nodes removed
        let iter = self.initial.iter().chain(&other.initial);
        let initial: Vec<_> = unique(iter).collect();

        // Create a temporary graph, so we can compute the first layer of nodes
        // that are common descendants contained in both traversals
        let graph = Graph {
            data: (0..self.topology.incoming().len()).collect(),
            topology: self.topology.clone(),
        };

        // If there are no common descendants, the traversals are disjoint and
        // can't converge, so we return the given traversal back to the caller
        let mut iter = graph.common_descendants(&initial);
        let Some(common) = iter.next() else {
            return Err(Error::Disjoint(other));
        };

        // Create the combined traversal, and mark all already visited nodes
        // that are ancestors of the common descendants as visited
        let prior = mem::replace(self, Self::new(&self.topology, initial));

        // Compute the visitable nodes for the combined traversal, which is the
        // union of both traversals with all redundant nodes removed. Note that
        // we must collect them in a temporary vector first, or this loop would
        // run indefinitely, as we'd be adding visitable nodes again and again.
        let mut visitable = VecDeque::new();
        while let Some(node) = self.take() {
            let p = prior.dependencies[node];
            let o = other.dependencies[node];

            // If the node has been visited in either traversal, and is not part
            // of the first layer of common descendants, mark it as visited in
            // the combined traversal, since we don't need to revisit ancestors
            // that have already been visited in either traversal
            if (p == u8::MAX || o == u8::MAX) && !common.contains(&node) {
                self.complete(node)?;
            } else {
                visitable.push_back(node);
            }
        }

        // Update visitable nodes
        self.visitable = visitable;

        // No errors occurred
        Ok(())
    }
}

#[allow(clippy::must_use_candidate)]
impl Traversal {
    /// Returns a reference to the graph topology.
    #[inline]
    pub fn topology(&self) -> &Topology {
        &self.topology
    }

    /// Returns a reference to the initial nodes.
    #[inline]
    pub fn initial(&self) -> &[usize] {
        &self.initial
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

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Deduplicates the given nodes while preserving their order.
#[inline]
fn unique<'a, I>(iter: I) -> impl Iterator<Item = usize>
where
    I: IntoIterator<Item = &'a usize>,
{
    let mut nodes = HashSet::default();
    iter.into_iter() // fmt
        .copied()
        .filter(move |&node| nodes.insert(node))
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod complete {
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
            for (node, mut descendants) in [
                (0, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (1, vec![1, 3, 4, 6, 7, 8]),
                (2, vec![2, 5, 7, 8]),
                (3, vec![3, 6, 8]),
                (4, vec![4, 6, 7, 8]),
                (5, vec![5, 7, 8]),
                (6, vec![6, 8]),
                (7, vec![7, 8]),
                (8, vec![8]),
            ] {
                let mut traversal = graph.traverse([node]);
                while let Some(node) = traversal.take() {
                    assert_eq!(node, descendants.remove(0));
                    assert!(traversal.complete(node).is_ok());
                }
            }
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
            for (node, mut descendants) in [
                (0, vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (1, vec![1, 3, 4, 6, 7, 8]),
                (2, vec![2, 5, 7, 8]),
                (3, vec![3, 6, 8]),
                (4, vec![4, 6, 7, 8]),
                (5, vec![5, 7, 8]),
                (6, vec![6, 8]),
                (7, vec![7, 8]),
                (8, vec![8]),
            ] {
                let mut traversal = graph.traverse([node]);
                while let Some(node) = traversal.take() {
                    assert_eq!(node, descendants.remove(0));
                    assert!(traversal.complete(node).is_ok());
                }
            }
        }
    }

    mod converge {
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
            for (i, j, descendants) in [
                (vec![0], vec![0], vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![1], vec![0], vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![8], vec![0], vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![1], vec![1], vec![1, 3, 4, 6, 7, 8]),
                (vec![1], vec![2], vec![1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![2], vec![4], vec![2, 4, 5, 6, 7, 8]),
                (vec![4], vec![2], vec![4, 2, 6, 5, 7, 8]),
                (vec![3], vec![5], vec![3, 5, 6, 7, 8]),
                (vec![6], vec![7], vec![6, 7, 8]),
                (vec![8], vec![8], vec![8]),
            ] {
                let mut traversal = graph.traverse(i);
                assert!(traversal.converge(graph.traverse(j)).is_ok());
                assert_eq!(
                    traversal.into_iter().collect::<Vec<_>>(), // fmt
                    descendants
                );
            }
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
            for (i, j, descendants) in [
                (vec![0], vec![0], vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![1], vec![0], vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![8], vec![0], vec![0, 1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![1], vec![1], vec![1, 3, 4, 6, 7, 8]),
                (vec![1], vec![2], vec![1, 2, 3, 4, 5, 6, 7, 8]),
                (vec![2], vec![4], vec![2, 4, 5, 6, 7, 8]),
                (vec![4], vec![2], vec![4, 2, 6, 5, 7, 8]),
                (vec![3], vec![5], vec![3, 5, 6, 7, 8]),
                (vec![6], vec![7], vec![6, 7, 8]),
                (vec![8], vec![8], vec![8]),
            ] {
                let mut traversal = graph.traverse(i);
                assert!(traversal.converge(graph.traverse(j)).is_ok());
                assert_eq!(
                    traversal.into_iter().collect::<Vec<_>>(), // fmt
                    descendants
                );
            }
        }
    }
}
