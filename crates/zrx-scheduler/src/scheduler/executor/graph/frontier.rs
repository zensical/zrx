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

//! Frontier.

use zrx_graph::{Topology, Traversal};

use crate::scheduler::value::{Value, Values};

use super::storage::Storage;
use super::Node;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Frontier.
///
/// This data type manages a topological traversal of a directed acyclic graph
/// (DAG), together with the storage of values that are passed from one action
/// to the next in topological order. Frontiers don't necessarily traverse the
/// entire graph, but can also start at internal nodes.
///
/// The storage only contains values that are still needed, i.e. the values of
/// nodes that have not been visited yet. Dependencies are tracked, so that if
/// all dependencies of a node (i.e. an action) have been resolved, the value
/// is removed from the storage, as it is no longer needed.
#[derive(Debug)]
pub struct Frontier {
    /// Graph traversal.
    traversal: Traversal,
    /// Dependent counts.
    dependents: Vec<u8>,
    /// Data storage.
    storage: Storage,
    /// Visitable nodes.
    visitable: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Frontier {
    /// Creates a frontier.
    pub fn new<I>(topology: &Topology, initial: I) -> Self
    where
        I: IntoIterator<Item = usize>,
    {
        let visitable = initial.into_iter().collect::<Vec<_>>();
        let traversal = Traversal::new(topology, visitable.clone());

        // Obtain outgoing edges and distance matrix
        let outgoing = traversal.topology().outgoing();
        let distance = traversal.topology().distance();

        // We need to store intermediate results to pass them to dependents once
        // all of their dependencies have been resolved. This is exactly why we
        // track the number of dependents of each node, which is the number of
        // outgoing edges for that node.
        let mut dependents = outgoing.degrees().to_vec();
        for node in outgoing {
            // We must adjust the dependent count of each node's dependencies
            // if it's not reachable from any of the initial nodes
            if !visitable.iter().any(|&n| distance[n][node] != u8::MAX) {
                // Obtain adjacency list of outgoing edges, and decrement the
                // number of unresolved dependencies for each dependent by one
                let incoming = topology.incoming();
                for &dependency in &incoming[node] {
                    dependents[dependency] -= 1;
                }
            }
        }

        // Return frontier
        Self {
            traversal,
            dependents,
            storage: Storage::new(),
            visitable: 0,
        }
    }

    /// Returns the next visitable node.
    #[inline]
    #[must_use]
    pub fn take(&mut self) -> Option<usize> {
        self.traversal.take().inspect(|_| self.visitable += 1)
    }

    /// Marks the given node as visited and stores its value, if any.
    pub fn complete(
        &mut self, node: Node<Option<Box<dyn Value>>>,
    ) -> Result<(), Option<Box<dyn Value>>> {
        match self.traversal.complete(node.id) {
            Ok(()) => {}
            Err(_) => return Err(node.data),
        }
        self.visitable -= 1;

        // In case the given node has dependents and contains a value, we can
        // just append it to the storage, so it's available to its dependents.
        // We can append the node without further checks, as we know that we
        // haven't seen it yet, as the traversal completed successfully.
        if self.dependents[node.id] != 0 {
            if let Some(data) = node.data {
                self.storage.append(node.id, data);
            }
        }

        // Obtain adjacency list of incoming edges, and decrement the number
        // of unresolved dependents for each dependency by one. When the number
        // of dependents for a dependency reaches zero, it can be removed from
        // storage, since it's no longer needed.
        let incoming = self.traversal.topology().incoming();
        for &dependency in &incoming[node.id] {
            self.dependents[dependency] -= 1;

            // We satisfied all dependents, so the dependency is removed
            if self.dependents[dependency] == 0 {
                self.storage.remove(dependency);
            }
        }

        // No errors occurred.
        Ok(())
    }

    /// Selects the node with the given identifier and returns its values.
    pub fn select(&mut self, id: usize) -> Node<Values<'_>> {
        let incoming = self.traversal.topology().incoming();
        let view = self.storage.select(&incoming[id]);
        Node { id, data: Values::View(view) }
    }
}

#[allow(clippy::must_use_candidate)]
impl Frontier {
    /// Returns the number of visitable nodes.
    #[inline]
    pub fn len(&self) -> usize {
        self.visitable + self.traversal.len()
    }

    /// Returns whether there are any visitable nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
