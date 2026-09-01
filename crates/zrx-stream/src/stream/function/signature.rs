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

//! Function signature.

#![cfg_attr(not(feature = "tracing"), allow(unused_variables))]

use std::marker::PhantomData;
use std::ops::Deref;

use super::arguments::{WithId, WithKey, WithSplat, WithValue};

mod map;
mod reduce;

pub use map::MapFn;
pub use reduce::ReduceFn;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Function signature.
pub struct Signature<F, A> {
    /// Function.
    function: F,
    /// Capture types.
    marker: PhantomData<fn() -> A>,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<F, A> Clone for Signature<F, A>
where
    F: Clone,
{
    /// Clones the function signature.
    #[inline]
    fn clone(&self) -> Self {
        Self {
            function: self.function.clone(),
            marker: PhantomData,
        }
    }
}

impl<F, A> Deref for Signature<F, A> {
    type Target = F;

    /// Dereferences to the function.
    fn deref(&self) -> &Self::Target {
        &self.function
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Selects the signature with a key argument.
///
/// This function can be used to deliberately select a function signature if
/// there are multiple signatures available for a given function. It's usually
/// not necessary to use this function, as the compiler can usually infer the
/// correct signature based on the context.
#[inline]
#[must_use]
pub fn with_key<F>(f: F) -> Signature<F, WithKey> {
    Signature {
        function: f,
        marker: PhantomData,
    }
}

/// Selects the signature with an identifier argument.
///
/// This function can be used to deliberately select a function signature if
/// there are multiple signatures available for a given function. It's usually
/// not necessary to use this function, as the compiler can usually infer the
/// correct signature based on the context.
#[inline]
#[must_use]
pub fn with_id<F>(f: F) -> Signature<F, WithId> {
    Signature {
        function: f,
        marker: PhantomData,
    }
}

/// Selects the signature with a value argument.
///
/// This function can be used to deliberately select a function signature if
/// there are multiple signatures available for a given function. It's usually
/// not necessary to use this function, as the compiler can usually infer the
/// correct signature based on the context.
#[inline]
#[must_use]
pub fn with_value<F>(f: F) -> Signature<F, WithValue> {
    Signature {
        function: f,
        marker: PhantomData,
    }
}

/// Selects the signature with a splat argument.
///
/// This function can be used to deliberately select a function signature if
/// there are multiple signatures available for a given function. It's usually
/// not necessary to use this function, as the compiler can usually infer the
/// correct signature based on the context.
#[inline]
#[must_use]
pub fn with_splat<F>(f: F) -> Signature<F, WithSplat> {
    Signature {
        function: f,
        marker: PhantomData,
    }
}
