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

//! Tuple.

use zrx_scheduler::value::{IntoOwned, TryFromValues, Value};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Presence.
pub trait Presence: 'static {}

// ----------------------------------------------------------------------------

/// Tuple.
///
/// This trait is used to define different configurations of tuples for join
/// operators, which centers around the presence of values in the tuple. Note
/// that the argument type defines the expected values in the tuple, and is
/// used as the join operator's own argument type.
///
/// In order to isolate lifetimes in the trait, and to omit the need for using
/// higher-ranked trait bounds in each operator, the implementations account
/// for the lifetimes with said higher-ranked trait bounds. While they are
/// rather ugly, there's no other way to express it without them.
///
/// This trait doesn't have a bound on [`Presence`], so we can keep operator
/// implementations simpler, but it's enforced on wrappers that use it.
pub trait Tuple<P>: Value {
    /// Tuple arguments type.
    type Arguments<'a>: TryFromValues<'a> + IntoOwned<Owned = Self>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// All items are required.
pub struct All;

/// First item is required.
pub struct First;

/// All items are optional.
pub struct Any;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Presence for All {}

impl Presence for First {}

impl Presence for Any {}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements tuple trait with all items required.
macro_rules! impl_tuple_all {
    ($($T:ident),+ $(,)?) => {
        impl<$($T),+> Tuple<All> for ($($T,)+)
        where
            Self: Value,
            for<'a> ($(&'a $T,)+):
                TryFromValues<'a> + IntoOwned<Owned = Self>,
        {
            type Arguments<'a> = ($(&'a $T,)*);
        }
    };
}

/// Implements tuple trait with first item required.
macro_rules! impl_tuple_first {
    ($T1:ident $(, $T:ident)* $(,)?) => {
        impl<$T1 $(, $T)*> Tuple<First> for ($T1, $(Option<$T>),*)
        where
            Self: Value,
            for<'a> (&'a $T1, $(Option<&'a $T>),*):
                TryFromValues<'a> + IntoOwned<Owned = Self>,
        {
            type Arguments<'a> = (&'a $T1, $(Option<&'a $T>),*);
        }
    };
}

/// Implements tuple trait with all items optional.
macro_rules! impl_tuple_any {
    ($($T:ident),+ $(,)?) => {
        impl<$($T),+> Tuple<Any> for ($(Option<$T>,)+)
        where
            Self: Value,
            for<'a> ($(Option<&'a $T>,)+):
                TryFromValues<'a> + IntoOwned<Owned = Self>,
        {
            type Arguments<'a> = ($(Option<&'a $T>,)*);
        }
    };
}

/// Implements tuple traits.
macro_rules! impl_tuple {
    ($($T:ident),+ $(,)?) => {
        impl_tuple_all!($($T),+);
        impl_tuple_first!($($T),+);
        impl_tuple_any!($($T),+);
    };
}

// ----------------------------------------------------------------------------

impl_tuple!(T1);
impl_tuple!(T1, T2);
impl_tuple!(T1, T2, T3);
impl_tuple!(T1, T2, T3, T4);
impl_tuple!(T1, T2, T3, T4, T5);
impl_tuple!(T1, T2, T3, T4, T5, T6);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
