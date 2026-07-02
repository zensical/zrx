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

//! Layout position set.

use std::ops::Range;

use super::{Condition, Layout};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Layout position set.
///
/// This scratch structure records, for each constrained slot across all dense
/// conditions, which input identifier positions matched that slot. Each slot
/// is represented as a [`u64`] bitset, where bit `n` means that the `n`th
/// input identifier matched the slot.
#[derive(Debug)]
pub struct Positions {
    /// Candidate match positions per constrained slot.
    pub slots: Vec<u64>,
    /// Number of input identifiers that were scanned.
    pub len: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Layout {
    /// Creates a fresh position set for candidate assembly.
    #[inline]
    #[must_use]
    pub fn positions(&self) -> Positions {
        Positions {
            slots: vec![0; self.slots],
            len: 0,
        }
    }
}

// ----------------------------------------------------------------------------

impl Positions {
    /// Marks the given position in the position set.
    #[inline]
    pub fn mark(&mut self, index: usize) {
        self.slots[index] |= 1 << (self.len - 1);
    }

    /// Advances the input sequence by one identifier.
    #[inline]
    pub fn advance(&mut self) {
        self.len += 1;
    }

    /// Returns whether the given positions satisfy the condition.
    ///
    /// This method first checks that every constrained slot of that condition
    /// matched at least once at some input position. If any slot stayed empty,
    /// the condition cannot possibly be satisfied and we can reject it without
    /// running the condition state machine.
    #[must_use]
    pub fn satisfies(
        &self, range: &Range<usize>, condition: &Condition,
    ) -> bool {
        let mut iter = self.slots[range.clone()].iter();
        if iter.any(|positions| *positions == 0) {
            return false;
        }

        // Check whether the positions satisfy the condition
        let positions = &self.slots[range.start..range.end];
        condition.satisfies(self.len, positions)
    }
}
