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

//! Macros for diagnostic creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Creates a diagnostic with error severity.
///
/// This macro creates a diagnostic message with [`Severity::Error`][], using
/// either a static string or a format string as an argument. Diagnostics are
/// not globally registered but always returned, so the caller needs to make
/// sure to forward them by including them into a [`Report`][].
///
/// The diagnostic will include file, line and column information.
///
/// [`Report`]: crate::diagnostic::report::Report
/// [`Severity::Error`]: crate::diagnostic::Severity::Error
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::error;
///
/// // Create diagnostic with static string
/// let diagnostic = error!("Static");
///
/// // Create diagnostic with format string
/// let diagnostic = error!("Format: {}", true);
/// ```
#[macro_export]
macro_rules! error {
    // Create and append diagnostic
    (&mut $sink:expr, $($arg:tt)+) => {
        $sink.push($crate::error!($($arg)+))
    };
    // Create and return diagnostic
    ($($arg:tt)*) => {
        $crate::__diagnostic!(
            $crate::Severity::Error,
            $($arg)*
        )
    };
}

/// Creates a diagnostic with warning severity.
///
/// This macro creates a diagnostic message with [`Severity::Warning`][], using
/// either a static string or a format string as an argument. Diagnostics are
/// not globally registered but always returned, so the caller needs to make
/// sure to forward them by including them into a [`Report`][].
///
/// The diagnostic will include file, line and column information.
///
/// [`Report`]: crate::diagnostic::report::Report
/// [`Severity::Warning`]: crate::diagnostic::Severity::Warning
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::warning;
///
/// // Create diagnostic with static string
/// let diagnostic = warning!("Static");
///
/// // Create diagnostic with format string
/// let diagnostic = warning!("Format: {}", true);
/// ```
#[macro_export]
macro_rules! warning {
    // Create and append diagnostic
    (&mut $sink:expr, $($arg:tt)+) => {
        $sink.push($crate::warning!($($arg)+))
    };
    // Create and return diagnostic
    ($($arg:tt)*) => {
        $crate::__diagnostic!(
            $crate::Severity::Warning,
            $($arg)*
        )
    };
}

/// Creates a diagnostic with info severity.
///
/// This macro creates a diagnostic message with [`Severity::Info`][], using
/// either a static string or a format string as an argument. Diagnostics are
/// not globally registered but always returned, so the caller needs to make
/// sure to forward them by including them into a [`Report`][].
///
/// The diagnostic will include file, line and column information.
///
/// [`Report`]: crate::diagnostic::report::Report
/// [`Severity::Info`]: crate::diagnostic::Severity::Info
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::info;
///
/// // Create diagnostic with static string
/// let diagnostic = info!("Static");
///
/// // Create diagnostic with format string
/// let diagnostic = info!("Format: {}", true);
/// ```
#[macro_export]
macro_rules! info {
    // Create and append diagnostic
    (&mut $sink:expr, $($arg:tt)+) => {
        $sink.push($crate::info!($($arg)+))
    };
    // Create and return diagnostic
    ($($arg:tt)*) => {
        $crate::__diagnostic!(
            $crate::Severity::Info,
            $($arg)*
        )
    };
}

/// Creates a diagnostic with hint severity.
///
/// This macro creates a diagnostic message with [`Severity::Hint`][], using
/// either a static string or a format string as an argument. Diagnostics are
/// not globally registered but always returned, so the caller needs to make
/// sure to forward them by including them into a [`Report`][].
///
/// The diagnostic will include file, line and column information.
///
/// [`Report`]: crate::diagnostic::report::Report
/// [`Severity::Hint`]: crate::diagnostic::Severity::Hint
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::hint;
///
/// // Create diagnostic with static string
/// let diagnostic = hint!("Static");
///
/// // Create diagnostic with format string
/// let diagnostic = hint!("Format: {}", true);
/// ```
#[macro_export]
macro_rules! hint {
    // Create and append diagnostic
    (&mut $sink:expr, $($arg:tt)+) => {
        $sink.push($crate::hint!($($arg)+))
    };
    // Create and return diagnostic
    ($($arg:tt)*) => {
        $crate::__diagnostic!(
            $crate::Severity::Hint,
            $($arg)*
        )
    };
}

/// Creates a diagnostic with debug severity.
///
/// This macro creates a diagnostic message with [`Severity::Debug`][], using
/// either a static string or a format string as an argument. Diagnostics are
/// not globally registered but always returned, so the caller needs to make
/// sure to forward them by including them into a [`Report`][].
///
/// The diagnostic will include file, line and column information.
///
/// [`Report`]: crate::diagnostic::report::Report
/// [`Severity::Debug`]: crate::diagnostic::Severity::Debug
///
/// # Examples
///
/// ```
/// use zrx_diagnostic::debug;
///
/// // Create diagnostic with static string
/// let diagnostic = debug!("Static");
///
/// // Create diagnostic with format string
/// let diagnostic = debug!("Format: {}", true);
/// ```
#[macro_export]
macro_rules! debug {
    // Create and append diagnostic
    (&mut $sink:expr, $($arg:tt)+) => {
        $sink.push($crate::debug!($($arg)+))
    };
    // Create and return diagnostic
    ($($arg:tt)*) => {
        $crate::__diagnostic!(
            $crate::Severity::Debug,
            $($arg)*
        )
    };
}

// ----------------------------------------------------------------------------

/// Creates a diagnostic with the given severity.
///
/// Note that this is an internal helper macro to create a diagnostics either
/// from a static string or format string, as well as the given severity. It's
/// not intended to be used directly. Use the severity-specific implementations
/// like [`error!`], [`warning!`] and friends instead.
#[doc(hidden)]
#[macro_export]
macro_rules! __diagnostic {
    // Create diagnostic with severity and static string
    ($severity:expr, $message:expr) => {
        $crate::Diagnostic::new($severity, $message)
            .location($crate::location!())
    };
    // Create diagnostic with severity and format string
    ($severity:expr, $fmt:expr, $($arg:tt)*) => {
        $crate::Diagnostic::new($severity, format!($fmt, $($arg)*))
            .location($crate::location!())
    };
}
