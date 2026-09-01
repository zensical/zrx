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

//! Complete plan construction before validation and freezing.

use crate::scheduler::Id;
use crate::scheduler::action::Job;

use super::{InputBinding, OutputBinding, Plan, PlanError, Route};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Inactive plan description that cannot be attached before validation.
#[must_use]
pub struct Builder<I>
where
    I: Id,
{
    jobs: Vec<Job<I>>,
    routes: Vec<Vec<Route>>,
    inputs: Vec<InputBinding>,
    outputs: Vec<OutputBinding>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Builder<I>
where
    I: Id,
{
    pub(super) fn new(jobs: Vec<Job<I>>, routes: Vec<Vec<Route>>) -> Self {
        Self {
            jobs,
            routes,
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Defines external graph-positioned inputs.
    pub fn inputs(mut self, inputs: Vec<InputBinding>) -> Self {
        self.inputs = inputs;
        self
    }

    /// Defines typed external outputs.
    pub fn outputs(mut self, outputs: Vec<OutputBinding>) -> Self {
        self.outputs = outputs;
        self
    }

    /// Validates and freezes the complete plan.
    ///
    /// # Errors
    ///
    /// Returns the first invalid route, binding, progress overlay, port, or cycle.
    pub fn build(mut self) -> Result<Plan<I>, PlanError> {
        if self.jobs.iter().any(Job::is_variadic) {
            self.resolve_variadic_inputs()?;
        }
        let plan = Plan::compile(self.jobs, self.routes)?;
        let plan = plan.install_outputs(self.outputs)?;
        let plan = plan.install_inputs(self.inputs)?;
        Ok(plan.install_progress()?)
    }

    fn resolve_variadic_inputs(&mut self) -> Result<(), PlanError> {
        // This is a cold construction allocation. Fixed-arity plans skip it,
        // and invocation lane storage remains inline through eight positions.
        let mut arities = vec![0; self.jobs.len()];
        for (source, targets) in self.routes.iter().enumerate() {
            for route in targets {
                if let Some(arity) = arities.get_mut(route.node) {
                    let required = route.lane.checked_add(1).ok_or(
                        super::RouteError::Lane {
                            from: source,
                            target: route.node,
                            lane: route.lane,
                        },
                    )?;
                    *arity = (*arity).max(required);
                }
            }
        }
        for input in &self.inputs {
            if let Some(arity) = arities.get_mut(input.route.node) {
                let required = input
                    .route
                    .lane
                    .checked_add(1)
                    .ok_or(super::InputError::Invalid(input.id))?;
                *arity = (*arity).max(required);
            }
        }
        for (job, arity) in self.jobs.iter_mut().zip(arities) {
            job.resolve_inputs(arity);
        }
        Ok(())
    }
}
