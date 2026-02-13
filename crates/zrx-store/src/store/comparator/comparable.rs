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

//! Comparable value.

use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::ops::Deref;

use super::{Ascending, Comparator};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Comparable value.
///
/// This data type is a thin wrapper around a value of type `T`, which allows
/// to set a custom comparator `C` to define the ordering of values in stores.
/// It implements the [`Eq`], [`PartialEq`], [`Ord`] and [`PartialOrd`] traits,
/// as well as [`Deref`], so the original value can be used transparently.
///
/// If `C` is a zero-sized type (ZST), and doesn't capture any variables, the
/// runtime overhead is zero, as the compiler is able to optimize it away.
///
/// # Examples
///
/// ```
/// use zrx_store::comparator::Comparable;
///
/// // Create and compare values
/// let a: Comparable<_> = 42.into();
/// let b: Comparable<_> = 84.into();
/// assert!(a < b);
/// ```
#[derive(Clone)]
pub struct Comparable<T, C = Ascending>(T, C);

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T, C> Comparable<T, C> {
    /// Creates a comparable value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::comparator::{Ascending, Comparable};
    ///
    /// // Create comparable value
    /// let value = Comparable::new(42, Ascending);
    /// assert_eq!(*value, 42);
    /// ```
    #[inline]
    pub fn new(value: T, comparator: C) -> Self {
        Comparable(value, comparator)
    }

    /// Returns the inner value, consuming the comparable value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::comparator::{Ascending, Comparable};
    ///
    /// // Create comparable value
    /// let value = Comparable::new(42, Ascending);
    /// assert_eq!(value.into_inner(), 42);
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<T> for Comparable<T> {
    /// Creates a comparable value from a value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::comparator::Comparable;
    ///
    /// // Create comparable value
    /// let value: Comparable<_> = 42.into();
    /// assert_eq!(*value, 42);
    /// ```
    #[inline]
    fn from(value: T) -> Self {
        Comparable(value, Ascending)
    }
}

// ----------------------------------------------------------------------------

impl<T, C> Borrow<T> for Comparable<T, C>
where
    C: Comparator<T>,
{
    /// Borrows the wrapped value.
    #[inline]
    fn borrow(&self) -> &T {
        &self.0
    }
}

impl<T, C> Deref for Comparable<T, C> {
    type Target = T;

    /// Dereferences to the wrapped value.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ----------------------------------------------------------------------------

impl<T, C> PartialEq for Comparable<T, C>
where
    T: PartialEq,
{
    /// Compares two values for equality.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::comparator::Comparable;
    ///
    /// // Create and compare values
    /// let a: Comparable<_> = 42.into();
    /// let b: Comparable<_> = 42.into();
    /// assert_eq!(a, b);
    /// ```
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T, C> Eq for Comparable<T, C> where T: Eq {}

// ----------------------------------------------------------------------------

impl<T, C> PartialOrd for Comparable<T, C>
where
    T: Eq,
    C: Comparator<T>,
{
    /// Orders two values.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::comparator::Comparable;
    ///
    /// // Create and compare values
    /// let a: Comparable<_> = 42.into();
    /// let b: Comparable<_> = 84.into();
    /// assert!(a < b);
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, C> Ord for Comparable<T, C>
where
    T: Eq,
    C: Comparator<T>,
{
    /// Orders two values.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_store::comparator::Comparable;
    ///
    /// // Create and compare values
    /// let a: Comparable<_> = 42.into();
    /// let b: Comparable<_> = 84.into();
    /// assert!(a < b);
    /// ```
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&self.0, &other.0)
    }
}

// ----------------------------------------------------------------------------

impl<T, C> fmt::Debug for Comparable<T, C>
where
    T: Debug,
{
    /// Formats the comparator for debugging.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
