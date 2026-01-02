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

//! Stream.

use std::marker::PhantomData;

use zrx_scheduler::{Id, Value};

pub mod barrier;
pub mod combinator;
pub mod function;
pub mod operator;
pub mod value;
pub mod workspace;

use function::InspectFn as ForEachFn;
use workspace::Workflow;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stream.
#[derive(Debug)]
pub struct Stream<I, T> {
    /// Identifier.
    id: usize,
    /// Associated workflow.
    workflow: Workflow<I>,
    /// Type marker.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value + Clone,
{
    pub fn for_each<F>(&self, f: F)
    where
        F: ForEachFn<I, T> + Clone,
    {
        self.inspect(f);
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Clone for Stream<I, T> {
    /// Clones the stream.
    #[inline]
    fn clone(&self) -> Self {
        let workflow = self.workflow.clone();
        Self { workflow, ..*self }
    }
}

// ----------------------------------------------------------------------------

impl<I, T> PartialEq for Stream<I, T> {
    /// Compares two streams for equality.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<I, T> Eq for Stream<I, T> {}
