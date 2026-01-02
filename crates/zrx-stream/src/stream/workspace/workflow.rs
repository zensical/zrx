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

//! Workflow.

use std::any::Any;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

use zrx_scheduler::{Id, Value};

use crate::stream::operator::Operator;
use crate::stream::Stream;

use super::traits::With;
use super::WorkspaceRef;

mod schedulable;

pub use schedulable::Schedulable;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Workflow.
#[derive(Debug)]
pub struct Workflow<I> {
    /// Shared inner state.
    inner: Rc<RefCell<WorkflowInner<I>>>,
}

/// Workflow inner state.
#[derive(Debug)]
pub struct WorkflowInner<I> {
    /// Identifier.
    id: usize,
    /// Associated workspace.
    workspace: WorkspaceRef<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Workflow<I>
where
    I: std::fmt::Debug,
{
    /// Creates a new workflow.
    #[must_use]
    pub fn new(id: usize, workspace: WorkspaceRef<I>) -> Self {
        Self {
            inner: Rc::new(RefCell::new(WorkflowInner { id, workspace })),
        }
    }

    /// Adds a source stream.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn add_source<T>(&self) -> Stream<I, T>
    where
        T: Any,
    {
        let id = self.with(|workflow| {
            let workspace = workflow.workspace.upgrade().expect("invariant");
            workspace.add_source::<T>()
        });
        Stream {
            id,
            workflow: self.clone(),
            marker: PhantomData,
        }
    }

    /// Adds an operator to the workflow.
    #[allow(clippy::missing_panics_doc)]
    pub fn add_operator<S, O, T, U>(&self, from: S, operator: O) -> Stream<I, U>
    where
        I: Id,
        S: IntoIterator<Item = usize>,
        O: Operator<I, T> + 'static,
        T: Value,
        U: Value,
    {
        let id = self.with(|workflow| {
            let workspace = workflow.workspace.upgrade().expect("invariant");
            workspace.add_action::<U, _, _>(from, Schedulable::new(operator))
        });
        Stream {
            id,
            workflow: self.clone(),
            marker: PhantomData,
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Workflow<I> {
    /// Returns the identifier of the workflow.
    pub fn id(&self) -> usize {
        self.with(|workflow| workflow.id)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> With for Workflow<I> {
    type Item = WorkflowInner<I>;

    /// Returns a reference to the inner state.
    #[inline]
    fn inner(&self) -> &RefCell<Self::Item> {
        &self.inner
    }
}

// ----------------------------------------------------------------------------

impl<I> Clone for Workflow<I> {
    /// Returns a copy of the workflow.
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}
