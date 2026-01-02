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

//! Barrier.

use ahash::HashMap;
use std::collections::hash_map::Iter;

use zrx_scheduler::Id;

mod condition;

pub use condition::Condition;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Barrier.
///
/// Barriers can be used to synchronize multiple streams. They are constructed
/// from a [`Condition`] that determines whether an identifier is part of the
/// barrier. If a barrier becomes empty, it is fulfilled, as all identifiers
/// were seen, so the operator can emit the corresponding [`Item`][].
///
/// [`Item`]: zrx_scheduler::effect::Item
///
/// # Examples
///
/// ```
/// use zrx_stream::barrier::{Barrier, Condition};
///
/// // Create barrier and insert identifier
/// let condition = Condition::new(|&id: &i32| id < 100);
/// let mut barrier = Barrier::new(condition);
/// barrier.insert(&42);
///
/// // Remove identifier and ensure it was present
/// let check = barrier.remove(&42);
/// assert_eq!(check, Some(false));
/// ```
#[derive(Debug)]
pub struct Barrier<I> {
    /// Barrier condition.
    condition: Condition<I>,
    /// Barrier items.
    items: HashMap<I, bool>,
    /// Number of observed items.
    fulfilled: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Barrier<I>
where
    I: Id,
{
    /// Creates a barrier with the given condition.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::barrier::{Barrier, Condition};
    ///
    /// // Create condition and barrier
    /// let condition = Condition::new(|&id: &i32| id < 100);
    /// let mut barrier = Barrier::new(condition);
    /// ```
    #[must_use]
    pub fn new(condition: Condition<I>) -> Self {
        Self {
            condition,
            items: HashMap::default(),
            fulfilled: 0,
        }
    }

    /// Inserts the identifier into the barrier if it satisfies the condition.
    #[inline]
    pub fn insert(&mut self, id: &I) -> Option<bool> {
        self.condition.satisfies(id).then(|| {
            let value = self.items.insert(id.clone(), false);
            if let Some(true) = value {
                self.fulfilled -= 1;
            }
            value.unwrap_or(true)
        })
    }

    /// Removes an identifier from the barrier if it satisfies the condition.
    #[inline]
    pub fn remove(&mut self, id: &I) -> Option<bool> {
        self.condition.satisfies(id).then(|| {
            let value = self.items.remove(id);
            if let Some(true) = value {
                self.fulfilled -= 1;
            }
            value.unwrap_or(true)
        })
    }

    /// Returns true if the item was fulfilled
    #[inline]
    pub fn notify(&mut self, id: &I) -> Option<bool> {
        match self.items.get_mut(id) {
            None => None,
            Some(true) => Some(false),
            Some(false) => {
                self.items.insert(id.clone(), true);
                self.fulfilled += 1;
                Some(true)
            }
        }
    }

    /// Checks whether the identifier satisfies the barrier condition.
    #[inline]
    pub fn satisfies(&self, id: &I) -> bool {
        self.condition.satisfies(id)
    }

    /// Creates an iterator over the barrier.
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_, I, bool> {
        self.items.iter()
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Barrier<I> {
    /// Returns the number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns whether there are any items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns whether the barrier is fulfilled.
    #[inline]
    pub fn is_fulfilled(&self) -> bool {
        self.items.len() == self.fulfilled
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I> IntoIterator for &'a Barrier<I>
where
    I: Id,
{
    type Item = (&'a I, &'a bool);
    type IntoIter = Iter<'a, I, bool>;

    /// Creates an iterator over the barrier.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
