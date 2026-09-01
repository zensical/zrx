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

//! Action error conversions.

use crate::scheduler::Value;

use super::Error;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Result`].
pub trait IntoResult<T = ()>: Sized {
    /// Converts into a function result.
    ///
    /// # Errors
    ///
    /// In case conversion fails, an error should be returned. Note that this
    /// trait is deliberately not named `TryIntoResult`, since it is itself
    /// infallible, and failures are always classified as ordinary causes. It
    /// merely provides a convenient way to return non-fallible results.
    fn into_result(self) -> Result<T, Error>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T, E> IntoResult<T> for Result<T, E>
where
    T: Value,
    E: Into<anyhow::Error>,
{
    /// Converts any result into an action result.
    #[inline]
    fn into_result(self) -> Result<T, Error> {
        self.map_err(|err| Error::from(err.into()))
    }
}

impl<T> IntoResult<T> for T
where
    T: Value,
{
    /// Converts any value into an action result.
    #[inline]
    fn into_result(self) -> Result<T, Error> {
        Ok(self)
    }
}
