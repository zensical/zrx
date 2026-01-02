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

//! Path extensions.

use std::path::{Path, PathBuf};

use super::transform;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Extension of [`Path`] and [`PathBuf`].
pub trait PathExt {
    /// Normalizes the given absolute or relative path.
    fn normalize(&self) -> PathBuf;

    /// Creates a relative path from the given base path.
    fn relative_to<P>(&self, base: P) -> PathBuf
    where
        P: AsRef<Path>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl PathExt for Path {
    /// Normalizes the given absolute or relative path.
    ///
    /// For more information, see [`transform::normalize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use zrx_path::PathExt;
    ///
    /// // Normalize path with `..` components
    /// let path = Path::new("a/../b").normalize();
    /// assert_eq!(path, Path::new("b"));
    /// ```
    #[inline]
    fn normalize(&self) -> PathBuf {
        transform::normalize(self)
    }

    /// Creates a relative path from the given base path.
    ///
    /// For more information, see [`transform::relative_to`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    /// use zrx_path::PathExt;
    ///
    /// // Create relative path from base
    /// let path = Path::new("a/b/c").relative_to("a/d/e");
    /// assert_eq!(path, Path::new("../b/c"));
    /// ```
    #[inline]
    fn relative_to<P>(&self, base: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        transform::relative_to(self, base)
    }
}

impl PathExt for PathBuf {
    /// Normalizes the given absolute or relative path.
    ///
    /// For more information, see [`transform::normalize`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use zrx_path::PathExt;
    ///
    /// // Normalize path with `..` components
    /// let path = PathBuf::from("a/../b").normalize();
    /// assert_eq!(path, PathBuf::from("b"));
    /// ```
    #[inline]
    fn normalize(&self) -> PathBuf {
        transform::normalize(self)
    }

    /// Creates a relative path from the given base path.
    ///
    /// For more information, see [`transform::relative_to`].
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use zrx_path::PathExt;
    ///
    /// // Create relative path from base
    /// let path = PathBuf::from("a/b/c").relative_to("a/d/e");
    /// assert_eq!(path, PathBuf::from("../b/c"));
    /// ```
    #[inline]
    fn relative_to<P>(&self, base: P) -> PathBuf
    where
        P: AsRef<Path>,
    {
        transform::relative_to(self, base)
    }
}
