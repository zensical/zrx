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

//! Barrier lifecycle.

use zrx_store::stash::Items;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Barrier lifecycle
///
/// Tracks which scopes are currently submitted and which have completed,
/// and encapsulates all lifecycle transitions and their idempotency guards.
#[derive(Clone, Debug, Default)]
pub struct Lifecycle {
    /// Submitted scopes.
    submitted: Items,
    /// Completed scopes.
    completed: Items,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Lifecycle {
    /// Submits a scope index.
    ///
    /// Returns `false` if the scope was already submitted (idempotency guard).
    pub fn submit(&mut self, index: usize) -> bool {
        self.submitted.insert(index)
    }

    /// Withdraws a submitted scope index (e.g. cancelled or removed).
    ///
    /// Returns `false` if the scope was not submitted.
    pub fn withdraw(&mut self, index: usize) -> bool {
        if !self.submitted.remove(index) {
            return false;
        }
        self.completed.remove(index);
        true
    }

    /// Completes a scope index, moving it from submitted to completed.
    ///
    /// Returns `false` if the scope was already completed (idempotency guard).
    pub fn complete(&mut self, index: usize) -> bool {
        self.submitted.remove(index);
        self.completed.insert(index)
    }

    /// Returns `true` if a barrier with the given item set is fulfilled.
    ///
    /// Fulfilled means no matching scopes are still submitted, and at least
    /// one is completed.
    pub fn is_complete(&self, items: &Items) -> bool {
        !items.has_any(&self.submitted) && items.has_any(&self.completed)
    }
}
