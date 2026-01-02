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

//! Stream set.

use std::vec::IntoIter;

use crate::stream::Stream;

mod convert;
mod ext;

pub use convert::IntoStreamSet;
pub use ext::StreamSetExt;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Stream set.
///
/// Stream sets are homogeneous collections of streams, implementing common set
/// operations like union, intersection, and difference. They are represented
/// as a vector of streams, in order to preserve the ordering in which they
/// were added, which is crucial for operators like [`Stream::coalesce`].
///
/// Operators implemented with stream sets include:
///
/// - [`Stream::union`]
/// - [`Stream::intersection`]
/// - [`Stream::difference`]
/// - [`Stream::coalesce`]
///
/// Note that stream sets implement set operations themselves, including union,
/// intersection, and difference, which means they can be combined with other
/// stream sets to implement combinatorical structural operators. However, the
/// union set operation is likely the most common use case.
#[derive(Clone, Debug)]
pub struct StreamSet<I, T> {
    /// Vector of streams.
    inner: Vec<Stream<I, T>>,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

impl<I, T> StreamSet<I, T> {
    /// Creates the union of two stream sets.
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
    /// let c = workflow.add_source::<i32>();
    ///
    /// // Create union of stream sets
    /// let set = [&a, &b].into_stream_set().union([&b, &c]);
    /// assert_eq!(set.len(), 3);
    /// ```
    #[must_use]
    pub fn union<S>(mut self, streams: S) -> Self
    where
        S: IntoStreamSet<I, T>,
    {
        let streams = streams.into_stream_set();
        for stream in streams {
            if !self.inner.contains(&stream) {
                self.inner.push(stream);
            }
        }
        self
    }

    /// Creates the intersection of two stream sets.
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
    /// let c = workflow.add_source::<i32>();
    ///
    /// // Create intersection of stream sets
    /// let set = [&a, &b].into_stream_set().intersection([&b, &c]);
    /// assert_eq!(set.len(), 1);
    /// ```
    #[must_use]
    pub fn intersection<S>(self, streams: S) -> Self
    where
        S: IntoStreamSet<I, T>,
    {
        let streams = streams.into_stream_set();
        self.into_iter()
            .filter(|stream| streams.contains(stream))
            .collect()
    }

    /// Creates the difference of two stream sets.
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
    /// let c = workflow.add_source::<i32>();
    ///
    /// // Create difference of stream sets
    /// let set = [&a, &b].into_stream_set().difference([&b, &c]);
    /// assert_eq!(set.len(), 1);
    /// ```
    #[must_use]
    pub fn difference<S>(self, streams: S) -> Self
    where
        S: IntoStreamSet<I, T>,
    {
        let streams = streams.into_stream_set();
        self.into_iter()
            .filter(|stream| !streams.contains(stream))
            .collect()
    }

    /// Returns whether the stream set is a subset.
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
    /// // Create stream set and check for subset
    /// let set = a.into_stream_set();
    /// assert!(set.is_subset([&a, &b]));
    /// ```
    pub fn is_subset<S>(&self, streams: S) -> bool
    where
        S: IntoStreamSet<I, T>,
    {
        let streams = streams.into_stream_set();
        let mut iter = self.inner.iter();
        iter.all(|stream| streams.contains(stream))
    }

    /// Returns whether the stream set is a superset.
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
    /// // Create stream set and check for superset
    /// let set = [&a, &b].into_stream_set();
    /// assert!(set.is_superset(&a));
    /// ```
    pub fn is_superset<S>(&self, streams: S) -> bool
    where
        S: IntoStreamSet<I, T>,
    {
        let streams = streams.into_stream_set();
        let mut iter = streams.into_iter();
        iter.all(|stream| self.contains(&stream))
    }

    /// Returns a reference to the stream at the given index.
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
    /// // Create stream set and obtain stream reference
    /// let set = [&a, &b].into_stream_set();
    /// assert_eq!(set.get(0), Some(&a));
    /// ```
    #[inline]
    #[must_use]
    pub fn get(&self, index: usize) -> Option<&Stream<I, T>> {
        self.inner.get(index)
    }

    /// Returns whether the stream set contains the given stream.
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
    /// // Create stream set and ensure presence of stream
    /// let set = [&a, &b].into_stream_set();
    /// assert!(set.contains(&a));
    /// ```
    #[inline]
    #[must_use]
    pub fn contains(&self, stream: &Stream<I, T>) -> bool {
        self.inner.contains(stream)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, T> StreamSet<I, T> {
    /// Returns the number of streams.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any streams.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> FromIterator<Stream<I, T>> for StreamSet<I, T> {
    /// Creates a stream set from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::{IntoStreamSet, StreamSet};
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
    /// // Create stream set from iterator
    /// let set = StreamSet::from_iter([a, b]);
    /// ```
    #[inline]
    fn from_iter<S>(iter: S) -> Self
    where
        S: IntoIterator<Item = Stream<I, T>>,
    {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<I, T> IntoIterator for StreamSet<I, T> {
    type Item = Stream<I, T>;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates an iterator over the stream set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::combinator::{IntoStreamSet, StreamSet};
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
    /// // Create and iterate over stream set
    /// let set = StreamSet::from_iter([a, b]);
    /// for stream in set {
    ///     println!("{stream:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
