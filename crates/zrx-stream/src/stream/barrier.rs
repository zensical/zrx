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

//! Barrier.

use std::fmt::{self, Debug};
use std::sync::Arc;

use zrx_scheduler::{Id, Key, Value};
use zrx_store::stash::Items;

pub mod advance;
mod lifecycle;
pub mod set;

pub use advance::Advance;
use lifecycle::Lifecycle;
pub use set::Barriers;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Barrier function.
trait BarrierFn<I>: Send + Sync {
    /// Returns whether the barrier contains the given scope.
    fn contains(&self, scope: &Key<I>) -> bool;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Barrier.
#[derive(Clone)]
pub struct Barrier<I> {
    /// Barrier function.
    function: Arc<dyn BarrierFn<I>>,
    /// Contained scopes.
    items: Items,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Barrier<I>
where
    I: Id,
{
    /// Creates a barrier with the given function.
    #[must_use]
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&Key<I>) -> bool + Send + Sync + 'static,
    {
        Self {
            function: Arc::new(f),
            items: Items::new(),
        }
    }

    /// Returns whether the barrier contains the given scope.
    #[inline]
    #[must_use]
    pub fn contains(&self, scope: &Key<I>) -> bool {
        self.function.contains(scope)
    }

    /// Inserts the given index into the barrier.
    #[inline]
    fn insert(&mut self, index: usize) -> bool {
        self.items.insert(index)
    }

    /// Removes the given index from the barrier.
    #[inline]
    fn remove(&mut self, index: usize) -> bool {
        self.items.remove(index)
    }

    /// Returns `true` if the barrier is fulfilled.
    #[inline]
    fn is_complete(&self, lifecycle: &Lifecycle) -> bool {
        lifecycle.is_complete(&self.items)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Barrier<I> {
    /// Returns the items of the barrier.
    #[inline]
    pub fn items(&self) -> &Items {
        &self.items
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Value for Barrier<I> where I: Id {}

// ----------------------------------------------------------------------------

impl<I> PartialEq for Barrier<I> {
    /// Compares two barriers for equality.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.function, &other.function)
    }
}

impl<I> Eq for Barrier<I> {}

// ----------------------------------------------------------------------------

impl<I> Debug for Barrier<I> {
    /// Formats the barrier for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let function = "Box<dyn BarrierFn>";
        f.debug_struct("Barrier")
            .field("function", &function)
            .field("items", &self.items)
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I> BarrierFn<I> for F
where
    F: Fn(&Key<I>) -> bool + Send + Sync,
{
    #[inline]
    fn contains(&self, scope: &Key<I>) -> bool {
        self(scope)
    }
}
