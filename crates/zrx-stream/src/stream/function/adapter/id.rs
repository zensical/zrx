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

//! Function adapter to pass the identifier to the function.

use std::ops::Deref;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Function adapter to pass the identifier to the function.
///
/// This adapter is just a wrapper around a function and acts as a marker to
/// omit conflicting implementations of function traits with more specialized
/// implementations. This implementation adds an identifier to the function
/// signature, which is necessary in certain contexts.
///
/// Use the [`with_id`] function to wrap a function with this adapter, since
/// this type is not part of the streaming API, as it's only necessary for the
/// implementation of the function traits themselves. See the documentation of
/// [`with_id`] for more information.
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct WithId<F> {
    /// Function.
    function: F,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<F> Deref for WithId<F> {
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

/// Passes the identifier to the function.
///
/// Function traits that implement this trait allow to pass a function that
/// receives the identifier of the item as the first argument. This is useful
/// when the function needs to know the identifier of the item being processed,
/// which is necessary in certain contexts.
///
/// Note that there are no trait bounds, since [`WithId`] is merely a marker
/// struct that is used to differentiate between function implementations. This
/// is also why the returned type implements [`Deref`], so that it behaves like
/// the underlying function.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_stream::function::{with_id, InspectFn};
///
/// // Define and execute function
/// let f = |&id: &&str, &n: &i32| println!("{id} -> {n}");
/// with_id(f).execute(&"id", &42)?;
/// # Ok(())
/// # }
/// ```
#[inline]
pub fn with_id<F>(f: F) -> WithId<F> {
    WithId { function: f }
}
