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

//! Helper traits.

use std::cell::RefCell;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// With shared mutability.
///
/// This trait is solely intended for internal use, and thus not exported. It
/// provides a way to conveniently access and modify shared state in scope.
pub trait With {
    /// Type of the inner state.
    type Item;

    /// Returns a reference to the inner state.
    fn inner(&self) -> &RefCell<Self::Item>;

    /// Passes a reference to the inner state to the function.
    #[inline]
    fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&Self::Item) -> R,
    {
        f(&*self.inner().borrow())
    }

    /// Passes a mutable reference to the inner state to the function.
    #[inline]
    fn with_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut Self::Item) -> R,
    {
        f(&mut *self.inner().borrow_mut())
    }
}
