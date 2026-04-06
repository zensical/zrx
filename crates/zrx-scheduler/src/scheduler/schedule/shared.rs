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

//! Shared value.

use std::cell::RefCell;
use std::rc::Rc;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Shared value.
///
/// This is essentially a utility wrapper around [`RefCell`] and [`Rc`], which
/// can be used to share a [`Builder`][] without awakening the borrow checker.
/// Note that it's only intended for use in single-threaded contexts, in order
/// to keep the overhead low, since it's only needed for the streaming API.
///
/// [`Builder`]: crate::scheduler::schedule::Builder
///
/// # Examples
///
/// ```
/// use zrx_scheduler::schedule::Shared;
///
/// // Create shared value
/// let value = Shared::new(42);
///
/// // Borrow mutably
/// value.with_mut(|inner| {
///     *inner += 1;
/// });
/// ```
#[derive(Debug, Default)]
pub struct Shared<T> {
    /// Inner value.
    inner: Rc<RefCell<T>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Shared<T> {
    /// Creates a shared value with the given value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::schedule::Shared;
    ///
    /// // Create shared value
    /// let value = Shared::new(42);
    /// ```
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            inner: Rc::new(RefCell::new(value)),
        }
    }

    /// Borrows the inner value and passes it to the given function.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::schedule::Shared;
    ///
    /// // Create shared value
    /// let value = Shared::new(42);
    ///
    /// // Borrow immutably
    /// value.with(|&inner| {
    ///     println!("{inner}");
    /// });
    /// ```
    #[inline]
    pub fn with<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        f(&*self.inner.borrow())
    }

    /// Mutably borrows the inner value and passes it to the given function.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::schedule::Shared;
    ///
    /// // Create shared value
    /// let value = Shared::new(42);
    ///
    /// // Borrow mutably
    /// value.with_mut(|inner| {
    ///     *inner += 1;
    /// });
    /// ```
    #[inline]
    pub fn with_mut<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        f(&mut *self.inner.borrow_mut())
    }

    /// Attempts to unwrap the inner value.
    ///
    /// This method tries to unwrap the inner value, which will only succeed if
    /// there's exactly one strong reference remaining.
    ///
    /// # Errors
    ///
    /// This method returns `Self` if there's more than one strong reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::schedule::Shared;
    ///
    /// // Create shared value
    /// let value = Shared::new(42);
    ///
    /// // Unwrap value
    /// if let Err(value) = value.try_into_inner() {
    ///     panic!("Could not unwrap {value:?}");
    /// }
    /// ```
    #[inline]
    pub fn try_into_inner(self) -> Result<T, Self> {
        Rc::try_unwrap(self.inner)
            .map_err(|inner| Self { inner })
            .map(RefCell::into_inner)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> AsRef<Shared<T>> for Shared<T> {
    /// Returns a reference to the shared value.
    #[inline]
    fn as_ref(&self) -> &Shared<T> {
        self
    }
}

// ----------------------------------------------------------------------------

impl<T> PartialEq for Shared<T> {
    /// Compares two shared values for equality.
    ///
    /// The inner value does not need to implement [`PartialEq`], since we only
    /// compare the pointers to the inner values, not the values themselves.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::schedule::Shared;
    ///
    /// // Create and compare shared values
    /// let a = Shared::new(42);
    /// let b = a.clone();
    /// assert_eq!(a, b);
    /// ```
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> Eq for Shared<T> {}

// ----------------------------------------------------------------------------

impl<T> Clone for Shared<T> {
    /// Clones the shared value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::schedule::Shared;
    ///
    /// // Create and clone shared value
    /// let value = Shared::new(42);
    /// value.clone();
    /// ```
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}
