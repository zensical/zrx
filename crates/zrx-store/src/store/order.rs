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

//! Comparator factories.

use std::cmp::Ordering;

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Comparator.
///
/// # Examples
///
/// ```
/// use std::cmp::Ordering;
/// use zrx_store::order;
///
/// // Transform value before comparison
/// let comparator = order::by(|value| -value);
/// assert_eq!(comparator(&42, &84), Ordering::Greater);
///
/// // Transform ordering after comparison
/// let comparator = order::with(Ordering::reverse);
/// assert_eq!(comparator(&42, &84), Ordering::Greater);
/// ```
pub type Comparator<T> = Box<dyn Fn(&T, &T) -> Ordering>;

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Returns a comparator that orders by a selector.
///
/// # Examples
///
/// ```
/// use std::collections::HashMap;
/// use zrx_store::decorator::Indexed;
/// use zrx_store::{order, StoreMut};
///
/// // Create store with custom order
/// let f = order::by(|value| -value);
/// let mut store = Indexed::<_, _, HashMap<_, _>>::with_order(f);
///
/// // Insert values
/// store.insert("a", 42);
/// store.insert("b", 84);
/// ```
#[inline]
pub fn by<F, T, U>(f: F) -> impl Fn(&T, &T) -> Ordering
where
    F: Fn(&T) -> U,
    U: Ord,
{
    move |a, b| f(a).cmp(&f(b))
}

/// Returns a comparator that orders with the given function.
///
/// # Examples
///
/// ```
/// use std::cmp::Ordering;
/// use std::collections::HashMap;
/// use zrx_store::decorator::Indexed;
/// use zrx_store::{order, StoreMut};
///
/// // Create store with custom order
/// let f = order::with(Ordering::reverse);
/// let mut store = Indexed::<_, _, HashMap<_, _>>::with_order(f);
///
/// // Insert values
/// store.insert("a", 42);
/// store.insert("b", 84);
/// ```
#[inline]
pub fn with<F, T>(f: F) -> impl Fn(&T, &T) -> Ordering
where
    F: Fn(Ordering) -> Ordering,
    T: Ord,
{
    move |a, b| f(a.cmp(b))
}
