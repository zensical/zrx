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

//! Iterator implementations for [`View`].

use std::any::Any;
use std::borrow::Cow;

use super::{Storages, View};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over the items of a [`View`].
///
/// This data type uses a [`Cow`] instead of an iterator over a [`usize`] slice
/// to allow for providing an [`IntoIterator`] implementation without cloning.
#[derive(Debug)]
pub struct Iter<'a> {
    /// Storage set.
    storages: &'a Storages,
    /// Storage indices.
    selected: Cow<'a, [usize]>,
    /// Current index.
    index: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl View<'_> {
    /// Creates an iterator over the items of the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage set and add storages
    /// let mut storages = Storages::new();
    /// let a = storages.insert([("key", 42)]);
    /// let b = storages.insert([("key", true)]);
    ///
    /// // Create view of storage set
    /// let view = storages.view([a, b]);
    /// for item in view.iter() {
    ///     // ...
    /// }
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            storages: self.storages,
            selected: self.indices.clone(),
            index: 0,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> Iterator for Iter<'a> {
    type Item = &'a dyn Any;

    /// Returns the next item.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.selected.get(self.index) {
            None => None,
            Some(&index) => {
                self.index += 1;
                Some(&self.storages[index])
            }
        }
    }

    /// Returns the bounds on the remaining length of the iterator.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for Iter<'_> {
    /// Returns the exact remaining length of the iterator.
    #[inline]
    fn len(&self) -> usize {
        self.selected.len() - self.index
    }
}

// ----------------------------------------------------------------------------

impl<'a> IntoIterator for &'a View<'a> {
    type Item = &'a dyn Any;
    type IntoIter = Iter<'a>;

    /// Creates an iterator over the items of the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage set and add storages
    /// let mut storages = Storages::new();
    /// let a = storages.insert([("key", 42)]);
    /// let b = storages.insert([("key", true)]);
    ///
    /// // Create view of storage set
    /// let view = storages.view([a, b]);
    /// for item in &view {
    ///     // ...
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for View<'a> {
    type Item = &'a dyn Any;
    type IntoIter = Iter<'a>;

    /// Creates a consuming iterator over the view.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::{Storage, Storages};
    ///
    /// // Create storage set and add storages
    /// let mut storages = Storages::new();
    /// let a = storages.insert([("key", 42)]);
    /// let b = storages.insert([("key", true)]);
    ///
    /// // Create view of storage set
    /// let view = storages.view([a, b]);
    /// for item in view {
    ///     // ...
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter {
            storages: self.storages,
            selected: self.indices,
            index: 0,
        }
    }
}
