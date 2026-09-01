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

//! Typed external bindings and internal destinations.

use thiserror::Error;

use crate::scheduler::action::Port;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid external input binding.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum InputError {
    /// Two external bindings name the same input.
    #[error("duplicate external input {0:?}")]
    Duplicate(InputId),
    /// An external input names a missing node, lane, or mismatched port.
    #[error("invalid external input {0:?}")]
    Invalid(InputId),
}

// ----------------------------------------------------------------------------

/// Invalid external output binding.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum OutputError {
    /// Two external bindings name the same output.
    #[error("duplicate external output {0:?}")]
    Duplicate(OutputId),
    /// An external output names a missing node or mismatched port.
    #[error("invalid external output {0:?}")]
    Invalid(OutputId),
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub(in crate::scheduler) enum Destination {
    Route(Route),
    Output(usize),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stable identity of one installed external input.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct InputId(u64);

// ----------------------------------------------------------------------------

/// Dense plan-local position of one installed external input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::scheduler) struct InputIndex(usize);

// ----------------------------------------------------------------------------

/// Stable identity of one installed external output.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutputId(u64);

// ----------------------------------------------------------------------------

/// One external output bound to a node's typed output.
pub struct OutputBinding {
    pub(in crate::scheduler) id: OutputId,
    pub(in crate::scheduler) source: usize,
    pub(in crate::scheduler) port: Port,
}

// ----------------------------------------------------------------------------

/// One statically validated output route.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Route {
    pub(in crate::scheduler) node: usize,
    pub(in crate::scheduler) lane: usize,
}

// ----------------------------------------------------------------------------

/// One external input normalized to an ordinary graph position.
pub struct InputBinding {
    pub(in crate::scheduler) id: InputId,
    pub(in crate::scheduler) route: Route,
    pub(in crate::scheduler) port: Port,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl InputId {
    /// Creates an input identity.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

// ----------------------------------------------------------------------------

impl InputIndex {
    pub(in crate::scheduler) const fn new(index: usize) -> Self {
        Self(index)
    }

    pub(in crate::scheduler) const fn get(self) -> usize {
        self.0
    }
}

// ----------------------------------------------------------------------------

impl OutputId {
    /// Creates an output identity.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

// ----------------------------------------------------------------------------

impl OutputBinding {
    /// Defines a typed external output.
    #[must_use]
    pub fn new<I, V>(id: OutputId, source: usize) -> Self
    where
        I: 'static,
        V: 'static,
    {
        Self {
            id,
            source,
            port: Port::of::<I, V>(),
        }
    }
}

// ----------------------------------------------------------------------------

impl Route {
    /// Routes output to one input lane.
    #[must_use]
    pub const fn new(node: usize, lane: usize) -> Self {
        Self { node, lane }
    }

    /// Returns the target node.
    #[must_use]
    pub const fn node(self) -> usize {
        self.node
    }

    /// Returns the target input lane.
    #[must_use]
    pub const fn lane(self) -> usize {
        self.lane
    }
}

// ----------------------------------------------------------------------------

impl Destination {
    pub(in crate::scheduler) const fn route(self) -> Option<Route> {
        match self {
            Self::Route(route) => Some(route),
            Self::Output(_) => None,
        }
    }
}

// ----------------------------------------------------------------------------

impl InputBinding {
    /// Defines a typed external input.
    #[must_use]
    pub fn new<I, V>(id: InputId, route: Route) -> Self
    where
        I: 'static,
        V: 'static,
    {
        Self {
            id,
            route,
            port: Port::of::<I, V>(),
        }
    }
}
