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

//! Storage indices conversions.

use std::borrow::Cow;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into a [`usize`] slice.
pub trait IntoIndices<'a> {
    /// Converts into a borrowed or owned slice.
    fn into_indices(self) -> Cow<'a, [usize]>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> IntoIndices<'a> for &'a [usize] {
    /// Converts the slice into a borrowed slice.
    #[inline]
    fn into_indices(self) -> Cow<'a, [usize]> {
        Cow::Borrowed(self)
    }
}

impl<'a, const N: usize> IntoIndices<'a> for [usize; N] {
    /// Converts the array into an owned slice.
    #[inline]
    fn into_indices(self) -> Cow<'a, [usize]> {
        Cow::Owned(self.into())
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<'a, T> IntoIndices<'a> for &'a T
where
    T: AsRef<[usize]>,
{
    #[inline]
    fn into_indices(self) -> Cow<'a, [usize]> {
        Cow::Borrowed(self.as_ref())
    }
}
