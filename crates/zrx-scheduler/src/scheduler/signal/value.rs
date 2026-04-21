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

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Value.
///
/// Note that this trait must be explicitly implemented for types that should
/// be handled by the scheduler, as it's not implemented for built-in types by
/// default, which helps to make subscriptions between components intentionally
/// explicit instead of implicit.
///
/// If this trait were implemented for built-in types by default, it would be
/// too easy to accidentally subscribe to a type without realizing it, which
/// could manifest in unexpected behavior that makes it harder to debug issues.
/// So, [`Action::Inputs`][] and [`Action::Output`][] must explicitly implement
/// this trait, requiring simple types to be wrapped in newtypes, which ensures
/// that all interactions between modules are intentional and well-defined.
///
/// Implementors must implement [`Any`], [`Clone`], [`Debug`], [`Eq`], as well
/// as [`Send`] and [`Sync`], so all values can be shared across threads and
/// printed for debugging. [`Clone`] is required for distributing values to the
/// subscribers of an [`Action`][], rendering [`Value`] as not dyn-compatible.
/// However, this is not a problem and actually a good thing, as values should
/// not be downcast, rather the stores and receivers that manage them.
///
/// [`Action`]: crate::scheduler::action::Action
/// [`Action::Inputs`]: crate::scheduler::action::Action::Inputs
/// [`Action::Output`]: crate::scheduler::action::Action::Output
pub trait Value: Any + Clone + Debug + Eq + Send + Sync {}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for () {}

impl<T> Value for Option<T> where T: Value {}

impl<T> Value for Vec<T> where T: Value {}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements value trait for a tuple.
macro_rules! impl_value_for_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<$($T),+> Value for ($($T,)+)
        where
            $($T: Value),+ {}
    };
}

// ----------------------------------------------------------------------------

impl_value_for_tuple!(T1);
impl_value_for_tuple!(T1, T2);
impl_value_for_tuple!(T1, T2, T3);
impl_value_for_tuple!(T1, T2, T3, T4);
impl_value_for_tuple!(T1, T2, T3, T4, T5);
impl_value_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_value_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_value_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
