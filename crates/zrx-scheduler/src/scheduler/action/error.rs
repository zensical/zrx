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

//! Action error.

use std::fmt::{self, Display};
use std::result;
use std::sync::Arc;

mod catch;
mod convert;

pub use catch::catch;
pub use convert::IntoResult;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Debug)]
enum Kind {
    /// Error in user-provided function.
    Cause(anyhow::Error),
    /// Panic in user-provided function.
    Panic(String),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Action error.
///
/// In order to integrate with user-provided error types that are not covered
/// by the existing variants, the [`anyhow`] crate is used as a catch-all. Any
/// error or panic that occurs in user-provided functions is wrapped in this
/// type, so we can handle it gracefully and provide useful error messages.
#[derive(Clone, Debug)]
pub struct Error(Arc<Kind>);

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self(Arc::new(Kind::Cause(error)))
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &*self.0 {
            Kind::Cause(error) => Display::fmt(error, formatter),
            Kind::Panic(message) => {
                write!(formatter, "caught panic: {message}")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &*self.0 {
            Kind::Cause(error) => Some(error.as_ref()),
            Kind::Panic(_) => None,
        }
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Action result.
pub type Result<T = ()> = result::Result<T, Error>;
