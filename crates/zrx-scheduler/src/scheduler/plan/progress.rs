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

//! Shared revision-progress overlays compiled from plan subscribers.

use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

use zrx_graph::Topology;
use zrx_graph::topology::Transitive;

use crate::scheduler::Id;
use crate::scheduler::action::Job;

use super::{InputId, InputIndex, Plan, Route};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid shared revision-progress overlay.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ProgressError {
    /// One relevant lane has zero or multiple progress arrivals.
    #[error(
        "progress overlay for input {input:?} has ambiguous lane {lane} on node {node}"
    )]
    Lane {
        input: InputId,
        node: usize,
        lane: usize,
    },
    /// A convergence requires a lane outside its fixed-width representation.
    ///
    /// Converged progress supports lane indices `0..=63`. A direct subscriber
    /// with only one relevant lane does not use the mask and may use a higher
    /// lane index.
    #[error(
        "progress overlay for input {input:?} converges through unsupported lane {lane} on node {node}"
    )]
    Width {
        input: InputId,
        node: usize,
        lane: usize,
    },
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Dense plan-local position of one shared progress overlay.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(in crate::scheduler) struct ProgressIndex(usize);

// ----------------------------------------------------------------------------

/// One node's convergence, transparent routes, and optional progress tap.
pub(in crate::scheduler) struct ProgressNode {
    pub(in crate::scheduler) lanes: Box<[usize]>,
    pub(in crate::scheduler) routes: Box<[Route]>,
    pub(in crate::scheduler) subscriber: bool,
}

// ----------------------------------------------------------------------------

pub(in crate::scheduler) struct Progress {
    pub(in crate::scheduler) input: InputIndex,
    pub(in crate::scheduler) nodes: Box<[Option<ProgressNode>]>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl ProgressIndex {
    pub(in crate::scheduler) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(in crate::scheduler) const fn get(self) -> usize {
        self.0
    }
}

// ----------------------------------------------------------------------------

impl<I> Plan<I>
where
    I: Id,
{
    pub(super) fn install_progress(mut self) -> Result<Self, ProgressError> {
        if !self.jobs.iter().any(Job::requires_progress) {
            self.progress_by_input = vec![None; self.inputs.len()];
            return Ok(self);
        }
        let reachability = self.graph.topology().clone().into_transitive();

        let mut plans = Vec::with_capacity(self.inputs.len());
        for input in &self.inputs {
            let subscribers = self
                .jobs
                .iter()
                .enumerate()
                .filter_map(|(node, job)| {
                    (job.requires_progress()
                        && reachability.has_path(input.route.node, node))
                    .then_some(node)
                })
                .collect::<BTreeSet<_>>();
            if !subscribers.is_empty() {
                plans.push((input.id, subscribers));
            }
        }

        let mut compiled = Vec::with_capacity(plans.len());
        let mut by_input = vec![None; self.inputs.len()];
        for (input, subscribers) in plans {
            let progress =
                self.compile_progress(&reachability, input, &subscribers)?;
            let index = ProgressIndex::new(compiled.len());
            debug_assert!(by_input[progress.input.get()].is_none());
            by_input[progress.input.get()] = Some(index);
            compiled.push(progress);
        }
        self.progress = compiled;
        self.progress_by_input = by_input;
        Ok(self)
    }

    fn compile_progress(
        &self, reachability: &Topology<Transitive>, input: InputId,
        subscribers: &BTreeSet<usize>,
    ) -> Result<Progress, ProgressError> {
        let input_index = self.inputs_by_id[&input];
        let source = self.inputs[input_index.get()].route;

        let relevant = (0..self.jobs.len())
            .map(|node| {
                reachability.has_path(source.node, node)
                    && subscribers.iter().any(|&subscriber| {
                        reachability.has_path(node, subscriber)
                    })
            })
            .collect::<Vec<_>>();
        let mut lanes = vec![BTreeSet::new(); self.jobs.len()];
        let mut arrivals = BTreeMap::new();
        lanes[source.node].insert(source.lane);
        *arrivals.entry(source).or_default() += 1;
        for node in 0..self.jobs.len() {
            if !relevant[node] {
                continue;
            }
            for route in self.graph[node]
                .destinations
                .iter()
                .filter_map(|destination| destination.route())
                .filter(|route| relevant[route.node])
            {
                lanes[route.node].insert(route.lane);
                *arrivals.entry(route).or_default() += 1;
            }
        }

        let mut nodes = Vec::with_capacity(self.jobs.len());
        for node in 0..self.jobs.len() {
            if !relevant[node] {
                nodes.push(None);
                continue;
            }
            for &lane in &lanes[node] {
                if lanes[node].len() > 1 && lane >= u64::BITS as usize {
                    return Err(ProgressError::Width { input, node, lane });
                }
                if arrivals.get(&Route::new(node, lane)) != Some(&1) {
                    return Err(ProgressError::Lane { input, node, lane });
                }
            }
            let routes = self.graph[node]
                .destinations
                .iter()
                .filter_map(|destination| destination.route())
                .filter(|route| relevant[route.node])
                .collect();
            nodes.push(Some(ProgressNode {
                lanes: lanes[node].iter().copied().collect(),
                routes,
                subscriber: subscribers.contains(&node),
            }));
        }
        Ok(Progress {
            input: input_index,
            nodes: nodes.into_boxed_slice(),
        })
    }
}
