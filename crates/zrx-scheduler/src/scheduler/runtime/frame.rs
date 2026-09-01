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

//! Ordered revision frames transported independently of action data.

use crate::scheduler::action::control::ProgressEvent;
use crate::scheduler::plan::ProgressIndex;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One key-free frame shared by all progress subscribers of an input.
#[derive(Clone)]
pub struct ProgressFrame {
    progress: ProgressIndex,
    sequence: u64,
    event: ProgressEvent,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl ProgressFrame {
    pub const fn new(
        progress: ProgressIndex, sequence: u64, event: ProgressEvent,
    ) -> Self {
        Self { progress, sequence, event }
    }

    pub const fn identity(&self) -> (ProgressIndex, u64) {
        (self.progress, self.sequence)
    }

    pub const fn event(&self) -> &ProgressEvent {
        &self.event
    }

    pub fn abort_end(&mut self) {
        if self.event.is_end() {
            self.event = ProgressEvent::Abort;
        }
    }

    pub fn merge(&mut self, other: Self) {
        assert_eq!(self.identity(), other.identity());
        match (&mut self.event, other.event) {
            (ProgressEvent::Begin, ProgressEvent::Begin)
            | (ProgressEvent::End | ProgressEvent::Abort, ProgressEvent::End)
            | (ProgressEvent::Abort, ProgressEvent::Abort) => {}
            (current @ ProgressEvent::End, ProgressEvent::Abort) => {
                *current = ProgressEvent::Abort;
            }
            _ => panic!("different progress events shared one identity"),
        }
    }

    pub const fn is_abort(&self) -> bool {
        self.event.is_abort()
    }

    pub const fn is_end(&self) -> bool {
        self.event.is_end()
    }
}
