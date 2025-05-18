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

//! Path utilities.

use std::path::{Component, PathBuf};

use super::Id;

mod error;

pub use error::{Error, Result};

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl TryFrom<&Id> for PathBuf {
    type Error = Error;

    /// Attempts to create a relative path from an identifier.
    ///
    /// Note that [`PathBuf`] is a platform-specific path type, which will also
    /// make sure that the path is correctly formatted for the current platform,
    /// i.e., it will use backslashes on Windows and forward slashes on Unix.
    /// Thus, paths can't be directly used as URLs.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::path::PathBuf;
    /// use zrx_id::Id;
    ///
    /// // Create formatted string from string
    /// let id: Id = "zri:file::docs:index.md:".parse()?;
    /// let path = PathBuf::try_from(&id)?;
    /// assert_eq!(path, PathBuf::from("docs").join("index.md"));
    /// # Ok(())
    /// # }
    /// ```
    fn try_from(id: &Id) -> Result<Self> {
        let mut stack = Vec::new();

        // Normalize path and analyze its components - since Windows supports
        // forward slashes and backslashes, we do not need to normalize it
        let path =
            PathBuf::from(id.context().as_ref()).join(id.path().as_ref());
        for component in path.components() {
            match component {
                Component::Normal(part) => stack.push(part),
                Component::CurDir => continue,

                // Disallow path traversal for security reasons, which means
                // `..` is not supported in paths, as it would allow to break
                // out of the context. If we find a use case where we need `..`,
                // we can lift this requirement in the future.
                Component::ParentDir => {
                    return Err(Error::ParentDir);
                }

                // Disallow absolute paths, as we need to ensure that paths are
                // always portable. Note that providers can use the binding to
                // resolve paths relative to different mount points, e.g., to
                // allow for plugins to ship with their own artifacts.
                Component::RootDir | Component::Prefix(_) => {
                    return Err(Error::RootDir);
                }
            }
        }

        // Collect path components into a path
        Ok(stack.iter().collect())
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Ensure that the given value is a valid path.
///
/// We normalize paths to use forward slashes, which is the default on Unix
/// systems and also supported on Windows. This way, we can ensure that paths
/// are portable and can be used as URLs. Rust's file system API ensures that
/// paths are correctly resolved for the current platform.
///
/// # Errors
///
/// If a backslash is found, [`Error::Backslash`] is returned.
#[inline]
pub fn validate<S>(value: S) -> Result<S>
where
    S: AsRef<[u8]>,
{
    if value.as_ref().contains(&b'\\') {
        Err(Error::Backslash)
    } else {
        Ok(value)
    }
}
