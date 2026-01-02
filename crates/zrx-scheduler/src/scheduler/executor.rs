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

//! Executor.

use ahash::HashMap;
use slab::Slab;
use std::collections::VecDeque;

use zrx_graph::Graph;

use super::action::output::OutputItem;
use super::action::{Action, Error, Input, Outputs, Result};
use super::effect::{Item, Signal};
use super::executor::graph::{Frontier, Node};
use super::id::Id;
use super::value::{self, Value};
use crate::action::descriptor::{Interest, Property};

pub mod graph;
pub mod queue;

pub use queue::{ToReceiver, Token};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Executor.
#[derive(Debug)]
pub struct Executor<I>
where
    I: Id,
{
    /// Executor graph.
    graph: Graph<Box<dyn Action<I>>>,
    /// Action queues.
    queues: Vec<VecDeque<usize>>,
    /// Frontier collection.
    frontiers: Slab<Entry<I>>,
    /// Interests.
    interests: HashMap<Interest, Vec<usize>>,
    /// Task concurrency.
    concurrency: Vec<usize>,
}

#[derive(Debug)]
struct Entry<I> {
    id: I,
    frontier: Option<Frontier>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

// should we really call this executor?
impl<I> Executor<I>
where
    I: Id,
{
    /// Creates a frontier engine.
    pub fn new(graph: Graph<Box<dyn Action<I>>>) -> Self {
        let mut interests = HashMap::default();
        for n in &graph {
            for interest in graph[n].descriptor().interests() {
                interests.entry(*interest).or_insert_with(Vec::new).push(n);
            }
        }
        let len = graph.len();
        Self {
            queues: vec![VecDeque::new(); graph.len()],
            graph,
            frontiers: Slab::default(),
            interests,
            concurrency: vec![0; len],
        }
    }

    // Submit a new item to the executor
    pub fn submit<T>(&mut self, item: OutputItem<I>, initial: T)
    where
        T: IntoIterator<Item = usize>,
    {
        let initial = initial.into_iter().collect::<Vec<_>>();
        let mut frontier =
            Frontier::new(self.graph.topology(), initial.clone());
        let n = frontier.take().unwrap();
        frontier.complete(Node { id: n, data: item.data }).unwrap();

        // Insert new frontier
        let id = item.id;
        let f = self.frontiers.insert(Entry {
            id: id.clone(),
            frontier: Some(frontier),
        });

        // Add initial nodes to the queue
        self.do_take(Token { frontier: f, node: n });

        // Notify interested parties about new submission
        if let Some(indices) = self.interests.get(&Interest::Submit) {
            for &node in indices {
                let action = &mut self.graph[node];
                let _ = action.execute(Input::Signal(Signal::Submit(&id)));
            }
        }
    }

    // Update the frontier with new items
    pub fn update(&mut self, token: Token, items: Vec<OutputItem<I>>) {
        let mut f = token.frontier;
        let mut completed = false;

        // Update concurrency
        self.concurrency[token.node] -= 1;

        // Traverse all items, as we need to check if we received an items for
        // the current frontier, or not. If we did not, we complete the frontier
        // without a value. We also might have received items with new ids,
        // which means we just clone or merge the frontier.
        for item in items {
            let mut n = token.node;
            if !self.frontiers.contains(f) {
                let mut frontier =
                    Frontier::new(self.graph.topology(), [token.node]);
                n = frontier.take().unwrap();
                f = self.frontiers.insert(Entry {
                    id: item.id.clone(),
                    frontier: Some(frontier),
                });
            }

            // Check if the item matches the current frontier - if it does, we
            // complete it normally, otherwise we need to clone the frontier
            let entry = &mut self.frontiers[f];
            let f = if entry.id == item.id {
                completed = true;
                f
            } else {
                let mut frontier =
                    Frontier::new(self.graph.topology(), [token.node]);
                n = frontier.take().unwrap();
                self.frontiers.insert(Entry {
                    id: item.id.clone(),
                    frontier: Some(frontier),
                })
            };
            self.do_complete(Token { frontier: f, node: n }, item.data);
        }

        // Otherwise complete the frontier without a value.
        if !completed {
            self.do_complete(Token { frontier: f, node: token.node }, None);
        }
    }

    // take: scheduling hint?
    // for now, return a vec of jobs here.
    pub fn take(&mut self) -> Vec<Job<I>> {
        // For now, we just naively find the first action that has frontiers to
        // process and process them all, done. Later, we should classify nodes
        // by their type, i.e., whether they are tasks, timers, or just run on
        // the main thread, so we can prioritize them accordingly.
        let mut opt = None;
        let mut max = 8;
        for (n, queue) in self.queues.iter_mut().enumerate() {
            let value = self.graph[n]
                .descriptor()
                .properties()
                .iter()
                .find_map(|p| {
                    if let Property::Concurrency(val) = p {
                        Some(*val)
                    } else {
                        None
                    }
                })
                .unwrap_or(8);
            if !queue.is_empty() && self.concurrency[n] < value {
                opt = Some(n);
                max = value;
                break;
            }
        }

        // No action could be processed
        let Some(n) = opt else { return Vec::new() };

        // Process jobs for the selected action
        let mut results = Vec::new();
        while let Some(f) = self.queues[n].pop_front() {
            let entry = &mut self.frontiers[f];
            let Some(frontier) = &mut entry.frontier else {
                continue;
            };

            // Obtain values for action execution
            let values = frontier.select(n);
            let id = values.id;

            // Execute action
            let input = Input::Item(Item::new(&entry.id, values.data));
            let (data, res) = match self.graph[n].execute(input) {
                Err(Error::Value(value::Error::Presence)) => (Some(None), None),
                res => (None, Some(res)),
            };

            // Handle completion immediately if we got a presence error
            if let Some(data) = data {
                self.do_complete(Token { frontier: f, node: n }, data);
                continue;
            }
            self.concurrency[id] += 1;
            results.push((Token { frontier: f, node: id }, res.unwrap()));
            max -= 1;
            if max == 0 {
                break;
            }
        }
        results
    }

    /// Completes the given frontier with the given data, if any.
    fn do_complete(&mut self, token: Token, data: Option<Box<dyn Value>>) {
        let f = token.frontier;
        let mut n = token.node;

        // Obtain frontier and identifier
        let entry = &mut self.frontiers[f];
        let id = entry.id.clone();

        // Handle completion
        let mut to_add = Vec::new();
        if let Some(frontier) = &mut entry.frontier {
            if let Err(data) = frontier.complete(Node { id: n, data }) {
                let mut frontier =
                    Frontier::new(self.graph.topology(), [token.node]);
                n = frontier.take().unwrap();
                frontier.complete(Node { id: n, data }).unwrap();
                to_add.push(frontier);

                // If the descriptor should flush, remove the frontier
                if self.graph[n]
                    .descriptor()
                    .properties()
                    .contains(&Property::Flush)
                {
                    entry.frontier = None;
                }
            }
            self.do_take(token);
        }

        // Add new frontiers if any were created
        if !to_add.is_empty() {
            let frontier = to_add.pop().unwrap();
            let f = self.frontiers.insert(Entry {
                id: id.clone(),
                frontier: Some(frontier),
            });
            self.do_take(Token { frontier: f, node: n });
        }
    }

    /// Takes the next node from the frontier and adds it to the queue.
    fn do_take(&mut self, token: Token) {
        let f = token.frontier;
        let entry = &mut self.frontiers[f];
        if let Some(frontier) = &mut entry.frontier {
            if frontier.is_empty() {
                self.frontiers.remove(f);
            } else {
                while let Some(node) = frontier.take() {
                    self.queues[node].push_back(f);
                }
            }
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Executor<I>
where
    I: Id,
{
    /// Returns the number of frontiers.
    #[inline]
    pub fn len(&self) -> usize {
        self.frontiers.len()
    }

    /// Returns whether there are any frontiers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.frontiers.is_empty()
            || self.frontiers.iter().all(|(_, x)| x.frontier.is_none())
    }

    // The executor can make progress if there are any frontiers in the queue.
    pub fn can_make_progress(&self) -> bool {
        let mut len_of_all = 0;
        for x in &self.queues {
            len_of_all += x.len();
        }
        len_of_all > 0
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Executor job.
pub type Job<I> = (Token, Result<Outputs<I>>);
