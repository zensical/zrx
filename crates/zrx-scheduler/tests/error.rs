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

//! Action error tests.

use std::io;

use zrx_scheduler::action::error::catch;
use zrx_scheduler::action::{Error, Result};

#[test]
fn cause_preserves_source() {
    let source = io::Error::other("source");
    let error = Error::from(anyhow::Error::new(source));

    assert_eq!(error.to_string(), "source");
    assert_eq!(
        std::error::Error::source(&error).map(ToString::to_string),
        Some("source".to_owned())
    );
}

#[test]
fn panic_preserves_normalized_message() {
    let error = catch(|| -> Result { panic!("payload") }).unwrap_err();

    assert_eq!(error.to_string(), "caught panic: payload");
    assert!(std::error::Error::source(&error).is_none());
}
