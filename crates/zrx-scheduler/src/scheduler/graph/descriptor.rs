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

//! Descriptor.

use std::any::{type_name, Any, TypeId};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Descriptor.
///
/// This data type represents the type of a [`Node`][] in the [`Graph`][] that
/// is managed by the [`Scheduler`][], since it is generic and solely operates
/// on type-erased nodes. Descriptors allow to route an [`Item`][] passed in
/// a [`Session`][] to the corresponding set of [`Source`][] nodes.
///
/// We currently only support Rust data types, so this will probably need to be
/// converted into an enum once we start working on Python support.
///
/// [`Graph`]: crate::scheduler::graph::Graph
/// [`Item`]: crate::scheduler::effect::Item
/// [`Node`]: crate::scheduler::graph::Node
/// [`Scheduler`]: crate::scheduler::Scheduler
/// [`Session`]: crate::scheduler::session::Session
/// [`Source`]: crate::scheduler::graph::Source
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Descriptor {
    /// Type identifier.
    type_id: TypeId,
    /// Type name.
    name: String,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Descriptor {
    /// Creates a descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::graph::Descriptor;
    ///
    /// // Create descriptor
    /// let descriptor = Descriptor::new::<i32>();
    /// ```
    #[must_use]
    pub fn new<T>() -> Self
    where
        T: Any,
    {
        let name = type_name::<T>();
        Descriptor {
            type_id: TypeId::of::<T>(),
            name: name.into(),
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl Descriptor {
    /// Returns the type identifier.
    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    /// Returns the type name.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }
}
