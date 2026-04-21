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

//! Scope set.

use std::vec::IntoIter;

use crate::scheduler::step::Scope;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scope set.
#[derive(Debug)]
pub struct Scopes<I> {
    /// Inner set of scopes.
    inner: Vec<Scope<I>>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<S, I> FromIterator<S> for Scopes<I>
where
    S: Into<Scope<I>>,
{
    /// Creates a scope set from an iterator.
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = S>,
    {
        let iter = iter.into_iter().map(Into::into);
        Self { inner: iter.collect() }
    }
}

impl<I> IntoIterator for Scopes<I> {
    type Item = Scope<I>;
    type IntoIter = IntoIter<Self::Item>;

    /// Converts the scope set into an iterator.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
