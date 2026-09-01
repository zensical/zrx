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

//! Action error utilities.

use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;

use super::{Error, Kind, Result};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Catches panics and converts them to errors.
///
/// This function is useful for wrapping code that may panic, i.e., to shield
/// against panics in user-defined code or third-party libraries. It captures
/// the panic and returns it as an [`Error`], allowing the program to
/// continue running gracefully instead of terminating unexpectedly.
///
/// # Errors
///
/// Returns a panic-classified [`Error`] if the provided function panics.
///
/// The [`AssertUnwindSafe`] marker is used to wrap the provided closure, which
/// suppresses the compiler's unwind-safety checks. Catching a panic isolates
/// the caller's staged ZRX state; it does not roll back interior mutation or
/// external effects performed before the panic. Captured state may be retained
/// and reused by the caller, so callers must ensure that it remains valid for
/// later use. This function deliberately makes no `UnwindSafe` guarantee.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::action::error::catch;
///
/// // Define function that panics
/// let res = catch(|| {
///     panic!("don't panic!");
///     Ok(42) // Never returned
/// });
///
/// // Assert that panic was caught
/// assert_eq!(res.unwrap_err().to_string(), "caught panic: don't panic!");
/// ```
#[inline]
pub fn catch<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    panic::catch_unwind(AssertUnwindSafe(f))
        .map_err(|payload| {
            let payload = match payload.downcast::<String>() {
                Ok(message) => {
                    return Error(Arc::new(Kind::Panic(*message)));
                }
                Err(payload) => payload,
            };
            let message = payload.downcast::<&'static str>().map_or_else(
                |_| "non-string panic payload".to_owned(),
                |value| (*value).to_owned(),
            );
            Error(Arc::new(Kind::Panic(message)))
        })
        .flatten()
}
