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

//! Action queue.

use slab::Slab;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;
use std::ops::{Index, IndexMut};

use zrx_store::StoreMutRef;

use crate::scheduler::action::graph::Node;
use crate::scheduler::action::Result;
use crate::scheduler::engine::queue::Token;
use crate::scheduler::schedule::Schedule;
use crate::scheduler::signal::Id;
use crate::scheduler::step::effect::Then;
use crate::scheduler::step::{Scope, Steps};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Action queue.
#[derive(Debug)]
pub struct Actions<I> {
    /// Registered schedules.
    schedules: Slab<Schedule<I>>,
    /// Module dependencies.
    dependencies: BTreeMap<Token, Vec<Token>>,
    /// Queue of schedules.
    queue: VecDeque<Token>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Actions<I>
where
    I: Id,
{
    /// Creates an action queue.
    pub fn new() -> Self {
        Self {
            schedules: Slab::new(),
            dependencies: BTreeMap::new(),
            queue: VecDeque::new(),
        }
    }

    /// Attaches a schedule to the action queue.
    pub fn attach(&mut self, schedule: Schedule<I>) -> usize {
        let m = self.schedules.insert(schedule);
        self.recompute();
        m
    }

    /// Detaches a schedule from the action queue.
    pub fn detach(&mut self, index: usize) -> Option<Schedule<I>> {
        self.schedules.try_remove(index).inspect(|_| {
            self.recompute();
        })
    }

    /// Submits a token to the action queue.
    pub fn submit(&mut self, token: Token) {
        if !self.queue.contains(&token) {
            self.queue.push_back(token);
        }
    }

    /// Returns the next item from the action queue.
    pub fn take(&mut self) -> Option<Item<I>> {
        self.queue.pop_front().map(|token| {
            let schedule = &mut self.schedules[token.module];
            (token, schedule.take(token.node))
        })
    }

    /// Ensure a frontier exists for the given token and scope.
    pub fn ensure(&mut self, token: Token, scope: &Scope<I>) -> Scope<I> {
        let schedule = &mut self.schedules[token.module];
        match scope.id() {
            Some(_) => scope.clone(),
            None => Scope::new(
                (**scope).clone(),
                schedule.submit(token.node, scope),
            ),
        }
    }

    /// Resumes a schedule with the given token and continuation.
    #[inline]
    pub fn resume(&mut self, token: Token, then: Then<I>) -> Item<I> {
        let schedule = &mut self.schedules[token.module];
        (
            token,
            vec![then
                .execute(schedule.context(token.node).build([]).unwrap())
                .map_err(Into::into)],
        )
    }

    /// Completes a schedule with the given token and scope.
    pub fn complete(&mut self, token: Token, scope: &Scope<I>) {
        let schedule = &mut self.schedules[token.module];
        for node in schedule.complete(token.node, scope) {
            let token = Token { module: token.module, node };
            if !self.queue.contains(&token) {
                self.queue.push_back(token);
            }
        }

        // Notify all dependent modules
        let prior = token;
        if let Some(deps) = self.dependencies.get(&prior) {
            for token in deps {
                let (prev, next) = self
                    .schedules
                    .get2_mut(prior.module, token.module)
                    .expect("invariant");

                // Obtain graph and storages from schedule
                let graph = &mut next.graph;
                let storages = &prev.storages;

                // Obtain sender from source node in dependent module
                let source = graph[token.node].as_source_mut().unwrap();
                source.sender().forward(&storages[prior.node], scope);

                // Add to queue
                if !self.queue.contains(token) {
                    self.queue.push_back(*token);
                }
            }
        }
    }

    /// Recompute dependencies between modules.
    fn recompute(&mut self) {
        self.dependencies.clear();

        // Precompute sources and targets for each module
        let mut sources = vec![BTreeMap::new(); self.schedules.len()];
        let mut targets = vec![BTreeMap::new(); self.schedules.len()];

        for (m, schedule) in &self.schedules {
            targets[m] = schedule.graph.group_sinks(Node::descriptor).collect();
            sources[m] = schedule.graph.sources().fold(
                BTreeMap::<_, Vec<usize>>::new(),
                |mut map, n| {
                    map.entry(schedule.graph[n].descriptor())
                        .or_default()
                        .push(n);
                    map
                },
            );
        }

        // Wire modules: find all producers for each consumer type
        for (consumer, _) in &self.schedules {
            for (descriptor, nodes) in &sources[consumer] {
                for (producer, _) in &self.schedules {
                    if producer == consumer {
                        continue;
                    }

                    // Skip if producer has an external source for this type —
                    // it inherited the type rather than originating it
                    if sources[producer].contains_key(descriptor) {
                        continue;
                    }

                    // Find target nodes in producer for this type
                    let Some(exit_nodes) = targets[producer].get(descriptor)
                    else {
                        continue;
                    };

                    // Wire each exit node to each target node
                    for &exit in exit_nodes {
                        for &entry in nodes {
                            self.dependencies
                                .get_or_insert_default(&Token {
                                    module: producer,
                                    node: exit,
                                })
                                .push(Token { module: consumer, node: entry });
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Actions<I> {
    /// Returns the number of actions.
    #[inline]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Returns whether there are any actions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Index<usize> for Actions<I> {
    type Output = Schedule<I>;

    /// Returns a reference to the schedule at the given index.
    #[inline]
    fn index(&self, token: usize) -> &Self::Output {
        &self.schedules[token]
    }
}

impl<I> IndexMut<usize> for Actions<I> {
    /// Returns a mutable reference to the schedule at the given index.
    #[inline]
    fn index_mut(&mut self, token: usize) -> &mut Self::Output {
        &mut self.schedules[token]
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Actions<I>
where
    I: Id,
{
    /// Creates an action queue.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Action queue item.
pub type Item<I> = (Token, Vec<Result<Steps<I>>>);
