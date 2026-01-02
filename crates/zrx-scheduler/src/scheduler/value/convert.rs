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

//! Value conversions.

use super::error::{Error, Result};
use super::Value;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Attempt conversion from [`Value`].
///
/// This trait implements conversion of an optional reference to a type-erased
/// value into a reference or optional reference of a concrete type `T`. When
/// trying to convert [`None`] into a non-optional reference, the conversion
/// returns an error to indicate the absence of the value.
pub trait TryFromValue<'a>: Sized + 'a {
    /// Attempts to convert from an optional value.
    ///
    /// # Errors
    ///
    /// In case conversion fails, an error should be returned. Since this trait
    /// is intended to be used in a low-level context, orchestrating the flow
    /// of values between actions, the errors just carry enough information so
    /// the reason of the failure can be determined during development.
    fn try_from_value(opt: Option<&'a dyn Value>) -> Result<Self>;
}

// ----------------------------------------------------------------------------

/// Attempt conversion from a [`Value`] iterator.
///
/// This trait implements conversion of an iterator of optional references to
/// type-erased values into a concrete type `T`, which can be a single value,
/// a vector of values, or even a tuple of values. An iterator of [`Value`]
/// trait objects is expected to be passed.
///
/// Internally, this trait invokes [`TryFromValue::try_from_value`] for each
/// value yielded by an iterator, allowing for type-safe conversion.
///
/// # Errors
///
/// The following errors might occur:
///
/// - [`Error::Mismatch`]: Number of values does not match.
/// - [`Error::Presence`]: Value is not present, i.e., [`None`].
/// - [`Error::Downcast`]: Value cannot be downcast.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_scheduler::value::TryFromValues;
/// use zrx_scheduler::values;
///
/// // Create and convert values
/// let values = values!(&1, &2, &3);
/// let target = <(&i32, Option<&i32>, &i32)>::try_from_values(values)?;
/// assert_eq!(target, (&1, Some(&2), &3));
/// # Ok(())
/// # }
/// ```
pub trait TryFromValues<'a>: Sized + 'a {
    /// Attempts to convert from an iterator of optional values.
    ///
    /// The canonical way to use this method is to pass [`Values`][] to it - a
    /// collection of optional value references as returned by [`values!`][].
    /// However, any iterator of optional references to type-erased [`Value`]
    /// trait objects can be passed, making it more flexible.
    ///
    /// [`values!`]: crate::values!
    /// [`Values`]: crate::scheduler::value::Values
    ///
    /// # Errors
    ///
    /// In case conversion fails, an error should be returned. Since this trait
    /// is intended to be used in a low-level context, orchestrating the flow of
    /// values between actions, the errors just carry enough information so the
    /// reason of the failure can be determined during development.
    fn try_from_values<V>(values: V) -> Result<Self>
    where
        V: IntoIterator<Item = Option<&'a dyn Value>>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<'a, T> TryFromValue<'a> for &'a T
where
    T: Value,
{
    /// Attempts to convert into a reference of `T`.
    ///
    /// # Errors
    ///
    /// The following errors might occur:
    ///
    /// - [`Error::Presence`]: Value is not present, i.e., [`None`].
    /// - [`Error::Downcast`]: Value cannot be downcast to `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::{TryFromValue, Value};
    ///
    /// // Create and convert optional value
    /// let opt = Some(&42 as &dyn Value);
    /// let target = <&i32>::try_from_value(opt)?;
    /// assert_eq!(target, &42);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from_value(opt: Option<&'a dyn Value>) -> Result<Self> {
        opt.map_or(Err(Error::Presence), |value| {
            value.downcast_ref::<T>().ok_or(Error::Downcast)
        })
    }
}

impl<'a, T> TryFromValue<'a> for Option<&'a T>
where
    T: Value,
{
    /// Attempts to convert into an optional reference of `T`.
    ///
    /// # Errors
    ///
    /// The following errors might occur:
    ///
    /// - [`Error::Downcast`]: Value cannot be downcast to `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::{TryFromValue, Value};
    ///
    /// // Create and convert optional value
    /// let opt = Some(&42 as &dyn Value);
    /// let target = <Option<&i32>>::try_from_value(opt)?;
    /// assert_eq!(target, Some(&42));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from_value(opt: Option<&'a dyn Value>) -> Result<Self> {
        opt.map_or(Ok(None), |value| {
            value.downcast_ref::<T>().ok_or(Error::Downcast).map(Some)
        })
    }
}

// ----------------------------------------------------------------------------

impl<'a, T> TryFromValues<'a> for T
where
    T: TryFromValue<'a>,
{
    /// Attempts to convert into a value.
    ///
    /// # Errors
    ///
    /// The following errors might occur:
    ///
    /// - [`Error::Mismatch`]: Number of values is not exactly one.
    /// - [`Error::Presence`]: Value is not present, i.e., [`None`].
    /// - [`Error::Downcast`]: Value cannot be downcast to `T`.
    ///
    /// # Examples
    ///
    /// Convert into a reference:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::TryFromValues;
    /// use zrx_scheduler::values;
    ///
    /// // Create and convert values
    /// let values = values!(&42);
    /// let target = <&i32>::try_from_values(values)?;
    /// assert_eq!(target, &42);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Convert into an optional reference:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::TryFromValues;
    /// use zrx_scheduler::values;
    ///
    /// // Create and convert values
    /// let values = values!(&42);
    /// let target = <Option<&i32>>::try_from_values(values)?;
    /// assert_eq!(target, Some(&42));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from_values<V>(values: V) -> Result<Self>
    where
        V: IntoIterator<Item = Option<&'a dyn Value>>,
    {
        let mut iter = values.into_iter();

        // Ensure that the iterator yields exactly one value
        if let (Some(opt), None) = (iter.next(), iter.next()) {
            T::try_from_value(opt)
        } else {
            Err(Error::Mismatch)
        }
    }
}

impl<'a, T> TryFromValues<'a> for Vec<T>
where
    T: TryFromValue<'a>,
{
    /// Attempts to convert into a vector of values.
    ///
    /// # Errors
    ///
    /// The following errors might occur:
    ///
    /// - [`Error::Presence`]: Value is not present, i.e., [`None`].
    /// - [`Error::Downcast`]: Value cannot be downcast to `T`.
    ///
    /// # Examples
    ///
    /// Convert into vector of references:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::TryFromValues;
    /// use zrx_scheduler::values;
    ///
    /// // Create and convert values
    /// let values = values!(&1, &2, &3);
    /// let target = <Vec<&i32>>::try_from_values(values)?;
    /// assert_eq!(target, vec![&1, &2, &3]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Convert into vector of optional references:
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::TryFromValues;
    /// use zrx_scheduler::values;
    ///
    /// // Create and convert values
    /// let values = values!(&1, &2, &3);
    /// let target = <Vec<Option<&i32>>>::try_from_values(values)?;
    /// assert_eq!(target, vec![Some(&1), Some(&2), Some(&3)]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from_values<V>(values: V) -> Result<Self>
    where
        V: IntoIterator<Item = Option<&'a dyn Value>>,
    {
        values.into_iter().map(T::try_from_value).collect()
    }
}

impl<'a> TryFromValues<'a> for () {
    /// Attempts to convert into the unit value.
    ///
    /// # Errors
    ///
    /// The following errors might occur:
    ///
    /// - [`Error::Mismatch`]: Number of values is not zero.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::value::TryFromValues;
    /// use zrx_scheduler::values;
    ///
    /// // Create and convert values
    /// let values = values!();
    /// let target = <()>::try_from_values(values)?;
    /// assert_eq!(target, ());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from_values<V>(values: V) -> Result<Self>
    where
        V: IntoIterator<Item = Option<&'a dyn Value>>,
    {
        match values.into_iter().next() {
            Some(_) => Err(Error::Mismatch),
            None => Ok(()),
        }
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements value conversion trait for a tuple.
macro_rules! impl_try_from_values_for_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<'a, $($T),+> TryFromValues<'a> for ($($T,)+)
        where
            $($T: TryFromValue<'a>,)+
        {
            #[inline]
            fn try_from_values<V>(values: V) -> Result<Self>
            where
                V: IntoIterator<Item = Option<&'a dyn Value>>,
            {
                let mut iter = values.into_iter();
                $(
                    #[allow(non_snake_case)]
                    let $T = $T::try_from_value(iter.next()
                        .ok_or(Error::Mismatch)?)?;
                )+

                // Ensure that the iterator yields no more values
                if iter.next().is_none() {
                    Ok(($($T,)+))
                } else {
                    Err(Error::Mismatch)
                }
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_try_from_values_for_tuple!(T1);
impl_try_from_values_for_tuple!(T1, T2);
impl_try_from_values_for_tuple!(T1, T2, T3);
impl_try_from_values_for_tuple!(T1, T2, T3, T4);
impl_try_from_values_for_tuple!(T1, T2, T3, T4, T5);
impl_try_from_values_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_try_from_values_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_try_from_values_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
