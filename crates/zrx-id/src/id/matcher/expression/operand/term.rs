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

//! Term.

use std::fmt;

use crate::id::matcher::Selector;
use crate::id::Id;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Term.
#[derive(Clone, PartialEq, Eq)]
pub enum Term {
    /// Identifier.
    Id(Id),
    /// Selector.
    Selector(Selector),
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<Id> for Term {
    /// Creates a term from the given identifier.
    #[inline]
    fn from(id: Id) -> Self {
        Self::Id(id)
    }
}

impl From<Selector> for Term {
    /// Creates a term from the given selector.
    #[inline]
    fn from(selector: Selector) -> Self {
        Self::Selector(selector)
    }
}

// ----------------------------------------------------------------------------

impl fmt::Debug for Term {
    /// Formats the term for debugging.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => id.fmt(f),
            Self::Selector(selector) => selector.fmt(f),
        }
    }
}
