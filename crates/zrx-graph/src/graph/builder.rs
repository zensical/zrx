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

//! Graph builder.

use std::collections::BTreeMap;
use std::ops::Index;

use super::error::{Error, Result};
use super::topology::Topology;
use super::Graph;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Graph builder.
#[derive(Clone, Debug)]
pub struct Builder<T, W = ()> {
    /// Nodes of the graph.
    nodes: Vec<T>,
    /// Edges of the graph.
    edges: Vec<Edge<W>>,
}

/// Graph edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge<W = ()> {
    /// Source node index.
    pub source: usize,
    /// Target node index.
    pub target: usize,
    /// Weight.
    pub weight: W,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Graph<T> {
    /// Creates a graph builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::Graph;
    ///
    /// // Create graph builder
    /// let mut builder = Graph::builder();
    /// let a = builder.add_node("a");
    /// let b = builder.add_node("b");
    ///
    /// // Create edges between nodes
    /// builder.add_edge(a, b, 0)?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn builder<W>() -> Builder<T, W>
    where
        W: Clone,
    {
        Builder {
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }
}

// ----------------------------------------------------------------------------

impl<T, W> Builder<T, W> {
    /// Adds a node to the graph.
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
    /// #
    /// # // Create edges between nodes
    /// # builder.add_edge(a, b, 0)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_node(&mut self, node: T) -> usize {
        self.nodes.push(node);
        self.nodes.len() - 1
    }

    /// Adds an edge to the graph.
    ///
    /// # Errors
    ///
    /// In case the source or target node doesn't exist, [`Error::NotFound`] is
    /// returned, to make sure the graph does not contain stale node references.
    /// By returning an error instead of panicking, we can provide recoverable
    /// and proper error handling to the caller.
    ///
    /// This is mentionable, as some other graph libraries will just panic and
    /// crash the program, like the popular [`petgraph`][] crate. Additionally,
    /// note that this method does not check whether an edge already exists, as
    /// the existence of multiple edges is a valid use case in some scenarios.
    ///
    /// [`petgraph`]: https://docs.rs/petgraph/
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_edge(
        &mut self, source: usize, target: usize, weight: W,
    ) -> Result {
        if source >= self.nodes.len() {
            return Err(Error::NotFound(source));
        }
        if target >= self.nodes.len() {
            return Err(Error::NotFound(target));
        }

        // Add edge, as both nodes were found
        self.edges.push(Edge { source, target, weight });
        Ok(())
    }

    /// Creates the edge graph of the graph.
    ///
    /// This method derives a new graph from the given graph in which each edge
    /// represents a transition from one edge to another based on their source
    /// and target nodes in the original graph, which means that the nodes of
    /// the edge graph are the edges of the original graph.
    ///
    /// Edge graphs are necessary for representing relationships between edges,
    /// which is exactly what we need for action graphs, where edges represent
    /// actions and their dependencies. During execution, we don't need to know
    /// the actual nodes, but rather the dependencies between the edges.
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
    /// // Create edge graph
    /// let edges = builder.to_edge_graph();
    /// assert_eq!(edges.nodes(), builder.edges());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn to_edge_graph(&self) -> Builder<Edge<W>>
    where
        W: Clone,
    {
        // We expect that the edges are ordered by target an weight, since the
        // former represents the corresponding action, and the latter the index
        // of the argument in the action. This is also why we index sources by
        // targets and not the other way around, i.e., to keep the ordering.
        let mut targets: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (source, edge) in self.edges.iter().enumerate() {
            targets.entry(edge.target).or_default().push(source);
        }

        // Enumerate all sources for each target and create the edges between
        // them in order to create the edge graph. The new edges don't receive
        // a weight, since the original edges are now the nodes, and there's no
        // other information that can't be obtained from the original graph.
        let mut edges = Vec::with_capacity(targets.len());
        for (target, edge) in self.edges.iter().enumerate() {
            if let Some(sources) = targets.get(&edge.source) {
                for &source in sources {
                    edges.push(Edge { source, target, weight: () });
                }
            }
        }

        // Return edge graph builder
        Builder {
            nodes: self.edges.clone(),
            edges,
        }
    }

    /// Builds the graph.
    ///
    /// This method creates the actual graph from the builder, which brings the
    /// graph into an executable form to allow for very efficient traversal.
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
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn build(self) -> Graph<T>
    where
        W: Clone,
    {
        Graph {
            topology: Topology::new(&self),
            data: self.nodes,
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<T, W> Builder<T, W> {
    /// Returns a reference to the nodes.
    #[inline]
    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }

    /// Returns a reference to the edges.
    #[inline]
    pub fn edges(&self) -> &[Edge<W>] {
        &self.edges
    }

    /// Returns the number of nodes.
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether there are any nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T, W> Index<usize> for Builder<T, W> {
    type Output = T;

    /// Returns a reference to the node at the index.
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
    /// // Obtain references to nodes
    /// assert_eq!(&builder[a], &"a");
    /// assert_eq!(&builder[b], &"b");
    /// assert_eq!(&builder[c], &"c");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.nodes[index]
    }
}

// ----------------------------------------------------------------------------

impl<T, W> Default for Builder<T, W>
where
    W: Clone,
{
    /// Creates a graph builder.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::Builder;
    ///
    /// // Create graph builder
    /// let mut builder = Builder::default();
    /// # let a = builder.add_node("a");
    /// # let b = builder.add_node("b");
    /// #
    /// # // Create edges between nodes
    /// # builder.add_edge(a, b, 0)?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn default() -> Self {
        Graph::builder()
    }
}
