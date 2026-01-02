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

//! Path utilities.

use std::path::{Component, Path};

mod error;

pub use error::{Error, Result};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Ensure that the given formatted string is a valid path.
#[inline]
pub fn validate<S>(value: S) -> Result
where
    S: AsRef<str>,
{
    // Ensure that the value does not contain backslashes, as paths which are
    // used in identifiers and selectors must always use forward slashes. This
    // is important to ensure that paths are always portable across platforms,
    // and that URLs can be easily derived from them.
    if value.as_ref().contains('\\') {
        return Err(Error::Backslash);
    }

    // Create a path from the value and inspect its components, so we can
    // ensure that it is relative, and is not a path traversal
    let path = Path::new(value.as_ref());
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}

            // Disallow path traversal for security reasons, which means `..`
            // is not supported in paths, as it would allow to break out of the
            // context. This means that paths must be normalized before being
            // passed to the builder.
            Component::ParentDir => {
                return Err(Error::ParentDir);
            }

            // Disallow absolute paths, as we need to ensure paths are always
            // portable. Note that providers can use the resource component to
            // resolve paths relative to different mount points, e.g., to allow
            // for modules to ship with their own artifacts.
            Component::RootDir | Component::Prefix(_) => {
                return Err(Error::RootDir);
            }
        }
    }

    // No errors occurred
    Ok(())
}
