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

//! Value storage.

use std::borrow::Cow;
use std::rc::Rc;

use crate::scheduler::value::Value;

use super::Node;

mod view;

pub use view::View;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Value storage.
#[derive(Clone, Debug, Default)]
pub struct Storage {
    /// Vector of values.
    inner: Vec<Node<Rc<dyn Value>>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Storage {
    /// Creates a storage.
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }

    /// Gets the value with the given identifier.
    pub fn get(&self, id: usize) -> Option<&dyn Value> {
        self.position(id)
            .map(|index| self.inner[index].data.as_ref())
    }

    // Selects the values with the given identifiers.
    pub fn select<'a, I>(&'a self, ids: I) -> View<'a>
    where
        I: Into<Cow<'a, [usize]>>,
    {
        View::new(self, ids)
    }

    /// Appends the given value with the given identifier.
    pub fn append<V>(&mut self, id: usize, value: V)
    where
        V: Into<Rc<dyn Value>>,
    {
        self.inner.push(Node { id, data: value.into() });
    }

    /// Removes the value with the given identifier.
    pub fn remove(&mut self, id: usize) -> Option<Rc<dyn Value>> {
        self.position(id).map(|index| {
            let prior = self.inner.swap_remove(index);
            prior.data
        })
    }

    /// Returns the position of the value with the given identifier.
    #[inline]
    fn position(&self, id: usize) -> Option<usize> {
        self.inner.iter().position(|item| item.id == id)
    }
}

#[allow(clippy::must_use_candidate)]
impl Storage {
    /// Returns the number of values.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
