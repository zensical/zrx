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

//! Runnable-node membership and fair invocation selection.

use std::collections::VecDeque;

// Initial fairness quantum before measurement-based scheduling can tune work
// per invocation. This is an execution bootstrap, not a semantic batch size.
const BOOTSTRAP_SLICE_ITEMS: usize = 1_024;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum InvocationClass {
    Event,
    Data,
    Wake,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Node {
    next_class: InvocationClass,
    next_lane: usize,
    queued: bool,
}

// ----------------------------------------------------------------------------

pub struct Scheduling {
    nodes: Vec<Node>,
    ready: VecDeque<usize>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl InvocationClass {
    const fn next(self) -> Self {
        match self {
            Self::Event => Self::Data,
            Self::Data => Self::Wake,
            Self::Wake => Self::Event,
        }
    }
}

// ----------------------------------------------------------------------------

impl Scheduling {
    pub fn new(nodes: usize) -> Self {
        Self {
            nodes: (0..nodes)
                .map(|_| Node {
                    next_class: InvocationClass::Event,
                    next_lane: 0,
                    queued: false,
                })
                .collect(),
            ready: VecDeque::new(),
        }
    }

    pub fn slice(items: usize, parallelism: usize) -> usize {
        assert!(items != 0, "empty segments are not schedulable");
        assert!(parallelism != 0, "node parallelism must be non-zero");
        items.div_ceil(parallelism).clamp(1, BOOTSTRAP_SLICE_ITEMS)
    }

    pub fn enqueue(&mut self, node: usize) {
        if !self.nodes[node].queued {
            self.nodes[node].queued = true;
            self.ready.push_back(node);
        }
    }

    pub fn pop(&mut self) -> Option<usize> {
        let node = self.ready.pop_front()?;
        self.nodes[node].queued = false;
        Some(node)
    }

    pub fn classes(&self, node: usize) -> [InvocationClass; 3] {
        let first = self.nodes[node].next_class;
        let second = first.next();
        [first, second, second.next()]
    }

    pub fn selected(&mut self, node: usize, class: InvocationClass) {
        self.nodes[node].next_class = class.next();
    }

    pub fn next_lane(&self, node: usize) -> usize {
        self.nodes[node].next_lane
    }

    pub fn selected_lane(
        &mut self, node: usize, lane: usize, lane_count: usize,
    ) {
        self.nodes[node].next_lane = (lane + 1) % lane_count;
    }
}

#[cfg(test)]
mod tests {
    use super::{BOOTSTRAP_SLICE_ITEMS, Scheduling};

    #[test]
    fn synchronous_execution_keeps_the_bootstrap_quantum() {
        assert_eq!(Scheduling::slice(1, 1), 1);
        assert_eq!(Scheduling::slice(10_000, 1), BOOTSTRAP_SLICE_ITEMS);
    }

    #[test]
    fn parallel_execution_derives_a_bounded_initial_slice() {
        assert_eq!(Scheduling::slice(1, 4), 1);
        assert_eq!(Scheduling::slice(100, 4), 25);
        assert_eq!(Scheduling::slice(10_000, 4), BOOTSTRAP_SLICE_ITEMS);
    }
}
