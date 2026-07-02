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

//! Condition.

use crate::id::expression::Expression;

mod builder;
mod segment;

use segment::Segment;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Condition.
#[derive(Debug)]
pub struct Condition {
    /// Condition segments.
    segments: Box<[Segment]>,
    /// Extracted expressions.
    expressions: Box<[Expression]>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Condition {
    /// Returns whether the condition is satisfied by the given positions.
    ///
    /// This method evaluates the given condition as a compact position-state
    /// machine. The current state bitset tracks all candidate positions that
    /// are reachable after consuming the current sequence prefix, allowing
    /// expression and gap segments to be applied as fast bit operations.
    ///
    /// Note that this method assumes that there're never more than 64 candidate
    /// identifiers, as the current implementation uses a `u64` to track the
    /// reachable positions. This is a reasonable assumption, as the number of
    /// candidate identifiers is usually significantly smaller.
    ///
    /// # Examples
    ///
    /// Consider the sequence `[A, Gap, B]` against a candidate of length `3`,
    /// where `A` matched at position `0` and `B` matched at position `2`:
    ///
    /// 1. Start with `state = 0b0001`, meaning position `0` is reachable.
    /// 2. `A` keeps position `0` and shifts to `state = 0b0010`.
    /// 3. `Gap` expands that position to a suffix: `state = 0b1110`.
    /// 4. `B` keeps position `2` and shifts to `state = 0b1000`.
    /// 5. Since bit `3` is set, the full sequence matches the candidate.
    #[must_use]
    pub fn satisfies(&self, len: usize, positions: &[u64]) -> bool {
        let mut state = 1u64;

        // Sequences with at least one gap must still match all expressions,
        // so the candidate can never be shorter than the number of slots
        if self.segments.len() > self.expressions.len() {
            if len < self.expressions.len() {
                return false;
            }

        // Sequences without gaps consume exactly one candidate identifier per
        // constrained expression, so the candidate length must match exactly.
        } else if len != self.expressions.len() {
            return false;
        }

        // Evaluate the condition as a compact position-state machine, segments
        // either keep the reachable positions (for expressions) or expand them
        // to a suffix (for gaps). Reachable positions are tracked as a bitset,
        // where each bit corresponds to a candidate position.
        for segment in &self.segments {
            state = match *segment {
                Segment::Expression(slot) => (positions[slot] & state) << 1,
                Segment::Gap if state == 0 => continue,
                Segment::Gap => {
                    let start = state.trailing_zeros() as usize;
                    let lower = u64::MAX >> (63 - len);
                    lower & (u64::MAX << start)
                }
            }
        }

        // At the end, position `len` must be reachable, meaning the full
        // sequence consumed a valid path through the candidate identifiers
        state & (1 << len) != 0
    }
}

#[allow(clippy::must_use_candidate)]
impl Condition {
    /// Returns the extracted expressions.
    #[inline]
    pub fn expressions(&self) -> &[Expression] {
        &self.expressions
    }
}
