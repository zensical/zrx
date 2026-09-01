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

//! Stream.

use std::marker::PhantomData;

pub use zrx_scheduler::Value;
use zrx_scheduler::action::Job;

mod combinator;
mod execution;
pub mod function;
pub mod key;
pub mod operator;
mod signal;
pub mod workflow;

pub use combinator::{StreamSetExt, StreamTupleExt};
pub use execution::{
    Advance, Error, Execution, Input, Output, Run, Runner, Scope, run,
};
pub use key::{Id, Key};
pub use operator::{Membership, Replication, concurrent, sequential};
pub use signal::Signal;
use workflow::Handle;
pub use workflow::{Direction, LookupError, Workflow};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Typed output of one workflow-local source or action.
pub struct Stream<I, T>
where
    I: Id,
{
    node: usize,
    workflow: Handle<I>,
    marker: PhantomData<fn() -> T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
{
    pub(crate) const fn new(node: usize, workflow: Handle<I>) -> Self {
        Self {
            node,
            workflow,
            marker: PhantomData,
        }
    }

    /// Returns the workflow-local node identifier.
    ///
    /// Higher-level composition layers can use this identity to associate
    /// sidecar metadata without making the stream layer own that metadata.
    #[inline]
    #[must_use]
    pub const fn node(&self) -> usize {
        self.node
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Clone for Stream<I, T>
where
    I: Id,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.node, self.workflow.clone())
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// One keyed stream mutation.
pub type Change<I, T> = zrx_scheduler::Change<Key<I>, T>;

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

pub(crate) fn source_job<I, T>() -> Job<Key<I>>
where
    I: Id,
    T: Value,
{
    Job::forward::<T>()
}
