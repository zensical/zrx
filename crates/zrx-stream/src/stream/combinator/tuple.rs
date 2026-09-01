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

//! Tuple stream composition.

use zrx_scheduler::Value;
use zrx_store::Value as StoreValue;

use crate::stream::Id;
use crate::stream::Stream;
use crate::stream::operator::{
    Anti, Coalesce, Full, Inner, Join, Left, Operator, Semi,
};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Join operations for tuples of streams.
pub trait StreamTupleExt<I>: Sized
where
    I: Id,
{
    /// Joined tuple value.
    type Output: Value;
    /// Left-joined tuple value.
    type LeftOutput: Value;
    /// Full-joined tuple value.
    type FullOutput: Value;
    /// First tuple value.
    type First: Value;

    /// Joins independently arriving values with equal keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::BTreeMap;
    ///
    /// use zrx_stream::{run, Change, StreamTupleExt};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let names = scope.iter([(1, "one"), (2, "two")]);
    ///     let values = scope.iter([(1, 10), (2, 20)]);
    ///     (names, values).join()
    /// })?
    /// .collect();
    ///
    /// let values: BTreeMap<_, _> = changes?
    ///     .into_iter()
    ///     .filter_map(|change| match change {
    ///         Change::Insert(key, value) => Some((key, value)),
    ///         Change::Remove(_) => None,
    ///     })
    ///     .collect();
    /// assert_eq!(values.values().copied().collect::<Vec<_>>(), [
    ///     ("one", 10),
    ///     ("two", 20),
    /// ]);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn join(self) -> Stream<I, Self::Output>;

    /// Left joins independently arriving values with equal keys.
    ///
    /// ```
    /// use zrx_stream::{run, StreamTupleExt};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let pages = scope.iter([(1, "page")]);
    ///     let titles = scope.iter([(1, "title")]);
    ///     (pages, titles).left_join()
    /// })?
    /// .collect();
    /// assert!(!changes?.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn left_join(self) -> Stream<I, Self::LeftOutput>;

    /// Full joins independently arriving values with equal keys.
    ///
    /// ```
    /// use zrx_stream::{run, StreamTupleExt};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let pages = scope.iter([(1, "page")]);
    ///     let titles = scope.iter([(2, "title")]);
    ///     (pages, titles).full_join()
    /// })?
    /// .collect();
    /// assert_eq!(changes?.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn full_join(self) -> Stream<I, Self::FullOutput>;

    /// Retains first-lane values with matches in every remaining lane.
    ///
    /// ```
    /// use zrx_stream::{run, StreamTupleExt};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let pages = scope.iter([(1, "page")]);
    ///     let published = scope.iter([(1, ())]);
    ///     (pages, published).semi_join()
    /// })?
    /// .collect();
    /// assert_eq!(changes?.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn semi_join(self) -> Stream<I, Self::First>;

    /// Retains first-lane values without matches in any remaining lane.
    ///
    /// ```
    /// use zrx_stream::{run, StreamTupleExt};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let pages = scope.iter([(1, "page")]);
    ///     let excluded = scope.iter([(2, ())]);
    ///     (pages, excluded).anti_join()
    /// })?
    /// .collect();
    /// assert_eq!(changes?.len(), 1);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn anti_join(self) -> Stream<I, Self::First>;
}

// ----------------------------------------------------------------------------

/// Priority operations for homogeneous tuples of streams.
pub trait StreamSetExt<I, T>: Sized
where
    I: Id,
    T: Value,
{
    /// Selects the first present value for each key in tuple order.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::{run, StreamSetExt};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     let preferred = scope.iter([(1, "preferred")]);
    ///     let fallback = scope.iter([(1, "fallback"), (2, "fallback")]);
    ///     (preferred, fallback).coalesce()
    /// })?
    /// .collect();
    /// assert_eq!(changes?.len(), 2);
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    fn coalesce(self) -> Stream<I, T>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

macro_rules! option {
    ($T:ident, $V:ident) => {
        Option<$V>
    };
}

macro_rules! stream {
    ($T:ident, $I:ident, $V:ident) => {
        Stream<$I, $V>
    };
}

macro_rules! impl_stream_tuple_ext {
    ($T1:ident $(, $T:ident)+ $(,)?) => {
        impl<I, $T1, $($T,)+> StreamTupleExt<I>
            for (Stream<I, $T1>, $(Stream<I, $T>,)+)
        where
            I: Id,
            $T1: StoreValue + Value,
            $($T: StoreValue + Value,)+
        {
            type Output = ($T1, $($T,)+);
            type LeftOutput = ($T1, $(Option<$T>,)+);
            type FullOutput = (Option<$T1>, $(Option<$T>,)+);
            type First = $T1;

            #[inline]
            fn join(self) -> Stream<I, Self::Output> {
                self.subscribe_progress(
                    Join::<
                        I,
                        ($T1, $($T,)+),
                        (Option<$T1>, $(Option<$T>,)+),
                        Inner,
                    >::new(),
                )
            }

            #[inline]
            fn left_join(self) -> Stream<I, Self::LeftOutput> {
                self.subscribe_progress(
                    Join::<
                        I,
                        ($T1, $(Option<$T>,)+),
                        (Option<$T1>, $(Option<$T>,)+),
                        Left,
                    >::new(),
                )
            }

            #[inline]
            fn full_join(self) -> Stream<I, Self::FullOutput> {
                self.subscribe_progress(
                    Join::<
                        I,
                        (Option<$T1>, $(Option<$T>,)+),
                        (Option<$T1>, $(Option<$T>,)+),
                        Full,
                    >::new(),
                )
            }

            #[inline]
            fn semi_join(self) -> Stream<I, Self::First> {
                self.subscribe_progress(
                    Join::<
                        I,
                        $T1,
                        (Option<$T1>, $(Option<$T>,)+),
                        Semi,
                    >::new(),
                )
            }

            #[inline]
            fn anti_join(self) -> Stream<I, Self::First> {
                self.subscribe_progress(
                    Join::<
                        I,
                        $T1,
                        (Option<$T1>, $(Option<$T>,)+),
                        Anti,
                    >::new(),
                )
            }
        }
    };
}

impl_stream_tuple_ext!(T1, T2);
impl_stream_tuple_ext!(T1, T2, T3);
impl_stream_tuple_ext!(T1, T2, T3, T4);
impl_stream_tuple_ext!(T1, T2, T3, T4, T5);
impl_stream_tuple_ext!(T1, T2, T3, T4, T5, T6);
impl_stream_tuple_ext!(T1, T2, T3, T4, T5, T6, T7);
impl_stream_tuple_ext!(T1, T2, T3, T4, T5, T6, T7, T8);

// ----------------------------------------------------------------------------

macro_rules! impl_stream_set_ext {
    ($T1:ident $(, $T:ident)+ $(,)?) => {
        impl<I, V> StreamSetExt<I, V>
            for (Stream<I, V>, $(stream!($T, I, V),)+)
        where
            I: Id,
            V: StoreValue + Value,
        {
            #[inline]
            fn coalesce(self) -> Stream<I, V> {
                self.subscribe_progress(Coalesce::<
                    I,
                    V,
                    (Option<V>, $(option!($T, V),)+),
                >::new())
            }
        }
    };
}

impl_stream_set_ext!(T1, T2);
impl_stream_set_ext!(T1, T2, T3);
impl_stream_set_ext!(T1, T2, T3, T4);
impl_stream_set_ext!(T1, T2, T3, T4, T5);
impl_stream_set_ext!(T1, T2, T3, T4, T5, T6);
impl_stream_set_ext!(T1, T2, T3, T4, T5, T6, T7);
impl_stream_set_ext!(T1, T2, T3, T4, T5, T6, T7, T8);
