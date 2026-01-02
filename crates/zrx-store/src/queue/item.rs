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

//! Queue item.

use std::cmp::Ordering;
use std::time::Instant;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Queue item.
///
/// Items are stored in queues, and can have arbitrary associated data. An item
/// has a specific deadline, which is the ordering property of the queue.
///
/// Items must only be considered for processing when their deadline has passed,
/// which is exactly what [`Queue::take`][] ensures. This allows to implement
/// timers and intervals in an efficient manner. In case two items have the
/// same deadline, order is undefined, but this doesn't matter for us.
///
/// Note that mutable data needs to be stored outside of the queue, as items are
/// immutable. The built-in [`Queue`][] uses a [`Slab`][] for this matter.
///
/// [`Queue`]: crate::store::queue::Queue
/// [`Queue::take`]: crate::store::queue::Queue::take
/// [`Slab`]: slab::Slab
#[derive(Clone, Debug)]
pub struct Item<T = usize> {
    /// Deadline.
    deadline: Instant,
    /// Associated data.
    data: T,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Item<T> {
    /// Creates a queue item.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Item;
    ///
    /// // Create queue item
    /// let item = Item::new(42);
    /// ```
    #[must_use]
    pub fn new(data: T) -> Self {
        Self { deadline: Instant::now(), data }
    }

    /// Updates the deadline of the queue item.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Instant;
    /// use zrx_store::queue::Item;
    ///
    /// // Create queue item and set deadline
    /// let mut item = Item::new(42);
    /// item.set_deadline(Instant::now());
    /// ```
    #[inline]
    pub fn set_deadline(&mut self, deadline: Instant) {
        self.deadline = deadline;
    }
}

// ----------------------------------------------------------------------------

impl<T> Item<T> {
    /// Returns the deadline.
    #[inline]
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// Returns a reference to the associated data.
    #[inline]
    pub fn data(&self) -> &T {
        &self.data
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> PartialEq for Item<T>
where
    T: Eq,
{
    /// Compares two queue items for equality.
    ///
    /// Note that two queue items are considered being equal if their associated
    /// data is equal. Deadlines are solely relevant for ordering.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Item;
    ///
    /// // Create and compare queue items
    /// let a = Item::new(42);
    /// let b = Item::new(42);
    /// assert_eq!(a, b);
    /// ```
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
    }
}

impl<T> Eq for Item<T> where T: Eq {}

// ----------------------------------------------------------------------------

impl<T> PartialOrd for Item<T>
where
    T: Eq,
{
    /// Orders two queue items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Item;
    ///
    /// // Create and compare queue items
    /// let a = Item::new(42);
    /// let b = Item::new(84);
    /// assert!(a <= b);
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Item<T>
where
    T: Eq,
{
    /// Orders two queue items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::queue::Item;
    ///
    /// // Create and compare queue items
    /// let a = Item::new(42);
    /// let b = Item::new(84);
    /// assert!(a <= b);
    /// ```
    fn cmp(&self, other: &Self) -> Ordering {
        self.deadline.cmp(&other.deadline)
    }
}
