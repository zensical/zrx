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

//! Action graph builder.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};

use zrx_graph::{self as graph};

use super::descriptor::Descriptor;
use super::{Action, Graph, Marker, Source};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Action graph builder.
#[derive(Debug, Default)]
pub struct Builder<I> {
    /// Inner graph builder.
    inner: graph::Builder<Descriptor, usize>,
    /// Actions.
    actions: Vec<Box<dyn Action<I>>>,
}

/// Action connector.
pub struct Connector<'a, I> {
    /// Graph builder.
    builder: &'a mut Builder<I>,
    /// Descriptor.
    descriptor: Descriptor,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Builder<I> {
    /// Creates a new action graph builder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: graph::Graph::builder(),
            actions: Vec::new(),
        }
    }

    /// Adds a source to the action graph.
    #[allow(clippy::missing_panics_doc)]
    #[inline]
    pub fn add_source<T>(&mut self) -> usize
    where
        T: Any,
    {
        // add a node, and then immediately attach it to a source action with
        // an id... aah this is part of the scheduler!!!!
        let from = self.inner.add_node(Descriptor::new::<T>());
        let node = self.inner.add_node(Descriptor::new::<T>());

        // Add action marker for source
        self.actions.push(Box::new(Marker));
        self.inner.add_edge(from, node, 0).expect("invariant");
        node
    }

    /// Adds an action to the action graph.
    #[inline]
    #[must_use]
    pub fn add_action<T>(&mut self) -> Connector<'_, I>
    where
        T: Any,
    {
        Connector {
            builder: self,
            descriptor: Descriptor::new::<T>(),
        }
    }

    /// Builds the action graph.
    ///
    /// This method creates the actual action graph from the builder, bringing
    /// it into an executable form. It does so by converting the graph into an
    /// edge graph, where actions are nodes, associating arguments for actions
    /// with their corresponding source nodes.
    #[allow(clippy::missing_panics_doc)]
    pub fn build(self) -> Graph<I> {
        let mut actions = Vec::with_capacity(self.inner.len());
        let mut degrees = vec![0; self.inner.len()];

        // In the graph builder of the scheduler, every edge corresponds to a
        // specific argument of an action, connecting the action to its source
        // nodes. Additionally, each action has a designated output type, i.e.,
        // its target node. Nodes store the type information between upstream
        // and downstream actions, but are themselves type-erased, meaning they
        // only retain information for efficient querying and construction.
        let mut offset = 0;
        for edge in self.inner.edges() {
            offset += edge.weight;

            // The first thing we need to know is which edges refer to the same
            // action, as the number of incoming edges per action corresponds to
            // the number of its arguments, and we must know the unique actions
            // of the graph. Moreover, we record which actions have no incoming
            // edges, as they are source nodes in the action graph, i.e., the
            // the graph we're about to construct.
            actions.push(actions.len() - offset);
            degrees[edge.target] += 1;
        }

        // Initialize the set of source nodes, which are action with no incoming
        // edges, which are the entry points to the action graph. Each source
        // node is associated with its respective action, combining sources of
        // the same type by deduplicating them through their descriptors.
        let mut sources = BTreeMap::default();
        for node in 0..self.inner.len() {
            if degrees[node] == 0 {
                sources
                    .entry(self.inner[node].clone())
                    .or_insert_with(BTreeSet::new)
                    .insert(actions[node]);
            }
        }

        // Next, we create a graph builder for actions, and add all actions as
        // nodes, i.e., the units of work the scheduler should coordinate
        let mut builder = graph::Graph::builder();
        for action in self.actions {
            builder.add_node(action);
        }

        // Then, we create the edge graph of the graph builder, which allows us
        // to inspect the dependencies between edges, and thus defined actions.
        // The edges of the edge graph are the actions, and thus what we need
        // to connect, but we must ensure that we don't add the same action
        // multiple times, since the arguments are now encoded in the ordered
        // set of dependencies between all actions.
        let edge_graph = self.inner.to_edge_graph();
        for edge in edge_graph.edges() {
            let node = &edge_graph[edge.source];

            // Add edge to graph in case we haven't added it yet. We only want
            // to add the unique actions to the graph, or we would process the
            // same action multiple times. Since every action has at least one
            // argument, we can just filter by argument index 0. If we run into
            // an error here, it denotes a bug in our implementation, as the
            // invariants must be upheld by this implementation.
            if node.weight == 0 {
                builder
                    .add_edge(actions[edge.source], actions[edge.target], ())
                    .expect("invariant");
            }
        }

        // Collect source set into a vector and return action graph
        Graph {
            actions: builder.build(),
            sources: sources
                .into_iter()
                .map(|(descriptor, actions)| Source {
                    descriptor,
                    actions: Vec::from_iter(actions),
                })
                .collect(),
        }
    }
}

impl<I> Connector<'_, I> {
    /// Connects the action to the given sources.
    pub fn with<S, A>(self, sources: S, action: A) -> usize
    where
        S: IntoIterator<Item = usize>,
        A: Action<I> + 'static,
    {
        self.builder.actions.push(Box::new(action));

        // Create target and connect to given sources - each action must have at
        // least one, but might also have multiple arguments, which are obtained
        // from the given sources. Thus, we create an edge for each given source
        // and connect it to the target node of the action.
        let target = self.builder.inner.add_node(self.descriptor);
        for (weight, source) in sources.into_iter().enumerate() {
            // Panic in case the source or target node does not exist, as this
            // denotes a bug in the graph construction of the scheduler
            self.builder
                .inner
                .add_edge(source, target, weight)
                .expect("invariant");
        }
        target
    }
}
