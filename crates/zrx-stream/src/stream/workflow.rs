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

//! Scoped construction of one typed, mergeable stream workflow.

use zrx_scheduler::Value;
use zrx_scheduler::action::Job;
use zrx_scheduler::plan::{
    InputBinding, OutputBinding, Plan, PlanError, Route,
};

use crate::stream::Id;
use crate::stream::Key;

mod builder;
mod endpoint;

pub use builder::Builder;
pub(in crate::stream) use builder::Handle;
use endpoint::unique;
pub use endpoint::{Direction, Input, Inputs, LookupError, Output, Outputs};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Closed, mergeable typed-action workflow with erased external ports.
#[must_use]
pub struct Workflow<I>
where
    I: Id,
{
    jobs: Vec<Job<Key<I>>>,
    routes: Vec<Vec<Route>>,
    input_bindings: Vec<InputBinding>,
    output_bindings: Vec<OutputBinding>,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
}

// ----------------------------------------------------------------------------

pub(super) struct Definition<I>
where
    I: Id,
{
    pub jobs: Vec<Job<Key<I>>>,
    pub routes: Vec<Vec<Route>>,
    pub input_bindings: Vec<InputBinding>,
    pub output_bindings: Vec<OutputBinding>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Workflow<I>
where
    I: Id,
{
    fn new(
        definition: Definition<I>, inputs: Vec<Input>, outputs: Vec<Output>,
    ) -> Self {
        let Definition {
            jobs,
            routes,
            input_bindings,
            output_bindings,
        } = definition;
        Self {
            jobs,
            routes,
            input_bindings,
            output_bindings,
            inputs,
            outputs,
        }
    }

    /// Constructs and closes one isolated stream workflow.
    pub fn build(build: impl FnOnce(&mut Builder<I>)) -> Self {
        let mut builder = Builder::new();
        build(&mut builder);
        builder.finish()
    }

    /// Returns the erased input catalogue in construction order.
    #[must_use]
    pub fn inputs(&self) -> Inputs<'_, I> {
        Inputs::new(&self.inputs)
    }

    /// Returns the erased output catalogue in construction order.
    #[must_use]
    pub fn outputs(&self) -> Outputs<'_, I> {
        Outputs::new(&self.outputs)
    }

    /// Finds the sole input carrying `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if no input or multiple inputs carry `T`.
    pub fn input<T>(&self) -> Result<Input, LookupError>
    where
        T: Value,
    {
        unique(
            self.inputs().of_type::<T>().copied(),
            Direction::Input,
            std::any::type_name::<T>(),
        )
    }

    /// Finds the sole output carrying `T`.
    ///
    /// # Errors
    ///
    /// Returns an error if no output or multiple outputs carry `T`.
    pub fn output<T>(&self) -> Result<Output, LookupError>
    where
        T: Value,
    {
        unique(
            self.outputs().of_type::<T>().copied(),
            Direction::Output,
            std::any::type_name::<T>(),
        )
    }

    pub(in crate::stream) fn lower(self) -> Result<Lowered<I>, PlanError> {
        let plan = Plan::builder(self.jobs, self.routes)
            .inputs(self.input_bindings)
            .outputs(self.output_bindings)
            .build()?;
        Ok((plan, self.inputs, self.outputs))
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

type Lowered<I> = (Plan<Key<I>>, Vec<Input>, Vec<Output>);

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use zrx_scheduler::Value;

    use crate::stream::Key;
    use crate::stream::function::Scope;
    use crate::stream::{StreamTupleExt, concurrent, sequential};

    use super::{Direction, LookupError, Workflow};

    #[derive(Clone)]
    struct OpaqueValue;

    impl Value for OpaqueValue {}

    #[test]
    fn maps_values_without_debug_or_equality() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let source = workflow.input::<OpaqueValue>();
            let mapped = source.clone().map(|_: &OpaqueValue| 1_u64);
            let filtered = source.filter_map(|_: &OpaqueValue| Some(2_u64));
            workflow.output(&mapped);
            workflow.output(&filtered);
        });

        workflow.lower().unwrap();
    }

    #[test]
    fn constructs_raw_sequential_and_bounded_maps() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let source = workflow.input::<u32>();
            let state = Mutex::new(1_u32);
            let sequential = source.map(sequential(move |value: &u32| {
                *value + *state.lock().unwrap()
            }));
            let adaptive = sequential.map(|value: &u32| *value * 2);
            let bounded = adaptive.map(concurrent(2, |value: &u32| *value - 2));
            workflow.output(&bounded);
        });
        workflow.lower().unwrap();
    }

    #[test]
    fn clones_stream_handles_for_fan_out_without_cloning_values() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let source = workflow.input::<u32>();
            let left = source.clone().map(|value: &u32| *value + 1);
            let right = source.map(|value: &u32| *value * 2);
            workflow.output(&left);
            workflow.output(&right);
        });
        workflow.lower().unwrap();
    }

    #[test]
    fn infers_the_complete_map_callback_vocabulary() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let scalar = workflow.input::<u32>();
            let tuple = workflow.input::<(u32, u32)>();

            let _ = scalar.map(|scope: &mut Scope<'_, u64>| {
                *scope.key().try_as_id().unwrap()
            });
            let _ = scalar.map(|scope: &mut Scope<'_, u64>, value: &u32| {
                u32::try_from(*scope.key().try_as_id().unwrap()).unwrap()
                    + *value
            });
            let _ = scalar.map(|key: &Key<u64>| *key.try_as_id().unwrap());
            let _ = scalar.map(|key: &Key<u64>, value: &u32| {
                u32::try_from(*key.try_as_id().unwrap()).unwrap() + *value
            });
            let _ = scalar.map(|id: &u64, value: &u32| {
                u32::try_from(*id).unwrap() + *value
            });
            let _ = scalar.map(|value: &u32| *value * 2);

            let _ = tuple.map(
                |scope: &mut Scope<'_, u64>, left: &u32, right: &u32| {
                    u32::try_from(*scope.key().try_as_id().unwrap()).unwrap()
                        + *left
                        + *right
                },
            );
            let _ = tuple.map(|key: &Key<u64>, left: &u32, right: &u32| {
                u32::try_from(*key.try_as_id().unwrap()).unwrap()
                    + *left
                    + *right
            });
            let _ = tuple.map(|id: &u64, left: &u32, right: &u32| {
                u32::try_from(*id).unwrap() + *left + *right
            });
            let output = tuple.map(|left: &u32, right: &u32| *left + *right);
            workflow.output(&output);
        });
        workflow.lower().unwrap();
    }

    #[test]
    fn constructs_joins_through_arity_eight() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let streams = (
                workflow.input::<u8>(),
                workflow.input::<u16>(),
                workflow.input::<u32>(),
                workflow.input::<u64>(),
                workflow.input::<i8>(),
                workflow.input::<i16>(),
                workflow.input::<i32>(),
                workflow.input::<i64>(),
            );
            let joined = streams.join();
            workflow.output(&joined);
        });
        workflow.lower().unwrap();
    }

    #[test]
    fn exposes_erased_ports_and_unique_typed_lookup() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let first = workflow.input::<u64>();
            let second = workflow.input::<String>();
            workflow.output(&first);
            workflow.output(&second);
        });

        assert_eq!(workflow.inputs().len(), 2);
        assert_eq!(workflow.outputs().len(), 2);
        assert_eq!(workflow.input::<u64>().unwrap(), workflow.inputs[0]);
        assert_eq!(workflow.output::<String>().unwrap(), workflow.outputs[1]);
        assert_eq!(
            workflow.input::<u8>(),
            Err(LookupError::Missing {
                direction: Direction::Input,
                value: "u8",
            })
        );
    }

    #[test]
    fn iterates_ambiguous_same_type_ports() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let first = workflow.input::<u64>();
            let second = workflow.input::<u64>();
            workflow.output(&first);
            workflow.output(&second);
        });

        assert!(matches!(
            workflow.input::<u64>(),
            Err(LookupError::Ambiguous {
                direction: Direction::Input,
                ..
            })
        ));
        assert_eq!(workflow.inputs().of_type::<u64>().count(), 2);
        assert_eq!(workflow.outputs().of_type::<u64>().count(), 2);
    }

    #[test]
    #[should_panic(expected = "another workflow construction")]
    fn rejects_a_foreign_stream_output() {
        let mut escaped = None;
        let _first = Workflow::<u64>::build(|workflow| {
            escaped = Some(workflow.input::<u64>());
        });
        let _second = Workflow::<u64>::build(|workflow| {
            workflow.output(escaped.as_ref().unwrap());
        });
    }

    #[test]
    #[should_panic(expected = "stream construction has ended")]
    fn escaped_stream_becomes_stale_after_construction() {
        let mut escaped = None;
        let _workflow = Workflow::<u64>::build(|workflow| {
            escaped = Some(workflow.input::<u32>());
        });
        let _ = escaped.unwrap().map(|value: &u32| *value * 2);
    }
}
