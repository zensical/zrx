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

//! Immutable, statically validated scheduler plan.

use ahash::HashMap;
use std::collections::BTreeSet;
use thiserror::Error;

use zrx_graph::Graph;

use crate::scheduler::Id;

use super::action::{Job, Port};

mod binding;
mod builder;
mod progress;
pub(in crate::scheduler) use binding::Destination;
pub(in crate::scheduler) use binding::InputIndex;
pub use binding::{
    InputBinding, InputError, InputId, OutputBinding, OutputError, OutputId,
    Route,
};
pub use builder::Builder;
pub use progress::ProgressError;
pub(in crate::scheduler) use progress::{
    Progress, ProgressIndex, ProgressNode,
};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid static route graph.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RouteError {
    /// Jobs and route lists have different lengths.
    #[error("jobs and route lists have different lengths")]
    Count,
    /// One route names a missing node.
    #[error("route from node {from} names missing node {target}")]
    Node { from: usize, target: usize },
    /// One route names a missing input lane.
    #[error(
        "route from node {from} names missing lane {lane} on node {target}"
    )]
    Lane {
        from: usize,
        target: usize,
        lane: usize,
    },
    /// Connected ports carry different keyed value types.
    #[error(
        "route from node {from} has a type mismatch on node {target} lane {lane}"
    )]
    Port {
        from: usize,
        target: usize,
        lane: usize,
    },
    /// One source declares the same target lane more than once.
    #[error("duplicate route from node {from} to node {target} lane {lane}")]
    Duplicate {
        from: usize,
        target: usize,
        lane: usize,
    },
    /// The installed action graph contains a cycle.
    #[error("action routes contain a cycle")]
    Cycle,
}

// ----------------------------------------------------------------------------

/// Invalid statically lowered plan.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PlanError {
    /// The static route graph is invalid.
    #[error(transparent)]
    Route(#[from] RouteError),
    /// An external input binding is invalid.
    #[error(transparent)]
    Input(#[from] InputError),
    /// An external output binding is invalid.
    #[error(transparent)]
    Output(#[from] OutputError),
    /// A shared revision-progress overlay is invalid.
    #[error(transparent)]
    Progress(#[from] ProgressError),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

pub(super) struct NodePlan {
    pub(super) inputs: Box<[Port]>,
    pub(super) destinations: Box<[Destination]>,
}

// ----------------------------------------------------------------------------

/// Immutable graph topology, external bindings, progress, and installed jobs.
pub struct Plan<I>
where
    I: Id,
{
    pub(super) graph: Graph<NodePlan>,
    pub(super) jobs: Vec<Job<I>>,
    pub(super) inputs: Vec<InputBinding>,
    pub(super) inputs_by_id: HashMap<InputId, InputIndex>,
    pub(super) outputs: Vec<OutputBinding>,
    pub(super) progress: Vec<Progress>,
    pub(super) progress_by_input: Vec<Option<ProgressIndex>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Plan<I>
where
    I: Id,
{
    /// Starts construction of one complete inactive plan.
    pub fn builder(jobs: Vec<Job<I>>, routes: Vec<Vec<Route>>) -> Builder<I> {
        Builder::new(jobs, routes)
    }

    fn compile(
        jobs: Vec<Job<I>>, routes: Vec<Vec<Route>>,
    ) -> Result<Self, PlanError> {
        if jobs.len() != routes.len() {
            return Err(RouteError::Count.into());
        }
        for (source, targets) in routes.iter().enumerate() {
            let mut unique = BTreeSet::new();
            for route in targets {
                if !unique.insert(*route) {
                    return Err(RouteError::Duplicate {
                        from: source,
                        target: route.node,
                        lane: route.lane,
                    }
                    .into());
                }
                let Some(target) = jobs.get(route.node) else {
                    return Err(RouteError::Node {
                        from: source,
                        target: route.node,
                    }
                    .into());
                };
                let Some(&port) = target.inputs().get(route.lane) else {
                    return Err(RouteError::Lane {
                        from: source,
                        target: route.node,
                        lane: route.lane,
                    }
                    .into());
                };
                if jobs[source].output() != port {
                    return Err(RouteError::Port {
                        from: source,
                        target: route.node,
                        lane: route.lane,
                    }
                    .into());
                }
            }
        }
        let mut edges = Vec::new();
        for (source, targets) in routes.iter().enumerate() {
            let mut unique = BTreeSet::new();
            for route in targets {
                if unique.insert(route.node) {
                    edges.push((source, route.node));
                }
            }
        }
        let mut graph = Graph::builder();
        for (job, targets) in jobs.iter().zip(routes) {
            graph.add_node(NodePlan {
                inputs: job.inputs().into(),
                destinations: targets
                    .into_iter()
                    .map(Destination::Route)
                    .collect(),
            });
        }
        for (source, target) in edges {
            graph
                .add_edge(source, target)
                .map_err(|_| RouteError::Node { from: source, target })?;
        }
        let graph = graph.build();
        if !graph.is_acyclic() {
            return Err(RouteError::Cycle.into());
        }
        Ok(Self {
            graph,
            jobs,
            inputs: Vec::new(),
            inputs_by_id: HashMap::default(),
            outputs: Vec::new(),
            progress: Vec::new(),
            progress_by_input: Vec::new(),
        })
    }

    /// Validates and installs external output bindings.
    ///
    /// # Errors
    ///
    /// Returns the first duplicate or invalid output binding.
    fn install_outputs(
        mut self, outputs: Vec<OutputBinding>,
    ) -> Result<Self, OutputError> {
        let mut installed = HashMap::default();
        let mut bindings = Vec::with_capacity(outputs.len());
        let mut by_node = vec![Vec::new(); self.jobs.len()];
        for output in outputs {
            let valid = self
                .jobs
                .get(output.source)
                .is_some_and(|job| job.output() == output.port);
            if !valid {
                return Err(OutputError::Invalid(output.id));
            }
            let id = output.id;
            let source = output.source;
            let index = bindings.len();
            if installed.insert(id, index).is_some() {
                return Err(OutputError::Duplicate(id));
            }
            bindings.push(output);
            by_node[source].push(Destination::Output(index));
        }
        for (node, outputs) in by_node.into_iter().enumerate() {
            let mut destinations: Vec<_> = self.graph[node]
                .destinations
                .iter()
                .copied()
                .filter(|destination| destination.route().is_some())
                .collect();
            destinations.extend(outputs);
            self.graph[node].destinations = destinations.into_boxed_slice();
        }
        self.outputs = bindings;
        Ok(self)
    }

    /// Validates and installs external graph-positioned inputs.
    ///
    /// # Errors
    ///
    /// Returns the first duplicate or invalid input binding.
    fn install_inputs(
        mut self, inputs: Vec<InputBinding>,
    ) -> Result<Self, InputError> {
        for input in inputs {
            let valid = self
                .graph
                .as_ref()
                .get(input.route.node)
                .and_then(|node| node.inputs.get(input.route.lane))
                .is_some_and(|port| *port == input.port);
            if !valid {
                return Err(InputError::Invalid(input.id));
            }
            let id = input.id;
            let index = InputIndex::new(self.inputs.len());
            if self.inputs_by_id.insert(id, index).is_some() {
                return Err(InputError::Duplicate(id));
            }
            self.inputs.push(input);
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use crate::scheduler::action::{Action, Context, Job};

    use super::{InputBinding, InputId, Plan, Route};

    struct Source;

    impl Action<u64> for Source {
        type Inputs = (u64,);
        type Output = u64;

        fn execute(&mut self, _context: Context<'_, u64, Self>) {}
    }

    struct Pair;

    struct Terminal;

    struct Variadic;

    impl Action<u64> for Pair {
        type Inputs = (u64, u64);
        type Output = u64;

        fn execute(&mut self, _context: Context<'_, u64, Self>) {}
    }

    impl Action<u64> for Terminal {
        type Inputs = (u64,);
        type Output = u64;

        fn execute(&mut self, _context: Context<'_, u64, Self>) {}
    }

    impl Action<u64> for Variadic {
        type Inputs = Vec<u64>;
        type Output = u64;

        fn execute(&mut self, _context: Context<'_, u64, Self>) {}
    }

    #[test]
    fn multi_lane_routes_share_one_topology_edge() {
        let plan = Plan::builder(
            vec![Job::new(Source), Job::new(Pair)],
            vec![vec![Route::new(1, 0), Route::new(1, 1)], vec![]],
        )
        .build()
        .unwrap();

        assert_eq!(plan.graph.adjacent(0).1.outgoing, &[1]);
        assert_eq!(plan.graph.adjacent(1).1.incoming, &[0]);
    }

    #[test]
    fn variadic_input_arity_is_resolved_from_routes() {
        let plan = Plan::builder(
            vec![Job::new(Source), Job::new(Variadic)],
            vec![(0..9).map(|lane| Route::new(1, lane)).collect(), vec![]],
        )
        .build()
        .unwrap();

        assert_eq!(plan.jobs[1].inputs().len(), 9);
        assert_eq!(plan.graph[1].inputs.len(), 9);
    }

    #[test]
    fn variadic_input_arity_is_resolved_from_external_bindings() {
        let plan = Plan::builder(vec![Job::new(Variadic)], vec![vec![]])
            .inputs(vec![InputBinding::new::<u64, u64>(
                InputId::new(1),
                Route::new(0, 8),
            )])
            .build()
            .unwrap();

        assert_eq!(plan.jobs[0].inputs().len(), 9);
        assert_eq!(plan.graph[0].inputs.len(), 9);
    }

    #[test]
    fn terminal_subscribers_share_one_input_progress_overlay() {
        let input = InputId::new(1);
        let plan = Plan::builder(
            vec![
                Job::new(Source),
                Job::new(Terminal).with_progress(),
                Job::new(Terminal).with_progress(),
            ],
            vec![vec![Route::new(1, 0), Route::new(2, 0)], vec![], vec![]],
        )
        .inputs(vec![InputBinding::new::<u64, u64>(input, Route::new(0, 0))])
        .build()
        .unwrap();

        assert_eq!(plan.progress.len(), 1);
        let nodes = &plan.progress[0].nodes;
        assert_eq!(nodes[0].as_ref().unwrap().routes.len(), 2);
        assert!(!nodes[0].as_ref().unwrap().subscriber);
        assert!(nodes[1].as_ref().unwrap().subscriber);
        assert!(nodes[2].as_ref().unwrap().subscriber);
    }

    #[test]
    fn construction_owned_subscribers_share_one_progress_overlay() {
        let input = InputId::new(1);
        let plan = Plan::builder(
            vec![
                Job::new(Source),
                Job::new(Source).with_progress(),
                Job::new(Terminal).with_progress(),
            ],
            vec![vec![Route::new(1, 0), Route::new(2, 0)], vec![], vec![]],
        )
        .inputs(vec![InputBinding::new::<u64, u64>(input, Route::new(0, 0))])
        .build()
        .unwrap();

        assert_eq!(plan.progress.len(), 1);
        let nodes = &plan.progress[0].nodes;
        assert!(!nodes[0].as_ref().unwrap().subscriber);
        assert!(nodes[1].as_ref().unwrap().subscriber);
        assert!(nodes[2].as_ref().unwrap().subscriber);
    }
}
