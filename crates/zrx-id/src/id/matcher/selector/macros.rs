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

//! Macros for selector creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Creates an selector.
///
/// This macro creates a [`Selector`][] from the given key-value pairs, offering
/// a shorter syntax compared to using the [`Selector::builder`] directly. It
/// also allows to create a new selector based on an existing one by passing it
/// as the first argument.
///
/// [`Selector`]: crate::id::matcher::Selector
/// [`Selector::builder`]: crate::id::matcher::Selector::builder
///
/// # Examples
///
/// Create selector
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::selector;
///
/// // Create selector
/// let selector = selector!(location = "**/*.md")?;
/// assert_eq!(selector.as_str(), "zrs:::::**/*.md:");
/// # Ok(())
/// # }
/// ```
///
/// Create selector from existing selector
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::{selector, Selector};
///
/// // Create selector from string
/// let selector: Selector = "zrs:::::**/*.md:".parse()?;
///
/// // Create selector from existing selector
/// let selector = selector!(selector; provider = "file")?;
/// assert_eq!(selector.as_str(), "zrs:file::::**/*.md:");
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! selector {
    // Create selector from the given selector
    ($from:expr; $($key:ident = $value:expr),* $(,)?) => {{
        let mut builder = $from.to_builder();
        $(
            builder = selector!(@set builder, $key, $value);
        )*
        builder.build()
    }};

    // Create selector
    ($($key:ident = $value:expr),* $(,)?) => {{
        let mut builder = $crate::Selector::builder();
        $(
            builder = selector!(@set builder, $key, $value);
        )*
        builder.build()
    }};

    // Internal: match each key to the builder method
    (@set $builder:ident, provider, $value:expr) => {
        $builder.with_provider($value)
    };
    (@set $builder:ident, resource, $value:expr) => {
        $builder.with_resource($value)
    };
    (@set $builder:ident, variant, $value:expr) => {
        $builder.with_variant($value)
    };
    (@set $builder:ident, context, $value:expr) => {
        $builder.with_context($value)
    };
    (@set $builder:ident, location, $value:expr) => {
        $builder.with_location($value)
    };
    (@set $builder:ident, fragment, $value:expr) => {
        $builder.with_fragment($value)
    };
}
