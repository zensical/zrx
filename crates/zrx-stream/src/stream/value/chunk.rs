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

//! Chunk of items.

use std::slice::Iter;
use std::vec::IntoIter;

use zrx_scheduler::effect::Item;
use zrx_scheduler::{Id, Value};

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Chunk of items.
///
/// This data type represents a collection of items that are grouped together
/// for processing. Unlike [`Delta`][], a [`Chunk`] contains items that are all
/// present (i.e., not optional). Chunks are used to process a batch of items
/// as a single unit, e.g., for windowing or pagination.
///
/// Note that chunks are assumed to always only contain unique items, meaning
/// there are no two items with the same identifier in a chunk. This invariant
/// must be checked with unit tests (which we provide for our operators), but
/// is not enforced at runtime for performance reasons.
///
/// [`Delta`]: crate::stream::value::Delta
///
/// # Examples
///
/// ```
/// use zrx_scheduler::effect::Item;
/// use zrx_stream::value::Chunk;
///
/// // Create chunk of items
/// let chunk = Chunk::from([
///     Item::new("a", 1),
///     Item::new("b", 2),
///     Item::new("c", 3),
/// ]);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chunk<I, T> {
    /// Vector of items.
    inner: Vec<Item<I, T>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Chunk<I, T> {
    /// Creates an iterator over the chunk of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Chunk;
    ///
    /// // Create chunk of items
    /// let chunk = Chunk::from([
    ///     Item::new("a", 1),
    ///     Item::new("b", 2),
    ///     Item::new("c", 3),
    /// ]);
    ///
    /// // Create iterator over items
    /// for item in chunk.iter() {
    ///     println!("{item:?}");
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<'_, Item<I, T>> {
        self.inner.iter()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Value for Chunk<I, T>
where
    I: Id,
    T: Value,
{
}

// ----------------------------------------------------------------------------

impl<I, T, U, const N: usize> From<[U; N]> for Chunk<I, T>
where
    U: Into<Item<I, T>>,
{
    /// Creates a chunk of items from a slice of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Chunk;
    ///
    /// // Create chunk of items
    /// let chunk = Chunk::from([
    ///     Item::new("a", 1),
    ///     Item::new("b", 2),
    ///     Item::new("c", 3),
    /// ]);
    /// ```
    #[inline]
    fn from(value: [U; N]) -> Self {
        Self::from_iter(value)
    }
}

// ----------------------------------------------------------------------------

impl<I, T, U> FromIterator<U> for Chunk<I, T>
where
    U: Into<Item<I, T>>,
{
    /// Creates a chunk of items from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Chunk;
    ///
    /// // Create chunk of items
    /// let chunk = Chunk::from_iter([
    ///     Item::new("a", 1),
    ///     Item::new("b", 2),
    ///     Item::new("c", 3),
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

impl<I, T> IntoIterator for Chunk<I, T> {
    type Item = Item<I, T>;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates an iterator over the chunk of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Chunk;
    ///
    /// // Create chunk of items
    /// let chunk = Chunk::from([
    ///     Item::new("a", 1),
    ///     Item::new("b", 2),
    ///     Item::new("c", 3),
    /// ]);
    ///
    /// // Create iterator over items
    /// for item in chunk.iter() {
    ///     println!("{item:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, I, T> IntoIterator for &'a Chunk<I, T> {
    type Item = &'a Item<I, T>;
    type IntoIter = Iter<'a, Item<I, T>>;

    /// Creates an iterator over the chunk of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    /// use zrx_stream::value::Chunk;
    ///
    /// // Create chunk of items
    /// let chunk = Chunk::from([
    ///     Item::new("a", 1),
    ///     Item::new("b", 2),
    ///     Item::new("c", 3),
    /// ]);
    ///
    /// // Create iterator over items
    /// for item in &chunk {
    ///     println!("{item:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Default for Chunk<I, T> {
    /// Creates an empty chunk of items.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::value::Chunk;
    ///
    /// // Create empty chunk of items
    /// let chunk = Chunk::default();
    /// # let _: Chunk<(), ()> = chunk;
    /// ```
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::default() }
    }
}
