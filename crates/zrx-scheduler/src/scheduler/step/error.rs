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

//! Step error.

use std::any::Any;
use std::panic::{self, AssertUnwindSafe};
use std::result;
use thiserror::Error;

use crate::scheduler::session;
use crate::scheduler::signal::key;

mod convert;

pub use convert::IntoResult;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Step error.
///
/// An [`Action`][] is composed of one or more [`Steps`][], which are returned
/// and executed sequentially as part of the action's definition. If an action
/// accepts any user-defined code, e.g. in the form of a function, it needs to
/// account for all errors occurring during the execution of that function.
///
/// The enum is deliberately not marked as non-exhaustive, because this type is
/// intended to be the sink for all errors that typically occur in actions, and
/// should never be used in external code. As such, we're allowed to freely add
/// new variants, moving errors out of [`Error::Other`] when necessary.
///
/// In order to integrate with user-provided error types that are not covered
/// by the existing variants, the [`anyhow`] crate is used as a catch-all.
///
/// [`Action`]: crate::scheduler::action::Action
/// [`Steps`]: crate::scheduler::step::Steps
#[derive(Debug, Error)]
pub enum Error {
    /// Session error.
    #[error(transparent)]
    Session(#[from] session::Error),
    /// Key error.
    #[error(transparent)]
    Key(#[from] key::Error),
    /// Other error.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    /// Caught panic.
    #[error("caught panic")]
    Panic(Box<dyn Any + Send>),
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Step result.
pub type Result<T = ()> = result::Result<T, Error>;

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
/// # Errors
///
/// Returns [`Error::Panic`] if the provided function panics.
///
/// # Safety
///
/// This function uses [`AssertUnwindSafe`] to wrap the provided closure, which
/// suppresses the compiler's unwind-safety checks. This is sound here because
/// the closure and all state it captures are discarded immediately after the
/// panic is caught – the [`Error::Panic`] variant owns the panic payload, but
/// nothing from the closure's captured environment is accessed or reused
/// afterwards. Callers must ensure that any side effects performed before the
/// panic (e.g. writes to shared state outside the closure) are accounted for,
/// as those cannot be unwound or recovered by this function.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::step::Error;
/// use zrx_scheduler::step::error::catch;
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
pub fn catch<F, T>(f: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    panic::catch_unwind(AssertUnwindSafe(f))
        .map_err(Error::Panic)
        .and_then(|res| res)
}
