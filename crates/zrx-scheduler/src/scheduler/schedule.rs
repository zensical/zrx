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

//! Schedule.

use std::collections::VecDeque;

use zrx_storage::Storages;

use crate::action::options::{Event, Interest};
use crate::scheduler::action::graph::{self, Graph};
use crate::scheduler::action::{context, Context, Result};
use crate::scheduler::signal::{Id, Scope};
use crate::scheduler::step::{Scoped, Steps};

pub mod builder;
mod frontier;
mod shared;

pub use builder::{Builder, Subscriber};
use frontier::{Frontier, Frontiers};
pub use shared::Shared;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Schedule.
#[derive(Debug)]
pub struct Schedule<I> {
    /// Action graph.
    pub graph: Graph<I>,
    /// Action storage set.
    pub storages: Storages,
    /// Frontier set.
    frontiers: Frontiers<I>,
    /// Frontier queues.
    queues: Vec<VecDeque<usize>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Schedule<I>
where
    I: Id,
{
    /// Submits a node.
    #[allow(clippy::missing_panics_doc)]
    pub fn submit(&mut self, node: usize, scope: &Scope<I>) -> usize {
        let traversal = self.graph.traverse([node]);

        // Create and insert frontier
        let frontier = Frontier::new(scope.clone(), traversal);
        let f = self.frontiers.insert(frontier).expect("invariant");

        // Notify interested nodes about new frontier
        for n in &self.graph {
            let options = self.graph[n].options();
            if options.interests.contains(&Interest::Enter) {
                let (action, adj) = self.graph.adjacent_mut(n);
                let (inputs, output) = self.storages.views(adj.incoming, n);
                let ctx = Context::builder()
                    .inputs(inputs)
                    .output(output)
                    .events(vec![Event::Insert(scope.clone())])
                    .build([])
                    .expect("invariant");
                // Hand events to action - later, we should enqueue events, and
                // invoke actions along the way together with all queued events
                action.execute(ctx);
            }
        }

        // Consume current node, and return frontier identifier
        let frontier = &mut self.frontiers[f];
        frontier.take().expect("invariant");
        f
    }

    /// Executes the given node with the provided context.
    #[allow(clippy::missing_panics_doc)]
    pub fn take(&mut self, node: usize) -> Vec<Result<Steps<I>>> {
        let scopes: Vec<_> = self.queues[node]
            .drain(..)
            .map(|f| Scoped::new(self.frontiers[f].scope().clone(), f))
            .collect();

        // Obtain action and input/output views for the node
        let (action, adj) = self.graph.adjacent_mut(node);
        let (inputs, output) = self.storages.views(adj.incoming, node);

        // Create action context
        let ctx = Context::builder()
            .inputs(inputs)
            .output(output)
            .build(scopes)
            .expect("invariant");

        // Execute action and collect results
        action.execute(ctx).collect()
    }

    /// Completes the given node.
    #[allow(clippy::missing_panics_doc)]
    pub fn complete(&mut self, node: usize, scoped: &Scoped<I>) -> Vec<usize> {
        let f = scoped.id().expect("invariant");

        // In case the fronter is empty after completion, remove it
        let res = self.frontiers[f].complete(node);
        if res.is_ok() && self.frontiers[f].is_empty() {
            self.frontiers.remove(f);
            return vec![];
        }

        // Otherwise obtain new nodes from frontier, and queue them
        let mut nodes = vec![];
        while let Some(next) = self.frontiers[f].take() {
            if self.queues[next].is_empty() {
                nodes.push(next);
            }

            // Here, we might consider using a bitmap for a faster inclusion
            // check while keeping the original order of nodes
            if !self.queues[next].contains(&f) {
                self.queues[next].push_back(f);
            }
        }
        nodes
    }

    /// Creates a context builder for the given node.
    pub fn context(&mut self, node: usize) -> context::Builder<'_, I> {
        let (_, adj) = self.graph.adjacent_mut(node);
        let (inputs, output) = self.storages.views(adj.incoming, node);
        Context::builder().inputs(inputs).output(output)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Schedule<I> {
    /// Returns the number of frontiers.
    #[inline]
    pub fn len(&self) -> usize {
        self.frontiers.len()
    }

    /// Returns whether there are any frontiers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.frontiers.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Default for Schedule<I> {
    /// Creates an empty schedule.
    ///
    /// While an empty schedule is not very useful, it's sometimes practical as
    /// a placeholder in documentation, and in types implementing [`Default`].
    #[inline]
    fn default() -> Self {
        Self::builder().build()
    }
}
