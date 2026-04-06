// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// Third-party contributions licensed under DCO

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

//! Stream.

use std::marker::PhantomData;

pub mod combinator;
pub mod function;
pub mod operator;
pub mod workflow;

use workflow::Builder;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stream.
#[derive(Debug)]
pub struct Stream<I, T> {
    /// Identifier.
    id: usize,
    /// Workflow builder.
    workflow: Builder<I>,
    /// Capture types.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Clone for Stream<I, T> {
    /// Clones the stream.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            workflow: self.workflow.clone(),
            marker: self.marker,
        }
    }
}
