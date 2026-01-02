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

//! Delta of items.

use std::slice::Iter;
use std::vec::IntoIter;

use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Delta of items.
///
/// This data type represents a collection of items that can include insertions
/// and deletions (if [`Option`] is [`None`]), which we call deltas. Deltas are
/// the means of differentially passing changes through a stream. They must be
/// applied to a store to manifest the changes they represent.
///
/// Note that deltas are assumed to always only contain unique items, meaning
/// there are no two items with the same identifier in a delta. This invariant
/// must be checked with unit tests (which we provide for our operators), but
/// is not enforced at runtime for performance reasons. Differential semantics
/// still hold if this invariant is violated, but performance might be impacted
/// due to unnecessary re-computations.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::effect::Item;
/// use zrx_stream::value::Delta;
///
/// // Create delta of items
/// let delta = Delta::from([
///     Item::new("a", Some(1)),
///     Item::new("b", Some(2)),
///     Item::new("c", Some(3)),
/// ]);
/// ```
#[derive(Clone, Debug)]
pub struct Delta<I, T> {
    /// Vector of items.
    inner: Vec<Item<I, Option<T>>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Delta<I, T> {
    /// Creates an iterator over the delta of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Delta;
    ///
    /// // Create delta of items
    /// let delta = Delta::from([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    ///
    /// // Create iterator over items
    /// for item in delta.iter() {
    ///     println!("{item:?}");
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, Item<I, Option<T>>> {
        self.inner.iter()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Value for Delta<I, T>
where
    I: Id,
    T: Value,
{
}

// ----------------------------------------------------------------------------

impl<I, T, U, const N: usize> From<[U; N]> for Delta<I, T>
where
    U: Into<Item<I, Option<T>>>,
{
    /// Creates a delta of items from a slice of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Delta;
    ///
    /// // Create delta of items
    /// let delta = Delta::from([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    /// ```
    #[inline]
    fn from(value: [U; N]) -> Self {
        Self::from_iter(value)
    }
}

// ----------------------------------------------------------------------------

impl<I, T, U> FromIterator<U> for Delta<I, T>
where
    U: Into<Item<I, Option<T>>>,
{
    /// Creates a delta of items from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Delta;
    ///
    /// // Create delta of items
    /// let delta = Delta::from_iter([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    /// ```
    #[inline]
    fn from_iter<V>(iter: V) -> Self
    where
        V: IntoIterator<Item = U>,
    {
        Self {
            inner: iter.into_iter().map(Into::into).collect(),
        }
    }
}

impl<I, T> IntoIterator for Delta<I, T> {
    type Item = Item<I, Option<T>>;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates an iterator over the delta of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Delta;
    ///
    /// // Create delta of items
    /// let delta = Delta::from([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    ///
    /// // Create iterator over items
    /// for item in delta {
    ///     println!("{item:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, I, T> IntoIterator for &'a Delta<I, T> {
    type Item = &'a Item<I, Option<T>>;
    type IntoIter = Iter<'a, Item<I, Option<T>>>;

    /// Creates an iterator over the delta of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Delta;
    ///
    /// // Create delta of items
    /// let delta = Delta::from([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    ///
    /// // Create iterator over items
    /// for item in &delta {
    ///     println!("{item:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Default for Delta<I, T> {
    /// Creates an empty delta of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::value::Delta;
    ///
    /// // Create empty delta of items
    /// let delta = Delta::default();
    /// # let _: Delta<(), ()> = delta;
    /// ```
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::default() }
    }
}
