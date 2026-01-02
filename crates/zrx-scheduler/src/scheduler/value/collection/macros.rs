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

//! Macros for value collection creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Creates a value collection from the given expressions.
///
/// This macro conveniently creates [`Values`][] from the provided expressions,
/// each of which is expected to implement the [`Value`][] trait and which can
/// then be passed to actions. Note that this is a low-level API, and in most
/// cases, you will not need to use it directly.
///
/// [`Value`]: crate::scheduler::value::Value
/// [`Values`]: crate::scheduler::value::Values
///
/// # Examples
///
/// ```
/// use zrx_scheduler::values;
///
/// // Create value collection
/// let values = values!(&1, &2, &3);
/// ```
#[macro_export]
macro_rules! values {
    ($($arg:expr),* $(,)?) => {
        $crate::value::Values::from_iter([
            $(Some($arg as &dyn $crate::value::Value)),*
        ])
    };
}
