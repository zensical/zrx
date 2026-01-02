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

//! Collection of items.

use std::any::Any;
use std::fmt::Debug;

use zrx_scheduler::{Id, Value};
use zrx_store::{Store, StoreIterable, StoreKeys, StoreValues};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Collection of items.
///
/// This data type combines the most common traits from our store abstractions
/// into a single trait that is _dyn-compatible_, so it can be used in operator
/// functions to erase the concrete store type, which is central to hiding the
/// implementation details of operators.
///
/// In order to automatically implement the [`Value`] trait for all data types
/// that implement [`Store`] and friends, we must implement [`Value`] on the
/// trait object `dyn Collection<I, T>`. Thus, we must require the supertraits
/// of [`Value`] on this trait, and for the blanket implementation. In case more
/// supertraits are added to [`Value`] in the future, we must add them here as
/// well, which is however unlikely to happen.
pub trait Collection<I, T>: Any + Debug + Send {
    /// Returns a reference to the value identified by the key.
    fn get(&self, id: &I) -> Option<&T>;

    /// Returns whether the collection contains the key.
    fn contains_key(&self, id: &I) -> bool;

    /// Returns the number of items in the collection.
    fn len(&self) -> usize;

    /// Returns whether the collection is empty.
    fn is_empty(&self) -> bool;

    /// Creates an iterator over the store.
    fn iter(&self) -> Iter<'_, I, T>;

    /// Creates a key iterator over the store.
    fn keys(&self) -> Keys<'_, I>;

    /// Creates a value iterator over the store.
    fn values(&self) -> Values<'_, T>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Value for dyn Collection<I, T>
where
    I: Id,
    T: Value,
{
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<I, T, S> Collection<I, T> for S
where
    I: Id,
    T: Value,
    S: Any + Debug + Send,
    S: Store<I, T> // fmt
        + StoreIterable<I, T>
        + StoreKeys<I, T>
        + StoreValues<I, T>,
{
    /// Returns a reference to the value identified by the key.
    #[inline]
    fn get(&self, id: &I) -> Option<&T> {
        Store::get(self, id)
    }

    /// Returns whether the collection contains the key.
    #[inline]
    fn contains_key(&self, id: &I) -> bool {
        Store::contains_key(self, id)
    }

    /// Returns the number of items in the collection.
    #[inline]
    fn len(&self) -> usize {
        Store::len(self)
    }

    /// Creates an iterator over the store.
    #[inline]
    fn is_empty(&self) -> bool {
        Store::is_empty(self)
    }

    /// Creates an iterator over the store.
    #[inline]
    fn iter(&self) -> Iter<'_, I, T> {
        Box::new(StoreIterable::iter(self))
    }

    /// Creates a key iterator over the store.
    #[inline]
    fn keys(&self) -> Keys<'_, I> {
        Box::new(StoreKeys::keys(self))
    }

    /// Creates a value iterator over the store.
    #[inline]
    fn values(&self) -> Values<'_, T> {
        Box::new(StoreValues::values(self))
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Collection iterator.
type Iter<'a, I, T> = Box<dyn Iterator<Item = (&'a I, &'a T)> + 'a>;

/// Collection key iterator.
type Keys<'a, I> = Box<dyn Iterator<Item = &'a I> + 'a>;

/// Collection value iterator.
type Values<'a, T> = Box<dyn Iterator<Item = &'a T> + 'a>;
