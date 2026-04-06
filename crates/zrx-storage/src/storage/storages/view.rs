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

//! View of a storage set.

use std::any::Any;
use std::borrow::Cow;
use std::ptr;

use super::Storages;

mod borrow;
mod iter;

use borrow::IntoIndices;
pub use iter::Iter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// View of a storage set.
///
/// This data type provides a temporary view of a storage set, which is solely
/// intended to be used for downcasting storages via [`TryAsStorages`][].
///
/// [`TryAsStorages`]: crate::convert::TryAsStorages
#[derive(Debug)]
pub struct View<'a> {
    /// Storage set.
    storages: &'a Storages,
    /// Storage indices.
    indices: Cow<'a, [usize]>,
}

/// Mutable view of a storage set.
///
/// This data type provides a temporary view of a storage set, which is solely
/// intended to be used for downcasting storages via [`TryAsStorageMut`][].
///
/// [`TryAsStorageMut`]: crate::convert::TryAsStorageMut
#[derive(Debug)]
pub struct ViewMut<'a> {
    /// Storage set.
    storages: &'a mut Storages,
    /// Storage index.
    index: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Storages {
    /// Creates a view of a storage set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::Storages;
    ///
    /// // Create storage set and add storages
    /// let mut storages = Storages::default();
    /// let a = storages.insert([("key", 42)]);
    /// let b = storages.insert([("key", true)]);
    ///
    /// // Create view of storage set
    /// let view = storages.view([a, b]);
    /// ```
    #[inline]
    #[must_use]
    pub fn view<'a, I>(&'a self, indices: I) -> View<'a>
    where
        I: IntoIndices<'a>,
    {
        View {
            storages: self,
            indices: indices.into_indices(),
        }
    }

    /// Creates a mutable view of a storage set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::Storages;
    ///
    /// // Create storage set and add storages
    /// let mut storages = Storages::default();
    /// let a = storages.insert([("key", 42)]);
    /// let b = storages.insert([("key", true)]);
    ///
    /// // Create mutable view of storage set
    /// let view = storages.view_mut(a);
    /// ```
    #[inline]
    #[must_use]
    pub fn view_mut(&mut self, index: usize) -> ViewMut<'_> {
        ViewMut { storages: self, index }
    }

    /// Creates a split view (immutable + mutable) of a storage set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_storage::Storages;
    ///
    /// // Create storage set and add storages
    /// let mut storages = Storages::default();
    /// let a = storages.insert([("key", 42)]);
    /// let b = storages.insert([("key", true)]);
    ///
    /// // Create split view of storage set
    /// let views = storages.views([a], b);
    /// ```
    #[inline]
    #[must_use]
    pub fn views<'a, I>(&'a mut self, indices: I, index: usize) -> Views<'a>
    where
        I: IntoIndices<'a>,
    {
        let indices = indices.into_indices();
        debug_assert!(!indices.contains(&index));

        // SAFETY: At the beginning of this function, we've asserted that the
        // given index is not included in the list of indices. This allows us
        // to obtain a raw pointer to the storage set, so we can return both,
        // immutable and mutable references, at the same time.
        unsafe {
            let pointer = ptr::from_mut(self);
            (
                View { storages: &*pointer, indices },
                ViewMut { storages: &mut *pointer, index },
            )
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a> ViewMut<'a> {
    /// Returns the inner mutable reference, consuming the view.
    #[inline]
    #[must_use]
    pub fn into_inner(self) -> &'a mut dyn Any {
        &mut self.storages[self.index]
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl AsMut<dyn Any> for ViewMut<'_> {
    /// Returns the inner mutable reference.
    #[inline]
    fn as_mut(&mut self) -> &mut dyn Any {
        &mut self.storages[self.index]
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Split view (immutable + mutable) of a storage set.
pub type Views<'a> = (View<'a>, ViewMut<'a>);
