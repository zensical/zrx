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

//! Functions for use in operators.

use std::panic::{self, AssertUnwindSafe};

use zrx_scheduler::action::Error;

mod adapter;
mod argument;
mod traits;

pub use adapter::{with_id, with_splat};
pub use argument::Splat;
pub use traits::*;

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Catches panics and converts them to errors.
///
/// This function is useful for wrapping code that may panic, i.e., to shield
/// against panics in user-defined code or third-party libraries. It captures
/// the panic and returns it as an [`Error::Panic`], allowing the program to
/// continue running gracefully instead of terminating unexpectedly.
///
/// All of the function traits that we provide use this internally to ensure
/// that any panic is caught and converted to an error. Thus, it's absolutely
/// recommended to wrap any user-defined function in this function when
/// implementing a custom function trait.
///
/// # Errors
///
/// Returns [`Error::Panic`] if the provided function panics.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::action::Error;
/// use zrx_stream::function::catch;
///
/// // Define function that panics
/// let res = catch(|| {
///     panic!("don't panic!");
///     Ok(42) // Never returned
/// });
///
/// // Assert that panic was caught
/// assert!(matches!(res, Err(Error::Panic(_))));
/// ```
#[inline]
pub fn catch<F, T>(f: F) -> Result<T, Error>
where
    F: FnOnce() -> Result<T, Error>,
{
    panic::catch_unwind(AssertUnwindSafe(f))
        .map_err(Error::Panic)
        .and_then(|res| res)
}
