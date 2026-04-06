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

use std::any::{Any, TypeId};

mod symbol;

use symbol::Symbol;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Descriptor.
///
/// This data type represents the descriptor of a [`Node`][] in the [`Graph`][],
/// which holds the type identifier and [`Symbol`] of the node, both of which
/// are used for node identification and matching during graph construction,
/// so that multiple action graphs can be stitched together dynamically.
///
/// [`Graph`]: crate::scheduler::action::graph::Graph
/// [`Node`]: crate::scheduler::action::graph::Node
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Descriptor {
    /// Type identifier.
    id: TypeId,
    /// Symbol.
    symbol: Symbol,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Descriptor {
    /// Creates a descriptor.
    #[inline]
    #[must_use]
    pub fn of<T>() -> Self
    where
        T: Any,
    {
        Descriptor {
            id: TypeId::of::<T>(),
            symbol: Symbol::of::<T>(),
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl Descriptor {
    /// Returns the type identifier.
    #[inline]
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// Returns the symbol.
    #[inline]
    pub fn symbol(&self) -> &Symbol {
        &self.symbol
    }
}
