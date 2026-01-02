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

//! Function adapter to splat the function's arguments.

use std::ops::Deref;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Function adapter to splat the function's arguments.
///
/// This adapter is just a wrapper around a function and acts as a marker to
/// omit conflicting implementations of function traits with more specialized
/// implementations. This implementation passes the arguments as a [`Splat`][]
/// to the function, allowing to conveniently work with multiple arguments.
///
/// Use the [`with_splat`] function to wrap a function with this adapter, since
/// this type is not part of the streaming API, as it's only necessary for the
/// implementation of the function traits themselves. See the documentation of
/// [`with_splat`] for more information.
///
/// [`Splat`]: crate::stream::function::Splat
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct WithSplat<F> {
    /// Function.
    function: F,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<F> Deref for WithSplat<F> {
    type Target = F;

    /// Dereferences to the wrapped function.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.function
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Splats the function's arguments.
///
/// Function traits that implement this trait allow to pass a function that
/// receives a splat argument instead of a single tuple argument.
///
/// Note that there are no trait bounds, since [`WithSplat`] is merely a marker
/// struct that is used to differentiate between function implementations. This
/// is also why the returned type implements [`Deref`], so that it behaves like
/// the underlying function.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_stream::function::{with_splat, InspectFn};
///
/// // Define and execute function
/// let f = |&a: &i32, &b: &i32| println!("({a}, {b})");
/// with_splat(f).execute(&"id", &(1, 2))?;
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn with_splat<F>(f: F) -> WithSplat<F> {
    WithSplat { function: f }
}
