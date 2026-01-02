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

//! Workspace.

use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

use zrx_scheduler::graph::Builder;
use zrx_scheduler::Action;

mod traits;
mod workflow;

use traits::With;
pub use workflow::Workflow;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Workspace.
#[derive(Debug, Default)]
pub struct Workspace<I> {
    /// Shared inner state.
    inner: Rc<RefCell<WorkspaceInner<I>>>,
}

/// Workspace.
#[derive(Debug)]
pub struct WorkspaceRef<I> {
    /// Shared inner state.
    inner: Weak<RefCell<WorkspaceInner<I>>>,
}

/// Workspace inner state.
#[derive(Debug, Default)]
pub struct WorkspaceInner<I> {
    /// Workflows.
    workflows: Vec<Workflow<I>>,
    /// Workspace builder.
    builder: Builder<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Workspace<I>
where
    I: std::fmt::Debug,
{
    /// Creates a workspace.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(WorkspaceInner {
                workflows: Vec::new(),
                builder: Builder::new(),
            })),
        }
    }

    /// Adds a workflow to the workspace.
    #[must_use]
    pub fn add_workflow(&self) -> Workflow<I> {
        self.with_mut(|workspace| {
            let id = workspace.workflows.len();
            let workflow = Workflow::new(
                id,
                WorkspaceRef {
                    inner: Rc::downgrade(&self.inner),
                },
            );
            workspace.workflows.push(workflow.clone());
            workflow
        })
    }

    /// Adds a source to the workspace.
    #[must_use]
    pub fn add_source<T>(&self) -> usize
    where
        T: Any,
    {
        self.with_mut(|workspace| workspace.builder.add_source::<T>())
    }

    /// Adds an action to the workspace.
    pub fn add_action<T, S, A>(&self, from: S, action: A) -> usize
    where
        T: 'static,
        S: IntoIterator<Item = usize>,
        A: Action<I> + 'static,
    {
        self.with_mut(|workspace| {
            workspace.builder.add_action::<T>().with(from, action)
        })
    }

    /// Consumes the workspace and returns the builder.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn into_builder(self) -> Builder<I> {
        let inner = Rc::try_unwrap(self.inner).expect("couldnt unwrap");
        let workspace = inner.into_inner();
        workspace.builder
    }
}

impl<I> WorkspaceRef<I> {
    /// Upgrades the weak reference to a strong reference.
    #[must_use]
    pub fn upgrade(&self) -> Option<Workspace<I>> {
        self.inner.upgrade().map(|inner| Workspace { inner })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> With for Workspace<I> {
    type Item = WorkspaceInner<I>;

    /// Returns a reference to the inner state.
    #[inline]
    fn inner(&self) -> &RefCell<Self::Item> {
        &self.inner
    }
}

impl<I> Clone for Workspace<I> {
    // relax trait bounds
    fn clone(&self) -> Self {
        let inner = Rc::clone(&self.inner);
        Self { inner }
    }
}

impl<I> Clone for WorkspaceRef<I> {
    // relax trait bounds
    fn clone(&self) -> Self {
        let inner = Weak::clone(&self.inner);
        Self { inner }
    }
}
