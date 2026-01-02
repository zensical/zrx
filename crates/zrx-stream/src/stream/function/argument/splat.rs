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

//! Splat argument.

use std::ptr;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Splat argument.
///
/// This is a marker trait that is used as a wrapper around tuples of arguments
/// allowing to invoke a function with multiple arguments as if it were a single
/// argument. Its purpose is to make writing operators with multiple arguments
/// more ergonomic, which is particularly useful for joins.
///
/// All function traits implement variations of [`Splat`] for tuples of up to 8
/// elements, which is likely to be sufficient for most use cases. Please note
/// that most of the time, this type is only ever needed for creating flexible
/// operator functions, so you'll rarely want to use it directly. Additionally,
/// note that [`Splat`] itself doesn't impose any trait bounds in order to not
/// require even more trait bounds in the signature of implementors. Moreover,
/// it doesn't implement [`Value`][], meaning it can only be used temporarily
/// inside operators, but never returned.
///
/// [`Value`]: zrx_scheduler::Value
///
/// # Examples
///
/// ```
/// use zrx_stream::function::Splat;
///
/// // Create splat from tuple
/// let splat = Splat::from((1, 2));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)] // do not remove
pub struct Splat<T> {
    /// Inner arguments.
    inner: T,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Splat<T> {
    /// Returns the inner tuple of arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::function::Splat;
    ///
    /// // Create splat from tuple
    /// let splat = Splat::from((1, 2));
    /// assert_eq!(splat.inner(), &(1, 2));
    /// ```
    #[inline]
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Returns the inner tuple of arguments, consuming the splat.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::function::Splat;
    ///
    /// // Create splat from tuple
    /// let splat = Splat::from((1, 2));
    /// assert_eq!(splat.into_inner(), (1, 2));
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> Splat<T> {
    /// Creates a splat argument from a reference to a tuple.
    ///
    /// This method converts `&T` to `&Splat<T>`, which allows us to use splat
    /// arguments in any function that takes a reference to a value `T` without
    /// cloning the value. While the conversion mandates the use of `unsafe`,
    /// it's safe due to identical memory layout guarantees.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::function::Splat;
    ///
    /// // Create splat from tuple reference
    /// let splat = Splat::from_ref(&(1, 2));
    /// ```
    #[inline]
    pub fn from_ref(inner: &T) -> &Self {
        // SAFETY: Safe, because `repr(transparent)` guarantees that the outer
        // type always has the exact same memory layout as the inner type
        unsafe { &*ptr::from_ref::<T>(inner).cast::<Splat<T>>() }
    }
}

// ----------------------------------------------------------------------------

impl<T> From<T> for Splat<T> {
    /// Creates a splat from a tuple of arguments.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::function::Splat;
    ///
    /// // Create splat from tuple
    /// let splat = Splat::from((1, 2));
    /// ```
    #[inline]
    fn from(inner: T) -> Self {
        Self { inner }
    }
}
