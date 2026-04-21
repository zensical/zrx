// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// Third-party contributions licensed under DCO

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

//! Step error conversions.

use std::error;

use crate::scheduler::signal::Value;

use super::Error;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Error`].
pub trait IntoError {
    /// Converts into a step error.
    ///
    /// This method is intended to be used within [`Action::execute`][], to make
    /// it convenient to convert an arbitrary error into a step error.
    ///
    /// [`Action::execute`]: crate::scheduler::action::Action::execute
    fn into_error(self) -> Error;
}

// ----------------------------------------------------------------------------

/// Conversion into [`Result`].
pub trait IntoResult<T>: Sized {
    /// Converts into a step result.
    fn into_result(self) -> Result<T, Error>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<E> IntoError for E
where
    E: error::Error + Send + 'static,
{
    /// Converts any error into a step error.
    ///
    /// This implementation attempts to downcast the error into one of the known
    /// variants, and falls back to the [`Error::Other`] variant in other cases.
    /// It also ensures that the error is not double-boxed, returning unknown
    /// errors captured by the [`Error::Other`] variant unchanged.
    #[inline]
    fn into_error(self) -> Error {
        let mut err: Box<dyn error::Error + Send> = Box::new(self);
        err = match err.downcast() {
            Ok(err) => return *err,
            Err(err) => err,
        };

        // Implements downcast attempt of error
        macro_rules! downcast {
            ($variant:path) => {
                err = match err.downcast() {
                    Ok(err) => return $variant(*err),
                    Err(err) => err,
                }
            };
        }

        // Downcast known error variants
        downcast!(Error::Session);
        downcast!(Error::Key);

        // Handle unknown error variants
        Error::Other(err)
    }
}

// ----------------------------------------------------------------------------

impl<T, E> IntoResult<T> for Result<T, E>
where
    T: Value,
    E: IntoError,
{
    /// Converts any result into a step result.
    #[inline]
    fn into_result(self) -> Result<T, Error> {
        self.map_err(IntoError::into_error)
    }
}

impl<T> IntoResult<T> for T
where
    T: Value,
{
    /// Converts any value into a step result.
    #[inline]
    fn into_result(self) -> Result<T, Error> {
        Ok(self)
    }
}
