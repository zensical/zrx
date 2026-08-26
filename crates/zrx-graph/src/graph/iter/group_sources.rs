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

//! Iterator over sources of each key-group.

use std::collections::btree_map::{self, BTreeMap};

use crate::graph::Graph;
use crate::graph::topology::{Topology, Transitive};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over sources of each key-group.
pub struct GroupSources<'a, K> {
    /// Graph topology.
    topology: &'a Topology<Transitive>,
    /// Iterator over groups.
    inner: btree_map::IntoIter<K, Vec<usize>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T, Transitive> {
    /// Creates an iterator over the sources of each key-group.
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
    /// let graph = builder.build().into_transitive();
    ///
    /// // Create iterator over sources of key-groups
    /// for (key, nodes) in graph.group_sources(|node| node.len()) {
    ///     println!("{key}: {nodes:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn group_sources<'a, F, K>(&'a self, f: F) -> GroupSources<'a, K>
    where
        F: Fn(&'a T) -> K,
        K: Ord,
    {
        let mut groups: BTreeMap<K, Vec<usize>> = BTreeMap::new();
        for node in self {
            groups.entry(f(&self[node])).or_default().push(node);
        }
        GroupSources {
            topology: &self.topology,
            inner: groups.into_iter(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<K> Iterator for GroupSources<'_, K> {
    type Item = (K, Vec<usize>);

    /// Returns the next key-group.
    fn next(&mut self) -> Option<Self::Item> {
        let (key, nodes) = self.inner.next()?;
        let iter = nodes.iter().copied().filter(|&node| {
            !nodes.iter().any(|&ancestor| {
                node != ancestor && self.topology.has_path(ancestor, node)
            })
        });

        // Return key-group
        Some((key, iter.collect()))
    }
}
