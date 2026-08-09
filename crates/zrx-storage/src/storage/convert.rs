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

//! Storage conversions.

use std::any::Any;
use std::fmt::Debug;

use zrx_store::{Key, Value};

use super::{Error, Result, Storage};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Attempt conversion into a [`Storage`] reference.
///
/// This trait implements conversion of an [`Any`] reference to an immutable
/// [`Storage`] reference, which is used as the inputs of a scheduler action.
pub trait TryAsStorage<K>: Value {
    /// Attempts to convert into a storage reference.
    ///
    /// # Errors
    ///
    /// In case conversion fails, an error should be returned. Since this trait
    /// is intended to be used in a low-level context, orchestrating conversion
    /// of storages within actions, the errors just carry enough information so
    /// the reason of the failure can be determined during development.
    fn try_as_storage(item: &dyn Any) -> Result<&Storage<K, Self>>;
}

/// Attempt conversion into a mutable [`Storage`] reference.
///
/// This trait implements conversion of a mutable [`Any`] reference to a mutable
/// [`Storage`] reference, which is used as the output of a scheduler action.
pub trait TryAsStorageMut<K>: Value {
    /// Attempts to convert into a mutable storage reference.
    ///
    /// # Errors
    ///
    /// In case conversion fails, an error should be returned. Since this trait
    /// is intended to be used in a low-level context, orchestrating conversion
    /// of storages within actions, the errors just carry enough information so
    /// the reason of the failure can be determined during development.
    fn try_as_storage_mut(item: &mut dyn Any) -> Result<&mut Storage<K, Self>>;
}

// ----------------------------------------------------------------------------

/// Attempt conversion into a [`Storage`] sequence or tuple.
///
/// This trait implements conversion of an iterator of [`Any`] references to
/// one or more storages, which can be a sequence or tuple.
///
/// __Warning__: Implementation requires the use of generic associated types,
/// as lifetimes need to be passed through the conversion process.
pub trait TryAsStorages<K>: Value {
    /// Target type of conversion.
    type Target<'a>: Debug;

    /// Attempts to convert into a tuple of storage references.
    ///
    /// While 1-tuples are converted to a single storage reference, tuples with
    /// multiple items are converted to a tuple of storage references, which is
    /// more ergonomic to work with in the context of actions, since 1-tuples
    /// would require awkward destructuring.
    ///
    /// # Errors
    ///
    /// The following errors might be returned:
    ///
    /// - [`Error::Mismatch`]: Number of items does not match.
    /// - [`Error::Downcast`]: Item cannot be downcast.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::any::Any;
    /// use zrx_storage::convert::TryAsStorages;
    /// use zrx_storage::Storage;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("key", 42)]);
    /// let b = Storage::from_iter([("key", true)]);
    ///
    /// // Obtain type-erased references
    /// let iter: Vec<&dyn Any> = vec![&a, &b];
    ///
    /// // Obtain storage references
    /// let storages = <(i32, bool)>::try_as_storages(iter)?;
    /// # let _: (&Storage<&str, _>, _) = storages;
    /// # Ok(())
    /// # }
    /// ```
    fn try_as_storages<'a, T>(iter: T) -> Result<Self::Target<'a>>
    where
        T: IntoIterator<Item = &'a dyn Any>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<K, V> TryAsStorage<K> for V
where
    K: Key,
    V: Value,
{
    /// Attempts to convert into a storage reference.
    ///
    /// # Errors
    ///
    /// The following errors might be returned:
    ///
    /// - [`Error::Downcast`]: Item cannot be downcast.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::any::Any;
    /// use zrx_storage::convert::TryAsStorage;
    /// use zrx_storage::Storage;
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::default();
    /// storage.insert("key", 42);
    ///
    /// // Obtain type-erased reference
    /// let item: &dyn Any = &storage;
    ///
    /// // Obtain storage reference
    /// let storage = <i32>::try_as_storage(item)?;
    /// # let _: &Storage<&str, _> = storage;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_as_storage(item: &dyn Any) -> Result<&Storage<K, Self>> {
        item.downcast_ref().ok_or(Error::Downcast)
    }
}

impl<K, V> TryAsStorageMut<K> for V
where
    K: Key,
    V: Value,
{
    /// Attempts to convert into a mutable storage reference.
    ///
    /// # Errors
    ///
    /// The following errors might be returned:
    ///
    /// - [`Error::Downcast`]: Item cannot be downcast.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::any::Any;
    /// use zrx_storage::convert::TryAsStorageMut;
    /// use zrx_storage::Storage;
    ///
    /// // Create storage and initial state
    /// let mut storage = Storage::default();
    /// storage.insert("key", 42);
    ///
    /// // Obtain mutable type-erased reference
    /// let item: &mut dyn Any = &mut storage;
    ///
    /// // Obtain mutable storage reference
    /// let storage = <i32>::try_as_storage_mut(item)?;
    /// # let _: &mut Storage<&str, _> = storage;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_as_storage_mut(item: &mut dyn Any) -> Result<&mut Storage<K, Self>> {
        item.downcast_mut().ok_or(Error::Downcast)
    }
}

// ----------------------------------------------------------------------------

impl<K> TryAsStorages<K> for ()
where
    K: Key,
{
    type Target<'a> = ();

    /// Attempts to convert into a unit value.
    #[inline]
    fn try_as_storages<'a, T>(iter: T) -> Result<Self::Target<'a>>
    where
        T: IntoIterator<Item = &'a dyn Any>,
    {
        match iter.into_iter().next() {
            Some(_) => Err(Error::Mismatch),
            None => Ok(()),
        }
    }
}

impl<K, V> TryAsStorages<K> for Vec<V>
where
    K: Key,
    V: TryAsStorage<K>,
{
    type Target<'a> = Vec<&'a Storage<K, V>>;

    /// Attempts to convert into a sequence of storage references.
    ///
    /// # Errors
    ///
    /// The following errors might be returned:
    ///
    /// - [`Error::Downcast`]: Item cannot be downcast.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::any::Any;
    /// use zrx_storage::convert::TryAsStorages;
    /// use zrx_storage::Storage;
    ///
    /// // Create storages from iterators
    /// let a = Storage::from_iter([("key", 42)]);
    /// let b = Storage::from_iter([("key", 84)]);
    ///
    /// // Obtain type-erased references
    /// let iter: Vec<&dyn Any> = vec![&a, &b];
    ///
    /// // Obtain storage references
    /// let storages = <Vec<i32>>::try_as_storages(iter)?;
    /// # let _: Vec<&Storage<&str, _>> = storages;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_as_storages<'a, T>(iter: T) -> Result<Self::Target<'a>>
    where
        T: IntoIterator<Item = &'a dyn Any>,
    {
        iter.into_iter().map(V::try_as_storage).collect()
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements storage conversion trait for a tuple.
macro_rules! impl_try_as_storages_for_tuple {
    ($($V:ident),+ $(,)?) => {
        impl<K, $($V),+> TryAsStorages<K> for ($($V,)+)
        where
            K: Key,
            $($V: TryAsStorage<K>,)+
        {
            #[allow(unused_parens)]
            type Target<'a> = ($(&'a Storage<K, $V>),+);

            #[inline]
            fn try_as_storages<'a, T>(iter: T) -> Result<Self::Target<'a>>
            where
                T: IntoIterator<Item = &'a dyn Any>,
            {
                let mut iter = iter.into_iter();
                $(
                    #[allow(non_snake_case)]
                    let $V = $V::try_as_storage(
                        iter.next().ok_or(Error::Mismatch)?
                    )?;
                )+

                // Ensure that the iterator yields no more values
                if iter.next().is_none() {
                    Ok(($($V),+))
                } else {
                    Err(Error::Mismatch)
                }
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_try_as_storages_for_tuple!(V1);
impl_try_as_storages_for_tuple!(V1, V2);
impl_try_as_storages_for_tuple!(V1, V2, V3);
impl_try_as_storages_for_tuple!(V1, V2, V3, V4);
impl_try_as_storages_for_tuple!(V1, V2, V3, V4, V5);
impl_try_as_storages_for_tuple!(V1, V2, V3, V4, V5, V6);
impl_try_as_storages_for_tuple!(V1, V2, V3, V4, V5, V6, V7);
impl_try_as_storages_for_tuple!(V1, V2, V3, V4, V5, V6, V7, V8);
