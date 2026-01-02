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

//! Value collection.

use std::fmt::Debug;
use std::vec::IntoIter;

use super::convert::TryFromValues;
use super::error::Result;
use super::{Value, View};

mod macros;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Value collection.
///
/// This data type contains a temporary collection of optional value references
/// that can be consumed via the iterator interface, or downcast to a specific
/// type by leveraging the [`TryFromValues`] trait. It can be created using the
/// [`values!`][] macro, or by selecting a [`View`] from a storage.
///
/// For more information on what [`Values`][] can be downcast into, refer to
/// the documentation of the implementations of the [`TryFromValues`] trait.
///
/// [`values!`]: crate::values!
/// [`Values`]: crate::scheduler::value::Values
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_scheduler::values;
///
/// // Create and downcast value collection
/// let values = values!(&42);
/// let target = values.downcast::<&i32>()?;
/// assert_eq!(target, &42);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub enum Values<'a> {
    /// Iterator over values.
    Iter(IntoIter<Option<&'a dyn Value>>),
    /// Iterator over a view of values.
    View(View<'a>),
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a> Values<'a> {
    /// Attempts to downcast the value collection to the given type.
    ///
    /// Note that downcasting is implemented as part of the [`TryFromValues`]
    /// trait, so any type that implements it can be passed as `T`. This might
    /// not only include single values, but also vectors and tuples of values,
    /// representing variadic collections in an efficient way.
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
    /// use zrx_scheduler::values;
    ///
    /// // Create and downcast value collection
    /// let values = values!(&42);
    /// let target = values.downcast::<&i32>()?;
    /// assert_eq!(target, &42);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn downcast<T>(self) -> Result<T>
    where
        T: TryFromValues<'a>,
    {
        // Theoretically, we could just call `T::try_from_values` with `self`,
        // but by unpacking the variants here, we only ever have to branch once
        match self {
            Values::Iter(iter) => T::try_from_values(iter),
            Values::View(view) => T::try_from_values(view),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> Iterator for Values<'a> {
    type Item = Option<&'a dyn Value>;

    /// Returns the next optional value reference.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::values;
    ///
    /// // Create iterator over value collection
    /// let mut values = values!(&42);
    /// while let Some(value) = values.next() {
    ///     println!("{value:?}");
    /// }
    /// ```
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Values::Iter(iter) => iter.next(),
            Values::View(view) => view.next(),
        }
    }

    /// Returns the bounds of the value collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::values;
    ///
    /// // Get bounds of value collection
    /// let values = values!(&42);
    /// assert_eq!(values.size_hint(), (1, Some(1)));
    /// ```
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Values::Iter(iter) => iter.size_hint(),
            Values::View(view) => view.size_hint(),
        }
    }
}

impl ExactSizeIterator for Values<'_> {
    /// Returns the length of the value collection.
    ///
    /// This method returns the remaining length of the value collection, so
    /// when it returns `0` when the iterator has been consumed completely.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::values;
    ///
    /// // Get length of value collection
    /// let values = values!(&42);
    /// assert_eq!(values.len(), 1);
    /// ```
    #[inline]
    fn len(&self) -> usize {
        match self {
            Values::Iter(iter) => iter.len(),
            Values::View(view) => view.len(),
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a> FromIterator<Option<&'a dyn Value>> for Values<'a> {
    /// Creates a value collection from an iterator.
    ///
    /// While [`Values`] is an iterator itself, we need to support the creation
    /// of value collections from any kind of iterator of optional references
    /// to values, e.g., as needed by the [`values!`][] macro, allowing us to
    /// pass in anything that is itself an iterator. If it would be possible
    /// for us to consume a slice, we would just use that, but any slice first
    /// needs to be collected into a vector, so here we are.
    ///
    /// [`values!`]: crate::values!
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::value::{Value, Values};
    ///
    /// // Create value collection from iterator
    /// let values = Values::from_iter([
    ///     Some(&1 as &dyn Value),
    ///     Some(&2 as &dyn Value),
    ///     Some(&3 as &dyn Value),
    /// ]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Option<&'a dyn Value>>,
    {
        Values::Iter(Vec::from_iter(iter).into_iter())
    }
}
