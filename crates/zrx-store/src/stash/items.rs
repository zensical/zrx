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

//! Item set.

mod drain;
mod into_iter;
mod iter;

pub use drain::Drain;
pub use into_iter::IntoIter;
pub use iter::Iter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Item set.
///
/// This item set implementation is based on a minimal bitset implementation,
/// that allows to efficiently manage and work with item sets and stashes. It
/// mustn't be considered a complete implementation of general purpose bitsets,
/// but only provides the methods we need for efficient stash management.
///
/// Using a focused implementation allows us to optimize for our specific use
/// case, and avoids yet another dependency to manage.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Items {
    /// Blocks of bits.
    data: Vec<u64>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Items {
    /// Creates an item set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set
    /// let items = Items::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an item set with the given capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set with capacity
    /// let items = Items::with_capacity(128);
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        // Note that the number of bits is rounded up to the next multiple of
        // 64, so that the bitset can be represented as a vector of 64-bit
        // blocks. It also means that the bitset can store at least the given
        // number of bits, but possibly more.
        Self {
            data: Vec::with_capacity(capacity.div_ceil(64)),
        }
    }

    /// Returns whether the item set contains the given item.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set
    /// let items = Items::from_iter([1]);
    ///
    /// // Ensure presence of items
    /// assert_eq!(items.contains(0), false);
    /// assert_eq!(items.contains(1), true);
    /// ```
    #[inline]
    #[must_use]
    pub fn contains(&self, index: usize) -> bool {
        let opt = self.data.get(index >> 6);
        opt.is_some_and(|&block| (block & mask(index)) != 0)
    }

    /// Inserts an item into the item set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set
    /// let mut items = Items::new();
    ///
    /// // Insert item
    /// assert_eq!(items.insert(0), true);
    /// assert_eq!(items.insert(0), false);
    /// ```
    #[inline]
    pub fn insert(&mut self, index: usize) -> bool {
        let group = self.resolve(index);
        if (self.data[group] & mask(index)) == 0 {
            self.data[group] |= mask(index);
            true
        } else {
            false
        }
    }

    /// Removes an item from the item set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set
    /// let mut items = Items::from_iter([0, 1]);
    ///
    /// // Remove item
    /// assert_eq!(items.remove(0), true);
    /// assert_eq!(items.remove(0), false);
    /// ```
    #[inline]
    pub fn remove(&mut self, index: usize) -> bool {
        let group = index >> 6;
        let Some(block) = self.data.get_mut(group) else {
            return false;
        };

        // Ensure presence of item before removing it
        if (*block & mask(index)) == 0 {
            return false;
        }

        // Remove the item and truncate trailing blocks
        *block &= !mask(index);
        if *block == 0 && group + 1 == self.data.len() {
            self.truncate();
        }
        true
    }

    /// Clears all items in the item set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set
    /// let mut items = Items::from_iter([0, 1, 2]);
    ///
    /// // Remove all items
    /// items.clear();
    /// assert!(items.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Computes the union with the given item set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create two item sets
    /// let mut a = Items::from_iter([0, 1]);
    /// let mut b = Items::from_iter([1, 2]);
    ///
    /// // Create union of item sets
    /// a.union(&b);
    /// assert_eq!(a, Items::from_iter([0, 1, 2]));
    /// ```
    pub fn union(&mut self, other: &Self) {
        self.data.resize(other.data.len().max(self.data.len()), 0);
        for (a, b) in self.data.iter_mut().zip(&other.data) {
            *a |= *b;
        }
    }

    /// Computes the intersection with the given item set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create two item sets
    /// let mut a = Items::from_iter([0, 1]);
    /// let mut b = Items::from_iter([1, 2]);
    ///
    /// // Create intersection of item sets
    /// a.intersect(&b);
    /// assert_eq!(a, Items::from_iter([1]));
    /// ```
    pub fn intersect(&mut self, other: &Self) {
        self.data.truncate(other.data.len());
        for (a, b) in self.data.iter_mut().zip(&other.data) {
            *a &= *b;
        }

        // Truncate trailing blocks that contain no items
        self.truncate();
    }

    /// Returns whether any of the given items is present.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create two item sets
    /// let mut a = Items::from_iter([0, 1]);
    /// let mut b = Items::from_iter([1, 2]);
    ///
    /// // Ensure presence of items
    /// assert!(b.has_any(&a));
    /// ```
    #[inline]
    #[must_use]
    pub fn has_any(&self, other: &Self) -> bool {
        let mut iter = self.data.iter().zip(&other.data);
        iter.any(|(&a, &b)| (a & b) != 0)
    }

    /// Returns whether the given items are all present.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create two item sets
    /// let mut a = Items::from_iter([0, 1]);
    /// let mut b = Items::from_iter([0, 1, 2]);
    ///
    /// // Ensure presence of items
    /// assert!(b.has_all(&a));
    /// ```
    #[inline]
    #[must_use]
    pub fn has_all(&self, other: &Self) -> bool {
        let mut iter = other.data.iter().enumerate();
        iter.all(|(a, &b)| (self.data.get(a).unwrap_or(&0) & b) == b)
    }

    /// Resolve the block for the given item.
    ///
    /// This method ensures that the item set has enough blocks to accommodate
    /// the given item, resizing the underlying vector if necessary.
    fn resolve(&mut self, index: usize) -> usize {
        let block = index >> 6;
        if block >= self.data.len() {
            let blocks = block + 1;
            self.data.resize(blocks, 0);
        }
        block
    }

    /// Truncates all trailing blocks that contain no items.
    fn truncate(&mut self) {
        let opt = self.data.iter().rposition(|&block| block != 0);
        self.data.truncate(opt.map_or(0, |index| index + 1));
    }
}

#[allow(clippy::must_use_candidate)]
impl Items {
    /// Returns the number of items.
    #[inline]
    pub fn len(&self) -> usize {
        let iter = self.data.iter();
        iter.map(|block| block.count_ones() as usize).sum()
    }

    /// Returns whether there are any items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl FromIterator<usize> for Items {
    /// Creates an item set from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Items;
    ///
    /// // Create item set from iterator
    /// let items = Items::from_iter([0, 1]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = usize>,
    {
        let mut items = Items::new();
        for index in iter {
            items.insert(index);
        }
        items
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Returns the mask for the given index.
#[inline]
const fn mask(index: usize) -> u64 {
    1 << (index & 63)
}
