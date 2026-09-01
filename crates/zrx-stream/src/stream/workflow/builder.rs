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

//! Mutable construction of one isolated typed stream graph.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use zrx_scheduler::Value;
use zrx_scheduler::action::{Job, Port};
use zrx_scheduler::plan::{
    InputBinding, InputId, OutputBinding, OutputId, Route,
};

use crate::stream::Id;
use crate::stream::{Key, Stream};

use super::endpoint::{Input, Output};
use super::{Definition, Workflow};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct InputSpec {
    endpoint: Input,
    binding: InputBinding,
}

// ----------------------------------------------------------------------------

struct OutputSpec {
    endpoint: Output,
    binding: OutputBinding,
}

// ----------------------------------------------------------------------------

pub struct Construction<I>
where
    I: Id,
{
    jobs: Vec<Job<Key<I>>>,
    routes: Vec<Vec<Route>>,
    inputs: Vec<InputSpec>,
    outputs: Vec<OutputSpec>,
}

// ----------------------------------------------------------------------------

/// Scoped mutable owner of one open typed stream construction.
pub struct Builder<I>
where
    I: Id,
{
    inner: Rc<RefCell<Construction<I>>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Construction<I>
where
    I: Id,
{
    fn new() -> Self {
        Self {
            jobs: Vec::new(),
            routes: Vec::new(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub(in crate::stream) fn job(
        &mut self, inputs: impl IntoIterator<Item = usize>, job: Job<Key<I>>,
    ) -> usize {
        let node = self.jobs.len();
        for (lane, source) in inputs.into_iter().enumerate() {
            self.routes[source].push(Route::new(node, lane));
        }
        self.jobs.push(job);
        self.routes.push(Vec::new());
        node
    }

    fn finish(self) -> Workflow<I> {
        let inputs = self.inputs.iter().map(|spec| spec.endpoint).collect();
        let outputs = self.outputs.iter().map(|spec| spec.endpoint).collect();
        let input_bindings =
            self.inputs.into_iter().map(|spec| spec.binding).collect();
        let output_bindings =
            self.outputs.into_iter().map(|spec| spec.binding).collect();
        Workflow::new(
            Definition {
                jobs: self.jobs,
                routes: self.routes,
                input_bindings,
                output_bindings,
            },
            inputs,
            outputs,
        )
    }
}

// ----------------------------------------------------------------------------

impl<I> Builder<I>
where
    I: Id,
{
    pub(in crate::stream) fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Construction::new())),
        }
    }

    /// Adds one typed externally addressable input.
    pub fn input<T>(&mut self) -> Stream<I, T>
    where
        T: Value,
    {
        self.input_endpoint().1
    }

    pub(in crate::stream) fn input_endpoint<T>(
        &mut self,
    ) -> (Input, Stream<I, T>)
    where
        T: Value,
    {
        let mut inner = self.inner.borrow_mut();
        let node = inner.job([], crate::stream::source_job::<I, T>());
        let id = InputId::new(
            u64::try_from(inner.inputs.len()).expect("input count fits in u64"),
        );
        let endpoint = Input::new(id, Port::of::<Key<I>, T>());
        inner.inputs.push(InputSpec {
            endpoint,
            binding: InputBinding::new::<Key<I>, T>(id, Route::new(node, 0)),
        });
        (endpoint, Stream::new(node, Rc::downgrade(&self.inner)))
    }

    /// Registers one typed external output.
    ///
    /// # Panics
    ///
    /// Panics if `stream` belongs to another workflow construction.
    pub fn output<T>(&mut self, stream: &Stream<I, T>)
    where
        T: Value,
    {
        self.output_endpoint(stream);
    }

    pub(in crate::stream) fn output_endpoint<T>(
        &mut self, stream: &Stream<I, T>,
    ) -> Output
    where
        T: Value,
    {
        assert!(
            self.owns(stream),
            "stream belongs to another workflow construction"
        );
        let mut inner = self.inner.borrow_mut();
        let id = OutputId::new(
            u64::try_from(inner.outputs.len())
                .expect("output count fits in u64"),
        );
        let endpoint = Output::new(id, Port::of::<Key<I>, T>());
        inner.outputs.push(OutputSpec {
            endpoint,
            binding: OutputBinding::new::<Key<I>, T>(id, stream.node()),
        });
        endpoint
    }

    /// Returns whether this construction owns the stream.
    #[must_use]
    pub fn owns<T>(&self, stream: &Stream<I, T>) -> bool {
        stream.workflow.ptr_eq(&Rc::downgrade(&self.inner))
    }

    pub(in crate::stream) fn finish(self) -> Workflow<I> {
        let inner = Rc::try_unwrap(self.inner).unwrap_or_else(|_| {
            panic!("stream workflow builder must be the sole strong owner")
        });
        inner.into_inner().finish()
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

pub type Handle<I> = Weak<RefCell<Construction<I>>>;
