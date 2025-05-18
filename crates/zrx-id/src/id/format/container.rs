// Copyright (c) 2024 Zensical <contributors@zensical.org>

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

//! Container.

use std::ops::{Deref, Range};
#[cfg(feature = "tinyvec")]
use tinyvec::{Array, TinyVec};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Container.
///
/// This trait defines the requirements for a container type that is used to
/// manage the storage of a UTF-8 string which is divided into a set of spans.
pub trait Container
where
    Self: for<'a> From<&'a [u8]>,
    Self: Deref<Target = [u8]>,
{
    /// Replace the specified range with the given value.
    ///
    /// Although implementations like [`Vec::splice`] return an iterator over
    /// the replaced section, we don't need it, so we can immediately drop it.
    fn splice<R, S>(&mut self, range: R, value: S)
    where
        R: Into<Range<usize>>,
        S: AsRef<[u8]>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Container for Vec<u8> {
    #[inline]
    fn splice<R, S>(&mut self, range: R, value: S)
    where
        R: Into<Range<usize>>,
        S: AsRef<[u8]>,
    {
        self.splice(range.into(), value.as_ref().iter().copied());
    }
}

#[cfg(feature = "tinyvec")]
impl<A: Array<Item = u8>> Container for TinyVec<A> {
    #[inline]
    fn splice<R, S>(&mut self, range: R, value: S)
    where
        R: Into<Range<usize>>,
        S: AsRef<[u8]>,
    {
        self.splice(range.into(), value.as_ref().iter().copied());
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Default container for formatted strings.
#[cfg(feature = "tinyvec")]
pub type Recommended = TinyVec<[u8; 64]>;

/// Default container for formatted strings.
#[cfg(not(feature = "tinyvec"))]
pub type Recommended = Vec<u8>;
