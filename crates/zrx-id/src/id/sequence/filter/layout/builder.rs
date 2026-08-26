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

//! Layout builder.

use super::Layout;
use super::item::Item;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Layout builder.
#[derive(Debug)]
pub struct Builder {
    /// Layout items.
    items: Vec<Item>,
    /// Layout slots.
    slots: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Layout {
    /// Creates a layout builder.
    #[inline]
    #[must_use]
    pub fn builder(capacity: usize) -> Builder {
        Builder {
            items: Vec::with_capacity(capacity),
            slots: 0,
        }
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Adds a condition and its expression slots.
    #[inline]
    pub fn add(&mut self, index: usize, slots: usize) {
        let start = self.slots;

        // Add the item and update the total number of slots
        self.slots += slots;
        self.items.push(Item::new(index, start..self.slots));
    }

    /// Builds the layout.
    #[inline]
    #[must_use]
    pub fn build(self) -> Layout {
        Layout {
            items: self.items.into_boxed_slice(),
            slots: self.slots,
        }
    }
}
