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

//! Action graph node.

use crate::scheduler::action::Context;
use crate::scheduler::signal::Id;

use super::descriptor::Descriptor;

mod handler;
mod kind;

pub use handler::{Handler, Iter};
pub use kind::{Kind, Source, Worker};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Action graph node.
#[derive(Debug)]
pub struct Node<I> {
    /// Descriptor.
    descriptor: Descriptor,
    /// Node kind.
    kind: Kind<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Node<I>
where
    I: Id,
{
    /// Creates an action graph node.
    #[must_use]
    pub fn new<K>(descriptor: Descriptor, kind: K) -> Self
    where
        K: Into<Kind<I>>,
    {
        Self { descriptor, kind: kind.into() }
    }

    /// Executes the node's handler.
    #[inline]
    pub fn execute(&mut self, ctx: Context<I>) -> Iter<'_, I> {
        match &mut self.kind {
            Kind::Source(source) => source.execute(ctx),
            Kind::Worker(worker) => worker.execute(ctx),
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Node<I> {
    /// Returns the descriptor.
    #[inline]
    pub fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    /// Returns the source if this is a source node.
    pub fn as_source_mut(&mut self) -> Option<&mut Source<I>> {
        if let Kind::Source(source) = &mut self.kind {
            Some(source)
        } else {
            None
        }
    }

    /// Returns the worker if this is a worker node.
    pub fn as_worker_mut(&mut self) -> Option<&mut Worker<I>> {
        if let Kind::Worker(worker) = &mut self.kind {
            Some(worker)
        } else {
            None
        }
    }
}
