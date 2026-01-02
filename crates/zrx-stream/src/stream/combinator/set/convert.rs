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

//! Stream set conversions.

use super::{Stream, StreamSet};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`StreamSet`].
pub trait IntoStreamSet<I, T> {
    /// Converts an iterator of stream references into a stream set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::IntoStreamSet;
    /// use zrx_stream::workspace::Workspace;
    ///
    /// // Create workspace and workflow
    /// let workspace = Workspace::<&str>::new();
    /// let workflow = workspace.add_workflow();
    ///
    /// // Create streams (homogeneous)
    /// let a = workflow.add_source::<i32>();
    /// let b = workflow.add_source::<i32>();
    ///
    /// // Create stream set
    /// let set = [&a, &b].into_stream_set();
    /// ```
    fn into_stream_set(self) -> StreamSet<I, T>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> IntoStreamSet<I, T> for &Stream<I, T> {
    /// Converts a stream reference into a stream set.
    ///
    /// Albeit this conversion is trivial, it allows to pass stream references
    /// to functions that expect sets, which can be quite convenient.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::IntoStreamSet;
    /// use zrx_stream::workspace::Workspace;
    ///
    /// // Create workspace and workflow
    /// let workspace = Workspace::<&str>::new();
    /// let workflow = workspace.add_workflow();
    ///
    /// // Create stream
    /// let stream = workflow.add_source::<i32>();
    ///
    /// // Create stream set
    /// let set = stream.into_stream_set();
    /// ```
    #[inline]
    fn into_stream_set(self) -> StreamSet<I, T> {
        StreamSet::from_iter([self.clone()])
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<'a, I, T, S> IntoStreamSet<I, T> for S
where
    S: IntoIterator<Item = &'a Stream<I, T>>,
    Stream<I, T>: 'a,
{
    #[inline]
    fn into_stream_set(self) -> StreamSet<I, T> {
        self.into_iter().cloned().collect()
    }
}
