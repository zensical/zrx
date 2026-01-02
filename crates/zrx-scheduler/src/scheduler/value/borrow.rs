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

//! Value ownership.

use std::borrow::ToOwned;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Obtain ownership of a [`Value`][].
///
/// This trait is used to convert borrowed values into owned values, which is
/// necessary to allow the scheduler to transfer ownership of values, e.g., to
/// move them onto a worker thread or into a different context. It has to be
/// implemented for all types that implement [`TryFromValues`][].
///
/// Note that it's not possible to just implement [`ToOwned`], as not all outer
/// representations are borrowed, like vectors and tuples of references.
///
/// [`TryFromValues`]: crate::scheduler::value::TryFromValues
/// [`Value`]: crate::scheduler::value::Value
pub trait IntoOwned {
    /// Owned type of the value.
    type Owned;

    /// Obtains ownership of the value.
    fn into_owned(self) -> Self::Owned;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> IntoOwned for &T
where
    T: ToOwned,
{
    type Owned = T::Owned;

    /// Obtains ownership of the reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::value::IntoOwned;
    ///
    /// // Create value and obtain ownership
    /// let value = (&42).into_owned();
    /// assert_eq!(value, 42);
    /// ```
    #[inline]
    fn into_owned(self) -> Self::Owned {
        self.to_owned()
    }
}

impl<T> IntoOwned for Option<&T>
where
    T: ToOwned,
{
    type Owned = Option<T::Owned>;

    /// Obtains ownership of the optional reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::value::IntoOwned;
    ///
    /// // Create value and obtain ownership
    /// let value = Some(&42).into_owned();
    /// assert_eq!(value, Some(42));
    /// ```
    #[inline]
    fn into_owned(self) -> Self::Owned {
        self.map(ToOwned::to_owned)
    }
}

// ----------------------------------------------------------------------------

impl<T> IntoOwned for Vec<T>
where
    T: IntoOwned,
{
    type Owned = Vec<T::Owned>;

    /// Obtains ownership of the vector of values.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::value::IntoOwned;
    ///
    /// // Create value and obtain ownership
    /// let value = vec![&1, &2, &3].into_owned();
    /// assert_eq!(value, vec![1, 2, 3]);
    /// ```
    fn into_owned(self) -> Self::Owned {
        self.into_iter().map(IntoOwned::into_owned).collect()
    }
}

impl IntoOwned for () {
    type Owned = ();

    /// Obtains ownership of the unit value.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::value::IntoOwned;
    ///
    /// // Create value and obtain ownership
    /// let value = ().into_owned();
    /// assert_eq!(value, ());
    /// ```
    #[inline]
    fn into_owned(self) -> Self::Owned {}
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements value ownership trait for a tuple
macro_rules! impl_into_owned_for_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<$($T),+> IntoOwned for ($($T,)+)
        where
            $($T: IntoOwned,)+
        {
            type Owned = ($($T::Owned,)+);

            #[inline]
            fn into_owned(self) -> Self::Owned {
                #[allow(non_snake_case)]
                let ($($T,)+) = self;
                ($($T.into_owned(),)+)
            }
        }
    }
}

// ----------------------------------------------------------------------------

impl_into_owned_for_tuple!(T1);
impl_into_owned_for_tuple!(T1, T2);
impl_into_owned_for_tuple!(T1, T2, T3);
impl_into_owned_for_tuple!(T1, T2, T3, T4);
impl_into_owned_for_tuple!(T1, T2, T3, T4, T5);
impl_into_owned_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_into_owned_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_into_owned_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
