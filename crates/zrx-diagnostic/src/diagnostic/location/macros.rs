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

//! Macros for location creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Creates a location at the invocation site.
///
/// This macro conveniently creates a [`Location`][], including file, line and
/// column information exactly where it was invoked. It's usually not intended
/// to be used on its own, as it's invoked by the diagnostic macros, including
/// [`error!`][], [`warning!`][] and friends to capture location information.
///
/// [`error!`]: crate::error!
/// [`warning!`]: crate::warning!
/// [`Location`]: crate::diagnostic::location::Location
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::location;
///
/// // Create location
/// let location = location!();
/// ```
#[macro_export]
macro_rules! location {
    () => {
        $crate::location::Location::new(
            file!(),
            $crate::location::Position::new(
                line!().saturating_sub(1),
                column!().saturating_sub(1),
            ),
        )
    };
}

/// Creates a location at the invocation site of a function.
///
/// This macro requires the function in which it is called to be annotated with
/// the `#[track_caller]` attribute, so it can reliably determine the location
/// at the invocation site. This is particularly helpful for functions taking
/// closures from arbitrary locations.
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::{caller, Location};
///
/// // Define function printing location
/// #[track_caller]
/// fn f() {
///     println!("{:?}", caller!())
/// }
///
/// // Invoke function printing location
/// let location = f();
/// ```
#[macro_export]
macro_rules! caller {
    () => {
        $crate::location::Location::from(std::panic::Location::caller())
    };
}
