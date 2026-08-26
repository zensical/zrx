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

use std::ops::{Index, Range};

use super::Graph;
use super::error::{Error, Result};
use super::topology::Topology;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Graph builder.
#[derive(Clone, Debug)]
pub struct Builder<T> {
    /// Nodes of the graph.
    nodes: Vec<T>,
    /// Edges of the graph.
    edges: Vec<Edge>,
}

/// Graph edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge {
    /// Source node index.
    pub source: usize,
    /// Target node index.
    pub target: usize,
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
    /// builder.add_edge(a, b)?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn builder() -> Builder<T> {
        Builder::default()
    }
}

// ----------------------------------------------------------------------------

impl<T> Builder<T> {
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
    /// # builder.add_edge(a, b)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_node(&mut self, data: T) -> usize {
        self.nodes.push(data);
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_edge(&mut self, source: usize, target: usize) -> Result {
        if source >= self.nodes.len() {
            return Err(Error::NotFound(source));
        }
        if target >= self.nodes.len() {
            return Err(Error::NotFound(target));
        }

        // Add edge, as both nodes were found
        self.edges.push(Edge { source, target });
        Ok(())
    }

    /// Creates an iterator over the graph builder.
    ///
    /// This iterator emits the node indices, which is exactly the same as
    /// iterating over the adjacency list using `0..self.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::Graph;
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create iterator over graph builder
    /// for node in builder.iter() {
    ///     println!("{node:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Range<usize> {
        0..self.nodes.len()
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
    /// builder.add_edge(a, b)?;
    /// builder.add_edge(b, c)?;
    ///
    /// // Create graph from builder
    /// let graph = builder.build();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn build(self) -> Graph<T> {
        let topology = Topology::new(self.nodes.len(), &self.edges);
        Graph { data: self.nodes, topology }
    }
}

#[allow(clippy::must_use_candidate)]
impl<T> Builder<T> {
    /// Returns a reference to the nodes.
    #[inline]
    pub fn nodes(&self) -> &[T] {
        &self.nodes
    }

    /// Returns a reference to the edges.
    #[inline]
    pub fn edges(&self) -> &[Edge] {
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

impl<T> Index<usize> for Builder<T> {
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
    /// use zrx_graph::Graph;
    /// use zrx_graph::topology::Adjacency;
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

impl<T> IntoIterator for &Builder<T> {
    type Item = usize;
    type IntoIter = Range<usize>;

    /// Creates an iterator over the graph builder.
    ///
    /// This iterator emits the node indices, which is exactly the same as
    /// iterating over the adjacency list using `0..self.len()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_graph::Graph;
    /// use zrx_graph::topology::Adjacency;
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
    /// // Create iterator over graph builder
    /// for node in &builder {
    ///     println!("{node:?}");
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

impl<T> Default for Builder<T> {
    /// Creates a graph builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_graph::Builder;
    ///
    /// // Create graph builder
    /// let mut builder = Builder::default();
    /// # let _: Builder<()> = builder;
    /// ```
    #[inline]
    fn default() -> Self {
        Self {
            nodes: Vec::default(),
            edges: Vec::default(),
        }
    }
}
