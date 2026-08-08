// ----------------------------------------------------------------------------

//! Drain iterator implementation for [`Matches`].

use super::Matches;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Drain iterator for [`Matches`].
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

impl Matches {
    /// Creates a drain iterator over the match set.
    ///
    /// Removes all matches from the set as they are yielded. If the iterator
    /// is dropped before exhaustion, remaining matches are also removed.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::matcher::Matches;
    ///
    /// // Create match set from iterator
    /// let mut matches = Matches::from_iter([0, 1, 2]);
    ///
    /// // Create iterator over match set
    /// for index in matches.drain() {
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
            let &block = self.data.get(self.index)?;
            if block != 0 {
                let num = block.trailing_zeros() as usize;

                // Clear the lowest bit and return it
                self.data[self.index] = block & (block - 1);
                return Some(self.index << 6 | num);
            }

            // Move to the next block
            self.index += 1;
        }
    }
}

// ----------------------------------------------------------------------------

impl Drop for Drain<'_> {
    /// Removes all remaining matches.
    fn drop(&mut self) {
        self.data.clear();
    }
}
