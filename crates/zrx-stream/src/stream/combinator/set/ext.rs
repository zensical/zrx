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

//! Stream set extensions.

use zrx_scheduler::{Id, Value};

use crate::stream::Stream;

use super::convert::IntoStreamSet;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Extension of [`StreamSet`][].
///
/// Although, conceptually, this extension trait does belong to [`StreamSet`][],
/// allowing to conveniently work with data types that can be converted into a
/// set of streams, it is deliberately implemented for anything that converts
/// via [`IntoStreamSet`], as this offers more flexibilty.
///
/// [`StreamSet`]: crate::stream::combinator::StreamSet
pub trait StreamSetExt<I, T>: IntoStreamSet<I, T> + Sized
where
    I: Id,
    T: Value + Clone + Eq,
{
    fn union(self) -> Option<Stream<I, T>> {
        self.into_stream_set() // fmt
            .into_union()
    }

    fn intersection(self) -> Option<Stream<I, T>> {
        self.into_stream_set() // fmt
            .into_intersection()
    }

    fn difference(self) -> Option<Stream<I, T>> {
        self.into_stream_set() // fmt
            .into_difference()
    }

    fn coalesce(self) -> Option<Stream<I, T>> {
        self.into_stream_set() // fmt
            .into_coalesce()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<'a, S, I, T> StreamSetExt<I, T> for S
where
    S: IntoIterator<Item = &'a Stream<I, T>>,
    I: Id,
    T: Value + Clone + Eq,
{
}
