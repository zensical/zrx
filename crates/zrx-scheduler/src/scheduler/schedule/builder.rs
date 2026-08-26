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

//! Schedule builder.

use std::collections::VecDeque;
use std::fmt::Debug;

use zrx_storage::Storages;

use super::Schedule;
use super::frontier::Frontiers;
use super::graph;

mod error;
mod subscriber;

pub use error::{Error, Result};
pub use subscriber::Subscriber;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Schedule builder.
#[derive(Debug)]
pub struct Builder<I> {
    /// Action graph builder.
    graph: graph::Builder<I>,
    /// Action storage set.
    storages: Storages,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Schedule<I> {
    /// Creates a schedule builder.
    ///
    /// Schedules are implemented as graphs, with each node being created from
    /// an [`Action`][], and edges representing the flow of data between nodes.
    /// Actions can be thought of as functions, where the inputs correspond to
    /// the arguments of a function, and the output to its return value.
    ///
    /// Naturally, each [`Schedule`] must have one or more [`Source`][] nodes,
    /// defining the entrypoints of the schedule. They can be linked to other
    /// schedules by connecting the outputs of their nodes to the inputs of
    /// other nodes, as well as to [`Session`][] handles.
    ///
    /// [`Action`]: crate::scheduler::action::Action
    /// [`Session`]: crate::scheduler::session::Session
    /// [`Source`]: crate::scheduler::action::graph::Source
    #[inline]
    #[must_use]
    pub fn builder() -> Builder<I> {
        Builder::default()
    }
}

// ----------------------------------------------------------------------------

impl<I> Builder<I> {
    /// Builds the schedule.
    #[inline]
    #[must_use]
    pub fn build(self) -> Schedule<I> {
        let len = self.graph.len();
        Schedule {
            graph: self.graph.build().into_transitive(),
            storages: self.storages,
            frontiers: Frontiers::default(),
            queues: vec![VecDeque::new(); len],
            events: (0..len).map(|_| VecDeque::new()).collect(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Default for Builder<I> {
    /// Creates a schedule builder.
    #[inline]
    fn default() -> Self {
        Self {
            graph: graph::Builder::default(),
            storages: Storages::default(),
        }
    }
}
