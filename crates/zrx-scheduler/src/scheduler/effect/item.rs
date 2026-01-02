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

//! Item.

use crate::scheduler::value::{
    IntoOwned, Result, TryFromValues, Value, Values,
};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Item.
///
/// This data type represents an item with an identifier and associated data,
/// and is the fundamental unit of data processed by the scheduler. Identifiers
/// and their associated data can be either borrowed or owned, depending on the
/// context in which the item is created and used.
///
/// For instance, since the scheduler manages the lifetime of items, an item can
/// pass borrowed data to an [`Action`][] as part of an [`Input`][], which might
/// return owned data associated with the same or new identifier(s) as part of
/// one or more [`Outputs`][]. Both inputs and outputs expect type-erased items,
/// which are obtained through [`Item::upcast`], since the scheduler can't care
/// about the type of data it processes, as well as the associated data to be
/// wrapped in an [`Option`], because both presence and absence of an item are
/// essential for differential processing and scheduling of actions.
///
/// [`Action`]: crate::scheduler::action::Action
/// [`Input`]: crate::scheduler::action::Input
/// [`Outputs`]: crate::scheduler::action::Outputs
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item<I, T> {
    /// Identifier.
    pub id: I,
    /// Associated data.
    pub data: T,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Item<I, T> {
    /// Creates an item.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", 42);
    /// ```
    #[must_use]
    pub fn new(id: I, data: T) -> Self {
        Self { id, data }
    }

    /// Returns the identifier and associated data, consuming the item.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", 42);
    /// assert_eq!(
    ///     item.into_parts(),
    ///     ("id", 42),
    /// );
    /// ```
    #[inline]
    pub fn into_parts(self) -> (I, T) {
        (self.id, self.data)
    }

    /// Obtains ownership of the identifier and associated data.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item and obtain ownership
    /// let item = Item::new(&"id", &42);
    /// assert_eq!(
    ///     item.into_owned(),
    ///     Item::new("id", 42),
    /// );
    /// ```
    #[inline]
    pub fn into_owned(self) -> Item<I::Owned, T::Owned>
    where
        I: IntoOwned,
        T: IntoOwned,
    {
        Item {
            id: self.id.into_owned(),
            data: self.data.into_owned(),
        }
    }

    /// Maps the associated data to a different type.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item and map data
    /// let item = Item::new("id", 42);
    /// assert_eq!(
    ///     item.map(|data| data * 2),
    ///     Item::new("id", 84),
    /// );
    /// ```
    #[inline]
    pub fn map<F, U>(self, f: F) -> Item<I, U>
    where
        F: FnOnce(T) -> U,
    {
        Item {
            id: self.id,
            data: f(self.data),
        }
    }
}

impl<I, T> Item<I, Option<T>>
where
    T: Value,
{
    /// Erases the type of the associated data.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create and upcast item
    /// let item = Item::new("id", Some(42)).upcast();
    /// ```
    #[inline]
    pub fn upcast(self) -> Item<I, Option<Box<dyn Value>>> {
        let data = self.data.map(|data| Box::new(data) as _);
        Item { id: self.id, data }
    }
}

impl<'a, I> Item<I, Values<'a>> {
    /// Attempts to downcast the associated data to the given type.
    ///
    /// # Errors
    ///
    /// The following errors might occur:
    ///
    /// - [`Error::Mismatch`][]: Number of values does not match.
    /// - [`Error::Presence`][]: Value is not present, i.e., [`None`].
    /// - [`Error::Downcast`][]: Value cannot be downcast to `T`.
    ///
    /// [`Error::Mismatch`]: crate::scheduler::value::Error::Mismatch
    /// [`Error::Presence`]: crate::scheduler::value::Error::Presence
    /// [`Error::Downcast`]: crate::scheduler::value::Error::Downcast
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::effect::Item;
    /// use zrx_scheduler::values;
    ///
    /// // Create and downcast item
    /// let item = Item::new("id", values!(&42));
    /// assert_eq!(
    ///     item.downcast::<&i32>()?,
    ///     Item::new("id", &42),
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn downcast<T>(self) -> Result<Item<I, T>>
    where
        T: TryFromValues<'a>,
    {
        self.data
            .downcast::<T>()
            .map(|data| Item { id: self.id, data })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> From<(I, T)> for Item<I, T> {
    /// Creates an item from a tuple.
    ///
    /// This conversion can be convenient for creating items from iterators,
    /// as items can themselves reference borrowed, as well as owned data.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item from tuple
    /// let item = Item::from(("id", 42));
    /// ```
    #[inline]
    fn from((id, data): (I, T)) -> Self {
        Self { id, data }
    }
}

impl<I, T> From<Item<I, T>> for (I, T) {
    /// Creates a tuple from an item.
    #[inline]
    fn from(item: Item<I, T>) -> Self {
        item.into_parts()
    }
}
