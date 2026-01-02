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

//! Store key.

use std::hash::Hash;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Store key.
///
/// This trait defines the basic requirements for a key used in a [`Store`][].
/// We can't use specific traits, e.g., [`Eq`][] + [`Hash`][] for hash maps or
/// [`Ord`][] for ordered keys, since we would lose the ability to allow for
/// using [`Borrow`][] to generalize the key type.
///
/// Thus, keys must implement [`Clone`], [`Eq`], [`Hash`] and [`Ord`], which we
/// consider a reasonable requirement and a good trade-off for a generic API.
///
/// [`Borrow`]: std::borrow::Borrow
/// [`Store`]: crate::store::Store
pub trait Key: Clone + Eq + Hash + Ord {}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> Key for T where T: Clone + Eq + Hash + Ord {}
