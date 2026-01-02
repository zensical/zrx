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

//! Input conversions.

use crate::scheduler::effect::Item;
use crate::scheduler::value::{Result, TryFromValues};

use super::InputItem;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Attempt conversion from [`InputItem`].
pub trait TryFromInputItem<'a, I>: Sized {
    /// Attempts to convert from an input item.
    ///
    /// # Errors
    ///
    /// In case conversion fails, an error should be returned. Since this trait
    /// is intended to be used in a low-level context, orchestrating the flow of
    /// values between actions, the errors just carry enough information so the
    /// reason of the failure can be determined during development.
    fn try_from_input_item(item: InputItem<'a, I>) -> Result<Self>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I, T> TryFromInputItem<'a, I> for Item<&'a I, T>
where
    T: TryFromValues<'a>,
{
    /// Attempts to convert into an item with data of type `T`.
    #[inline]
    fn try_from_input_item(item: InputItem<'a, I>) -> Result<Self> {
        item.downcast().map(|item| Item::new(item.id, item.data))
    }
}
