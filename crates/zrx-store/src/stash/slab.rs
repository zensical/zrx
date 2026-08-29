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

//! Generational slab.

use slab;
use std::ops::{Index, IndexMut};

mod iter;
pub mod map;
mod slot;

pub use iter::{Iter, IterMut};
pub use map::Map;
pub use slot::Slot;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Generational slab.
///
/// This data type decorates a [`Slab`][] with a generation assigned to each
/// insertion. It deliberately exposes only operations that preserve generated
/// identities, rather than a map interface whose insertion semantics require a
/// caller-provided key.
///
/// Insertion, removal and access are constant-time. Iteration follows the
/// underlying [`Slab`][] order, which is stable but not sorted by generation.
///
/// [`Slab`]: slab::Slab
///
/// # Examples
///
/// ```
/// use zrx_store::stash::Slab;
///
/// // Create slab and initial state
/// let mut slab = Slab::default();
/// slab.insert(4);
/// slab.insert(2);
/// slab.insert(3);
/// slab.insert(1);
///
/// // Create iterator over slab
/// for (slot, value) in &slab {
///     println!("[{slot}]: {value}");
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Slab<T> {
    /// Underlying slab.
    inner: slab::Slab<Entry<T>>,
    /// Next generation.
    generation: u64,
}

// ----------------------------------------------------------------------------

/// Generational slab entry.
#[derive(Clone, Debug)]
struct Entry<T> {
    /// Slab value.
    value: T,
    /// Slab generation.
    generation: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Slab<T> {
    /// Creates a generational slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab
    /// let mut slab = Slab::new();
    ///
    /// // Insert value
    /// slab.insert(42);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a reference to the value in the slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Obtain reference to value
    /// let value = slab.get(slot);
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, slot: Slot) -> Option<&T> {
        let entry = self.inner.get(slot.index())?;
        (entry.generation == slot.generation()) // fmt
            .then_some(&entry.value)
    }

    /// Returns a mutable reference to the value in the slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Obtain mutable reference to value
    /// let value = slab.get_mut(slot);
    /// assert_eq!(value, Some(&mut 42));
    /// ```
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, slot: Slot) -> Option<&mut T> {
        let entry = self.inner.get_mut(slot.index())?;
        (entry.generation == slot.generation()) // fmt
            .then_some(&mut entry.value)
    }

    /// Returns two distinct mutable references to the values in the slots.
    ///
    /// # Panics
    ///
    /// Panics if both slots refer to the same underlying slab index.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let a = slab.insert(42);
    /// let b = slab.insert(84);
    ///
    /// // Obtain mutable references to values
    /// let values = slab.get2_mut(a, b);
    /// assert_eq!(values, Some((&mut 42, &mut 84)));
    /// ```
    #[inline]
    #[must_use]
    pub fn get2_mut(&mut self, a: Slot, b: Slot) -> Option<(&mut T, &mut T)> {
        let (s, t) = self.inner.get2_mut(a.index(), b.index())?;
        (s.generation == a.generation() && t.generation == b.generation())
            .then_some((&mut s.value, &mut t.value))
    }

    /// Returns whether the slab contains the slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Ensure presence of value
    /// let check = slab.contains(slot);
    /// assert_eq!(check, true);
    /// ```
    #[inline]
    #[must_use]
    pub fn contains(&self, slot: Slot) -> bool {
        self.get(slot).is_some()
    }

    /// Inserts the value and returns its slot.
    ///
    /// # Panics
    ///
    /// Panics if the [`u64`] generation counter overflows.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    ///
    /// // Insert value
    /// let slot = slab.insert(42);
    ///
    /// // Obtain reference to value
    /// let value = slab.get(slot);
    /// assert_eq!(value, Some(&42));
    /// ```
    #[inline]
    #[must_use]
    pub fn insert(&mut self, value: T) -> Slot {
        let generation = self.generation;
        self.generation = generation.checked_add(1).expect("invariant");

        // Obtain vacant entry and create slot with assigned generation
        let vacant = self.inner.vacant_entry();
        let slot = Slot::from_parts(vacant.key(), generation);

        // Insert value created with assigned slot
        vacant.insert(Entry { value, generation });
        slot
    }

    /// Removes the value identified by the slot.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Remove and return value
    /// let value = slab.remove(slot);
    /// assert_eq!(value, Some(42));
    /// ```
    pub fn remove(&mut self, slot: Slot) -> Option<T> {
        let entry = self.inner.get(slot.index())?;
        if entry.generation == slot.generation() {
            self.inner.try_remove(slot.index()).map(|entry| entry.value)
        } else {
            None
        }
    }

    /// Clears the slab, removing all items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Remove all items
    /// slab.clear();
    /// assert!(slab.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.inner.clear();
    }
}

#[allow(clippy::must_use_candidate)]
impl<T> Slab<T> {
    /// Returns the number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> Index<Slot> for Slab<T> {
    type Output = T;

    /// Returns a reference to the value in the slot.
    ///
    /// # Panics
    ///
    /// Panics if the slot is out of bounds or stale.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Obtain reference to value
    /// let value = &slab[slot];
    /// assert_eq!(value, &42);
    /// ```
    #[inline]
    fn index(&self, slot: Slot) -> &Self::Output {
        self.get(slot).expect("invalid slot")
    }
}

impl<T> IndexMut<Slot> for Slab<T> {
    /// Returns a mutable reference to the value in the slot.
    ///
    /// # Panics
    ///
    /// Panics if the slot is out of bounds or stale.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// let slot = slab.insert(42);
    ///
    /// // Obtain mutable reference to value
    /// let value = &mut slab[slot];
    /// assert_eq!(value, &mut 42);
    /// ```
    #[inline]
    fn index_mut(&mut self, slot: Slot) -> &mut Self::Output {
        self.get_mut(slot).expect("invalid slot")
    }
}

// ----------------------------------------------------------------------------

impl<'a, T> IntoIterator for &'a Slab<T> {
    type Item = (Slot, &'a T);
    type IntoIter = Iter<'a, T>;

    /// Creates an iterator over the slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// slab.insert(42);
    ///
    /// // Create iterator over slab
    /// for (slot, value) in &slab {
    ///     println!("[{slot}]: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Slab<T> {
    type Item = (Slot, &'a mut T);
    type IntoIter = IterMut<'a, T>;

    /// Creates a mutable iterator over the slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab and initial state
    /// let mut slab = Slab::default();
    /// slab.insert(42);
    ///
    /// // Create mutable iterator over slab
    /// for (slot, value) in &mut slab {
    ///     println!("[{slot}]: {value}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

// ----------------------------------------------------------------------------

impl<T> Default for Slab<T> {
    /// Creates a generational slab.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::stash::Slab;
    ///
    /// // Create slab
    /// let mut slab = Slab::default();
    ///
    /// // Insert value
    /// slab.insert(42);
    /// ```
    #[inline]
    fn default() -> Self {
        Self {
            inner: slab::Slab::default(),
            generation: 0,
        }
    }
}
