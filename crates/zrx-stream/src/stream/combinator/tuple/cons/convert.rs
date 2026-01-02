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

//! Stream tuple construction conversions.

use crate::stream::combinator::{IntoStreamTuple, StreamTuple};
use crate::stream::Stream;

use super::StreamTupleCons;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`StreamTupleCons`].
pub trait IntoStreamTupleCons<I, T> {
    /// Output type of conversion.
    type Output: StreamTuple<I>;

    /// Combines a stream with a tuple of stream references.
    ///
    /// While this method's signature looks like it should rather be inverted,
    /// this trait is solely intended as a helper trait to combine stream tuples
    /// with a stream. It combines [`IntoStreamTuple`] and [`StreamTupleCons`],
    /// allowing to keep the user-facing API ergonomic and convenient.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::tuple::cons::IntoStreamTupleCons;
    /// use zrx_stream::workspace::Workspace;
    ///
    /// // Create workspace and workflow
    /// let workspace = Workspace::<&str>::new();
    /// let workflow = workspace.add_workflow();
    ///
    /// // Create streams (heterogeneous)
    /// let a = workflow.add_source::<i32>();
    /// let b = workflow.add_source::<bool>();
    ///
    /// // Create stream tuple
    /// let tuple = b.into_stream_tuple_cons(a);
    /// ```
    fn into_stream_tuple_cons(self, stream: Stream<I, T>) -> Self::Output;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<I, T, S> IntoStreamTupleCons<I, T> for S
where
    S: IntoStreamTuple<I>,
    S::Output: StreamTupleCons<I, T>,
{
    type Output = <S::Output as StreamTupleCons<I, T>>::Output;

    #[inline]
    fn into_stream_tuple_cons(self, stream: Stream<I, T>) -> Self::Output {
        StreamTupleCons::cons(stream, self.into_stream_tuple())
    }
}
