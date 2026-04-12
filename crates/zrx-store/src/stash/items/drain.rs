// ----------------------------------------------------------------------------

//! Drain iterator implementation for [`Items`].

use super::Items;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Drain iterator for [`Items`].
#[derive(Debug)]
pub struct Drain<'a> {
    /// Blocks of bits.
    data: &'a mut Vec<u64>,
    /// Current block index.
    index: usize,
    /// Current block.
    block: u64,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Items {
    /// Creates a drain iterator over the item set.
    ///
    /// Removes all items from the set as they are yielded. If the iterator
    /// is dropped before exhaustion, remaining items are also removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set from iterator
    /// let mut items = Items::from_iter([0, 1, 2]);
    ///
    /// // Create iterator over item set
    /// for index in items.drain() {
    ///     println!("{index:?}");
    /// }
    /// ```
    #[inline]
    pub fn drain(&mut self) -> Drain<'_> {
        let block = self.data[0];
        Drain {
            block,
            data: &mut self.data,
            index: 0,
        }
    }
}

// ----------------------------------------------------------------------------

impl Iterator for Drain<'_> {
    type Item = usize;

    /// Returns the next item.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.block != 0 {
                let num = self.block.trailing_zeros() as usize;
                self.data[self.index] &= self.data[self.index] - 1;

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

            self.block = self.data[self.index];
        }
    }
}

// ----------------------------------------------------------------------------

impl Drop for Drain<'_> {
    /// Removes all remaining items.
    fn drop(&mut self) {
        self.data[self.index..].fill(0);
    }
}
