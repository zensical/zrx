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

//! Value.

use std::any::Any;
use std::fmt::Debug;
use std::time::{Duration, Instant};

use zrx_id::Id;

use super::executor::graph::View;

mod borrow;
mod collection;
mod convert;
mod error;
mod ext;
mod tuple;

pub use borrow::IntoOwned;
pub use collection::Values;
pub use convert::{TryFromValue, TryFromValues};
pub use error::{Error, Result};
pub use ext::ValueExt;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Value.
///
/// This trait represents values which are the means of passing and returning
/// data to and from an [`Action`][]. As scheduling must happen independently
/// of concrete data types, values are upcasted to be passed between actions.
/// While this adds the overhead of dynamic dispatch, as well as downcasting,
/// it's the only way to implement a generic scheduler, allowing for different
/// types of values to be passed around without knowing their concrete types.
///
/// Note that this trait must be explicitly implemented, since it is also used
/// as a marker trait in several occasions, e.g., when returning results from
/// fallible actions, or to discern potentially conflicting implementations of
/// function traits used in operators.
///
/// Implementors must implement the [`Any`], [`Debug`] and [`Send`] traits, so
/// values can be shared across thread boundaries and printed during debugging.
///
/// [`Action`]: crate::scheduler::action::Action
pub trait Value: Any + Debug + Send {}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl dyn Value {
    /// Attempts to downcast the value to a reference of `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Value;
    ///
    /// // Create and downcast value
    /// let value: Box<dyn Value> = Box::new(42);
    /// assert_eq!(
    ///     value.downcast::<i32>(),
    ///     Some(42),
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn downcast<T>(self: Box<Self>) -> Option<T>
    where
        T: Any,
    {
        (self as Box<dyn Any>)
            .downcast::<T>()
            .map(|value| *value)
            .ok()
    }

    /// Attempts to downcast the value to a reference of `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Value;
    ///
    /// // Create and downcast value
    /// let value: &dyn Value = &42;
    /// assert_eq!(
    ///     value.downcast_ref::<i32>(),
    ///     Some(&42),
    /// );
    /// ```
    #[inline]
    pub fn downcast_ref<T>(&self) -> Option<&T>
    where
        T: Any,
    {
        (self as &dyn Any).downcast_ref::<T>()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for Box<dyn Value> {}

impl<T> Value for Vec<T> where T: Value {}

impl<T> Value for Option<T> where T: Value {}

// ----------------------------------------------------------------------------

/// Implements value trait for the given types.
macro_rules! impl_values {
    ($($T:ty),*) => {
        $(
            impl Value for $T {}
        )+
    };
}

// ----------------------------------------------------------------------------

impl_values!(());
impl_values!(bool);
impl_values!(u8, u16, u32, u64, u128, usize);
impl_values!(i8, i16, i32, i64, i128, isize);
impl_values!(f32, f64);
impl_values!(&'static str, String);
impl_values!(Duration, Instant);

// ----------------------------------------------------------------------------

impl_values!(Id);
