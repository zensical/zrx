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

//! URI representation.

use std::borrow::Cow;
use std::fmt;
use std::path::Path;

use zrx_path::PathExt;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// URI representation of an identifier.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Uri<'a> {
    /// Inner string.
    inner: Cow<'a, str>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Uri<'_> {
    /// Creates a relative URI from the given base URI.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::uri::Uri;
    ///
    /// // Create relative URI from base
    /// let uri = Uri::from("a.md").relative_to("a/b.md");
    /// assert_eq!(uri.as_str(), "../a.md");
    /// ```
    #[inline]
    #[must_use]
    pub fn relative_to<S>(&self, base: S) -> Self
    where
        S: AsRef<str>,
    {
        let path = Path::new(self.inner.as_ref())
            .relative_to(base.as_ref())
            .to_string_lossy()
            .replace('\\', "/");

        // Create relative URI
        Self { inner: Cow::Owned(path) }
    }

    /// Returns the string representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::uri::Uri;
    ///
    /// // Create URI from string
    /// let uri = Uri::from("index.md");
    ///
    /// // Obtain string representation
    /// assert_eq!(uri.as_str(), "index.md");
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.inner.as_ref()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl AsRef<str> for Uri<'_> {
    /// Returns the string representation.
    #[inline]
    fn as_ref(&self) -> &str {
        self.inner.as_ref()
    }
}

// ----------------------------------------------------------------------------

impl<'a, T> From<T> for Uri<'a>
where
    T: Into<Cow<'a, str>>,
{
    /// Creates a URI from a string.
    #[inline]
    fn from(value: T) -> Self {
        Self { inner: value.into() }
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for Uri<'_> {
    /// Formats the URI for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}
