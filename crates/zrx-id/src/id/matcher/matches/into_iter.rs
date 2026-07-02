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

//! Consuming iterator implementation for [`Matches`].

use super::Matches;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Consuming iterator for [`Matches`].
#[derive(Debug)]
pub struct IntoIter {
    /// Blocks of bits.
    data: Vec<u64>,
    /// Current block index.
    index: usize,
    /// Current block.
    block: u64,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl IntoIterator for Matches {
    type Item = usize;
    type IntoIter = IntoIter;

    /// Creates a consuming iterator over the match set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::matcher::Matches;
    ///
    /// // Create match set from iterator
    /// let mut matches = Matches::from_iter([0, 1]);
    ///
    /// // Create iterator over match set
    /// for index in matches {
    ///     println!("{index:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        let block = self.data[0];
        IntoIter {
            data: self.data,
            index: 0,
            block,
        }
    }
}

// ----------------------------------------------------------------------------

impl Iterator for IntoIter {
    type Item = usize;

    /// Returns the next match.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.block != 0 {
                let num = self.block.trailing_zeros() as usize;

                // Clear the lowest bit and return it
                self.block &= self.block - 1;
                return Some(self.index << 6 | num);
            }

            // Move to the next block
            self.index += 1;

            // If all blocks are exhausted, we're done
            if self.index >= self.data.len() {
                return None;
            }

            // Update the current block to the next block
            self.block = self.data[self.index];
        }
    }
}
