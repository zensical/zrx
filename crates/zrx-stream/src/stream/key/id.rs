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

//! Stream identifier.

use std::fmt::{Debug, Display};
use std::hash::Hash;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// One component of a hierarchical stream [`Key`].
///
/// Stream identifiers carry the capabilities needed by keyed operators:
/// equality, hashing and ordering for retained state, formatting for
/// diagnostics and tracing, and thread-safe ownership for execution.
///
/// This contract deliberately belongs to the stream layer. The scheduler only
/// transports the complete opaque [`Key`] and does not interpret its
/// components.
///
/// [`Key`]: super::Key
pub trait Id:
    Clone + Debug + Display + Eq + Hash + Ord + Send + Sync + 'static
{
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

#[rustfmt::skip]
impl<T> Id for T
where
    T: Clone + Debug + Display + Eq + Hash + Ord + Send + Sync + 'static {}
