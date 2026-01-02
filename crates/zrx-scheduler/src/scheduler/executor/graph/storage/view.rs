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

//! Value view.

use std::borrow::Cow;

use crate::scheduler::value::Value;

use super::Storage;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Value view.
#[derive(Clone, Debug)]
pub struct View<'a> {
    /// Value storage.
    storage: &'a Storage,
    /// Value identifiers.
    ids: Cow<'a, [usize]>,
    /// View offset.
    offset: usize,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a> View<'a> {
    /// Creates a value view.
    #[must_use]
    pub fn new<I>(storage: &'a Storage, ids: I) -> Self
    where
        I: Into<Cow<'a, [usize]>>,
    {
        Self {
            storage,
            ids: ids.into(),
            offset: 0,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> Iterator for View<'a> {
    type Item = Option<&'a dyn Value>;

    /// Returns the next optional value reference.
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset < self.ids.len() {
            let index = self.ids[self.offset];
            self.offset += 1;
            Some(self.storage.get(index))
        } else {
            None
        }
    }

    /// Returns the bounds of the value view.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.ids.len(), Some(self.ids.len()))
    }
}

impl ExactSizeIterator for View<'_> {
    /// Returns the length of the value view.
    #[inline]
    fn len(&self) -> usize {
        self.ids.len()
    }
}
