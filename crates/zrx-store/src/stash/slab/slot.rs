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
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Generational slot.

use std::fmt::{self, Display, Write};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Generational slot.
///
/// Slots combine indices with the generation assigned when their values were
/// inserted. Removing a value invalidates its slot, even if the underlying
/// slab index is later reused. Slots are local to the [`Slab`][] that created
/// them – a slot from another slab must not be treated as a unique identity.
///
/// [`Slab`]: crate::stash::Slab
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slot {
    /// Slab index.
    index: usize,
    /// Slab generation.
    generation: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Slot {
    /// Reconstructs a generational slot from its parts.
    ///
    /// This method is named `from_parts` rather than `new` because it does
    /// not allocate a slot or make it valid. It only reconstructs a candidate
    /// identity; access succeeds when both parts identify a current value in
    /// the receiving slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slot;
    ///
    /// // Reconstruct slot from parts
    /// let slot = Slot::from_parts(0, 1);
    /// ```
    #[must_use]
    pub fn from_parts(index: usize, generation: u64) -> Self {
        Self { index, generation }
    }
}

#[allow(clippy::must_use_candidate)]
impl Slot {
    /// Returns the slab index.
    #[inline]
    pub fn index(self) -> usize {
        self.index
    }

    /// Returns the slab generation.
    #[inline]
    pub fn generation(self) -> u64 {
        self.generation
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl AsRef<Slot> for Slot {
    /// Returns the generational slot as is.
    #[inline]
    fn as_ref(&self) -> &Slot {
        self
    }
}

// ----------------------------------------------------------------------------

impl Display for Slot {
    /// Formats the generational slot for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.index, f)?;
        f.write_char('@')?;
        Display::fmt(&self.generation, f)
    }
}
