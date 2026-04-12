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
        Drain { data: &mut self.data, index: 0 }
    }
}

// ----------------------------------------------------------------------------

impl Iterator for Drain<'_> {
    type Item = usize;

    /// Returns the next item.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let block = self.data[self.index];
            if block != 0 {
                let num = block.trailing_zeros() as usize;
                self.data[self.index] = block & (block - 1);

                // Clear the lowest bit and return it
                return Some(self.index << 6 | num);
            }

            // Move to the next block
            self.index += 1;

            // If all blocks are exhausted, we're done
            if self.index >= self.data.len() {
                return None;
            }
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
