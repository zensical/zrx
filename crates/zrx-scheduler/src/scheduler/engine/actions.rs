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

use zrx_graph::Graph;
use zrx_store::StoreMutRef;

use crate::scheduler::action::Result;
use crate::scheduler::engine::queue::Token;
use crate::scheduler::schedule::Schedule;
use crate::scheduler::signal::Id;
use crate::scheduler::step::effect::Then;
use crate::scheduler::step::{Scoped, Steps};

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

    /// Ensure a frontier exists for the given token and scoped value.
    pub fn ensure(&mut self, token: Token, scoped: &Scoped<I>) -> Scoped<I> {
        let schedule = &mut self.schedules[token.module];
        match scoped.id() {
            Some(_) => scoped.clone(),
            None => Scoped::new(
                (**scoped).clone(),
                schedule.submit(token.node, scoped),
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

    /// Completes a schedule with the given token and scoped value.
    pub fn complete(&mut self, token: Token, scope: &Scoped<I>) {
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
        let mut modules = Graph::builder();
        for (m, _) in &self.schedules {
            modules.add_node(m);

            // Update dependencies between modules
            let g = &self.schedules[m].graph;
            for n in &modules {
                let h = &self.schedules[n].graph;

                // Compute dependencies from new module to existing modules
                for source in g.sources() {
                    for target in h {
                        if m == n && source == target || h.is_source(target) {
                            continue;
                        }

                        // Ensure types match, otherwise we have no dependency
                        if g[source].descriptor() != h[target].descriptor() {
                            continue;
                        }

                        // Add dependency
                        self.dependencies
                            .get_or_insert_default(&Token {
                                module: n,
                                node: target,
                            })
                            .push(Token { module: m, node: source });
                    }
                }

                // Compute dependencies from existing modules to new module
                for source in h.sources() {
                    for target in g {
                        if m == n && source == target || g.is_source(target) {
                            continue;
                        }

                        // Ensure types match, otherwise we have no dependency
                        if h[source].descriptor() != g[target].descriptor() {
                            continue;
                        }

                        // Add dependency
                        self.dependencies
                            .get_or_insert_default(&Token {
                                module: m,
                                node: target,
                            })
                            .push(Token { module: n, node: source });
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
