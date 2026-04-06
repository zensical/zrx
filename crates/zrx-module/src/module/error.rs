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

//! Module error.

use std::{error, result};
use thiserror::Error;

use zrx_id as id;
use zrx_id::expression;

mod convert;

pub use convert::IntoError;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Module error.
///
/// Modules are the fundamental building blocks of the system, as they allow to
/// register functionality with the system as part of the implementation of the
/// [`Module`][] trait. As such, [`Module::setup`][] must be allowed to return
/// any kind of error, which is then converted into a module error.
///
/// The enum is deliberately not marked as non-exhaustive, because this type is
/// intended to be the sink for all errors that typically occur in modules, and
/// should never be used in external code. As such, we're allowed to freely add
/// new variants, moving errors out of [`Error::Other`] when necessary.
///
/// In order to integrate with error types that are not already covered by the
/// existing variants, the [`IntoError`] conversion trait is provided. Please
/// note that this trait should only be used sparingly, and could denote a code
/// smell, since [`Module::setup`][] implementations should rarely be fallible
/// besides identifier and expression parsing errors, especially not involving
/// side effects like file system or network access. Implementations should only
/// fail due to programmer or configuration errors, which should be covered by
/// the existing variants. In case you think an essential variant is missing,
/// please report it on our issue tracker.
///
/// [`Module`]: crate::Module
/// [`Module::setup`]: crate::Module::setup
#[derive(Debug, Error)]
pub enum Error {
    /// Identifier error.
    #[error(transparent)]
    Id(#[from] id::Error),
    /// Expression error.
    #[error(transparent)]
    Expression(#[from] expression::Error),
    /// Catch-all errors.
    #[error(transparent)]
    Other(#[from] Box<dyn error::Error + Send + 'static>),
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Module result.
pub type Result<T = ()> = result::Result<T, Error>;
