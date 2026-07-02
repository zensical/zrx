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

//! Filter error.

use std::{num, result};
use thiserror::Error;

use crate::id;
use crate::id::expression::filter;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Filter error.
#[derive(Debug, Error)]
pub enum Error {
    /// Numeric conversion error.
    #[error(transparent)]
    Numeric(#[from] num::TryFromIntError),
    /// Identifier error.
    #[error(transparent)]
    Id(#[from] id::Error),
    /// Expression filter error.
    #[error(transparent)]
    Filter(#[from] filter::Error),
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Filter result.
pub type Result<T = ()> = result::Result<T, Error>;
