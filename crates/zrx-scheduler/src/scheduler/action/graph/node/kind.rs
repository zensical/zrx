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

//! Node kind.

use super::handler::{Handler, Iter};

mod source;
mod worker;

pub use source::Source;
pub use worker::Worker;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Node kind.
#[derive(Debug)]
pub enum Kind<I> {
    /// Source.
    Source(Source<I>),
    /// Worker.
    Worker(Worker<I>),
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> From<Source<I>> for Kind<I> {
    /// Creates a node kind from a source.
    #[inline]
    fn from(source: Source<I>) -> Self {
        Self::Source(source)
    }
}

impl<I> From<Worker<I>> for Kind<I> {
    /// Creates a node kind from a worker.
    #[inline]
    fn from(intermediate: Worker<I>) -> Self {
        Self::Worker(intermediate)
    }
}
