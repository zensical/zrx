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

use std::error;

use super::Error;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Error`].
///
/// This trait is solely provided for [`IntoReport`][], which makes use of it
/// to conveniently convert any error type into [`Error`]. This is particularly
/// useful when it comes to user-provided functions that may return any kind of
/// error, as this trait ensures those errors are downcasted accordingly.
///
/// [`IntoReport`]: crate::scheduler::action::report::IntoReport
pub trait IntoError {
    /// Converts into an action error.
    ///
    /// This method is used to convert any error type into an [`Error`]. As of
    /// now, the only errors that receive special treatment are errors of type
    /// [`io::Error`][], as they're common and we can handle them more easily.
    /// All other errors are converted into the [`Error::Other`] variant.
    ///
    /// Note that [`Error::Value`] is deliberately not handled, so it would be
    /// converted to [`Error::Other`] as well, since we use this error variant
    /// for fine-grained control of the scheduler. In fact, [`Error::Value`]
    /// is only intended for internal use, but still an action error.
    ///
    /// [`io::Error`]: std::io::Error
    fn into_error(self) -> Error;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<E> IntoError for E
where
    E: error::Error + Send + 'static,
{
    /// Creates an action error from any other error.
    #[inline]
    fn into_error(self) -> Error {
        let temp: Box<dyn error::Error + Send> = Box::new(self);
        temp.downcast()
            .map_or_else(Error::Other, |err| Error::Io(*err))
    }
}
