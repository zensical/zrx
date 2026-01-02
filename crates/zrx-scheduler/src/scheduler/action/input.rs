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

//! Input.

use std::fmt;

use crate::scheduler::effect::{Item, Signal};
use crate::scheduler::value::Values;

mod convert;

pub use convert::TryFromInputItem;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Input.
///
/// Inputs are short-lived containers that hold an [`Item`] with references to
/// an [`Id`][] and [`Values`] or a [`Signal`], and are designed to efficiently
/// pass arguments to an [`Action`][]. Identifiers are passed by reference, so
/// that actions can decide on their own whether cloning is necessary.
///
/// Our canonical implementation [`zrx::id`][] uses an [`Arc`][] to wrap a slice
/// of bytes, which makes cloning as cheap and fast as possible. Thus, in case
/// you don't want to roll your own identifiers, it's definitely recommended
/// to use our optimized implementation.
///
/// [`Arc`]: std::sync::Arc
/// [`Action`]: crate::scheduler::action::Action
/// [`Id`]: crate::scheduler::id::Id
/// [`zrx::id`]: https://docs.rs/zrx/latest/zrx/id/
#[derive(Clone)]
pub enum Input<'a, I> {
    /// Item.
    Item(InputItem<'a, I>),
    /// Signal.
    Signal(Signal<'a, I>),
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> fmt::Debug for Input<'_, I>
where
    I: fmt::Debug,
{
    /// Formats the input for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Input::Item(item) => item.fmt(f),
            Input::Signal(signal) => signal.fmt(f),
        }
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Input item.
pub type InputItem<'a, I> = Item<&'a I, Values<'a>>;
