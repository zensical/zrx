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

//! Macros for identifier creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Creates an identifier.
///
/// This macro creates an [`Id`][] from the given key-value pairs, offering a
/// shorter syntax compared to using the [`Id::builder`] directly. Additionally,
/// it allows to create a new identifier based on an existing one by passing
/// it as the first argument.
///
/// [`Id`]: crate::id::Id
/// [`Id::builder`]: crate::id::Id::builder
///
/// # Examples
///
/// Create identifier
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::id;
///
/// // Create identifier
/// let id = id!(provider = "file", context = "docs", location = "index.md")?;
/// assert_eq!(id.as_str(), "zri:file:::docs:index.md:");
/// # Ok(())
/// # }
/// ```
///
/// Create identifier from existing identifier
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::{id, Id};
///
/// // Create identifier from string
/// let id: Id = "zri:file:::docs:index.md:".parse()?;
///
/// // Create identifier from existing identifier
/// let id = id!(id; location = "README.md")?;
/// assert_eq!(id.as_str(), "zri:file:::docs:README.md:");
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! id {
    // Create identifier from the given identifier
    ($from:expr; $($key:ident = $value:expr),* $(,)?) => {{
        let mut builder = $from.to_builder();
        $(
            builder = id!(@set builder, $key, $value);
        )*
        builder.build()
    }};

    // Create identifier
    ($($key:ident = $value:expr),* $(,)?) => {{
        let mut builder = $crate::Id::builder();
        $(
            builder = id!(@set builder, $key, $value);
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
