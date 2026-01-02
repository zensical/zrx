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

//! Stream tuple conversions.

use crate::stream::Stream;

use super::StreamTuple;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`StreamTuple`].
pub trait IntoStreamTuple<I> {
    /// Output type of conversion.
    type Output: StreamTuple<I>;

    /// Converts a tuple of stream references into a stream tuple.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::IntoStreamTuple;
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
    /// let tuple = (&a, &b).into_stream_tuple();
    /// ```
    fn into_stream_tuple(self) -> Self::Output;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> IntoStreamTuple<I> for &Stream<I, T> {
    type Output = (Stream<I, T>,);

    /// Converts a stream reference into a stream tuple.
    ///
    /// Albeit this conversion is trivial, it allows to pass stream references
    /// to functions that expect tuples, which can be quite convenient.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::IntoStreamTuple;
    /// use zrx_stream::workspace::Workspace;
    ///
    /// // Create workspace and workflow
    /// let workspace = Workspace::<&str>::new();
    /// let workflow = workspace.add_workflow();
    ///
    /// // Create stream
    /// let stream = workflow.add_source::<i32>();
    ///
    /// // Create stream tuple
    /// let tuple = stream.into_stream_tuple();
    /// ```
    #[inline]
    fn into_stream_tuple(self) -> Self::Output {
        (self.clone(),)
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements stream tuple conversion trait.
macro_rules! impl_into_stream_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> IntoStreamTuple<I> for ($(&Stream<I, $T>,)+) {
            type Output = ($(Stream<I, $T>,)+);

            #[inline]
            fn into_stream_tuple(self) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($T,)+) = self;
                ($($T.clone(),)+)
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_into_stream_tuple!(T1);
impl_into_stream_tuple!(T1, T2);
impl_into_stream_tuple!(T1, T2, T3);
impl_into_stream_tuple!(T1, T2, T3, T4);
impl_into_stream_tuple!(T1, T2, T3, T4, T5);
impl_into_stream_tuple!(T1, T2, T3, T4, T5, T6);
impl_into_stream_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_into_stream_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
