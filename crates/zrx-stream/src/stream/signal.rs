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

//! Scalar stream shape.

use std::ops::Deref;

use zrx_scheduler::Value;
use zrx_scheduler::action::replication::IntoReplication;

use crate::stream::Id;

use super::Stream;
use super::function::{Arguments, MapFn};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Zero-or-one scalar value carried by the ordinary stream machinery.
///
/// A signal is absent or contains one value under the reserved empty key. It
/// dereferences to its underlying stream so relation combinators can consume
/// it without adding another scheduler representation.
pub struct Signal<I, T>
where
    I: Id,
{
    stream: Stream<I, T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Signal<I, T>
where
    I: Id,
{
    pub(in crate::stream) const fn new(stream: Stream<I, T>) -> Self {
        Self { stream }
    }

    /// Transforms a present scalar value while preserving absence.
    #[inline]
    #[must_use]
    pub fn map<F, A, U>(&self, function: F) -> Signal<I, U>
    where
        F: IntoReplication,
        F::Target: MapFn<A, I, T, U>,
        A: Arguments,
        T: Value,
        U: Value,
    {
        Signal::new(self.stream.map(function))
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Clone for Signal<I, T>
where
    I: Id,
{
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.stream.clone())
    }
}

impl<I, T> Deref for Signal<I, T>
where
    I: Id,
{
    type Target = Stream<I, T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.stream
    }
}
