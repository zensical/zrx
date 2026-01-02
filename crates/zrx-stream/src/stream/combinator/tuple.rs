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

//! Stream tuple.

use crate::stream::workspace::Workflow;
use crate::stream::Stream;

pub mod cons;
mod convert;
mod ext;
pub mod join;

pub use cons::StreamTupleCons;
pub use convert::IntoStreamTuple;
pub use ext::StreamTupleExt;
pub use join::StreamTupleJoin;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Stream tuple.
///
/// Stream tuples are heterogeneous collections of streams, which are essential
/// to differentially source inputs for functions that take multiple arguments.
/// They are implemented as tuples of streams with the help of macros in sizes
/// of 1 to 8, which also applies to all derived traits.
///
/// Operators implemented with stream tuples include:
///
/// - [`Stream::join`] + variations
/// - [`Stream::left_join`] + variations
/// - [`Stream::full_join`] + variations
///
/// As such, [`StreamTuple`] is solely a base trait with some methods attached
/// that allows to conveniently work with stream tuples. Trait derivations like
/// [`StreamTupleJoin`] and friends extend the functionality of [`StreamTuple`]
/// to implement join operations on tuples of streams and more.
pub trait StreamTuple<I>: Sized {
    fn workflow(&self) -> &Workflow<I>;
    fn ids(&self) -> Vec<usize>;
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements stream tuple trait with all items required.
macro_rules! impl_stream_tuple {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> StreamTuple<I>
            for ($(Stream<I, $T>,)+)
        {
            #[inline]
            fn workflow(&self) -> &Workflow<I> {
                &self.0.workflow
            }

            #[inline]
            fn ids(&self) -> Vec<usize> {
                #[allow(non_snake_case)]
                let ($($T,)+) = self;
                vec![$($T.id),+]
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_stream_tuple!(T1);
impl_stream_tuple!(T1, T2);
impl_stream_tuple!(T1, T2, T3);
impl_stream_tuple!(T1, T2, T3, T4);
impl_stream_tuple!(T1, T2, T3, T4, T5);
impl_stream_tuple!(T1, T2, T3, T4, T5, T6);
impl_stream_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_stream_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
