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

//! Store key and value.

use std::fmt::Debug;
use std::hash::Hash;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Store key.
///
/// This trait defines the basic requirements for a key used in a [`Store`][].
/// We can't use specific traits bounds, e.g., [`Eq`] + [`Hash`] for hash maps
/// and [`Ord`] for ordered keys, since we would lose the ability to allow for
/// using [`Borrow`][] to generalize the key type.
///
/// Keys must implement [`Clone`], [`Debug`], [`Eq`], [`Hash`] and [`Ord`], all
/// of which we consider reasonable requirements for a generic API.
///
/// __Warning__: The `'static` lifetime which is required by this trait is a
/// deliberate design choice to simplify trait bounds across the code base. If
/// we would not require the lifetime, we would need to add a lifetime parameter
/// to almost all types using this trait, which makes it cumbersome to use.
///
/// [`Borrow`]: std::borrow::Borrow
/// [`Store`]: crate::store::Store
pub trait Key: Clone + Debug + Eq + Hash + Ord + Sized + 'static {}

/// Store value.
///
/// This trait defines the basic requirements for a value used in a [`Store`][].
/// Values must implement [`Debug`] and [`Eq`], where the latter is required to
/// allow for equality checks when inserting values with [`StoreMut::insert`][],
/// returning the prior value only if it's different.
///
/// __Warning__: The `'static` lifetime which is required by this trait is a
/// deliberate design choice to simplify trait bounds across the code base. If
/// we would not require the lifetime, we would need to add a lifetime parameter
/// to almost all types using this trait, which makes it cumbersome to use.
///
/// [`Store`]: crate::store::Store
/// [`StoreMut::insert`]: crate::store::StoreMut::insert
pub trait Value: Debug + Eq + Sized + 'static {}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> Key for T where T: Clone + Debug + Eq + Hash + Ord + 'static {}

impl<T> Value for T where T: Debug + Eq + 'static {}
