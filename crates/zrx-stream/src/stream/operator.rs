// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// Third-party contributions licensed under DCO

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

//! Stream operators.

#![allow(clippy::must_use_candidate)] // check why we need this
#![allow(clippy::new_without_default)]
#![allow(clippy::return_self_not_must_use)]

use std::marker::PhantomData;

use zrx_scheduler::action::Action;
use zrx_scheduler::schedule::Subscriber;
use zrx_scheduler::Id;

use super::Stream;

mod filter;
mod filter_map;
mod inspect;
mod join;
mod map;
mod product;
mod select;

pub use filter::Filter;
pub use filter_map::FilterMap;
pub use inspect::Inspect;
pub use join::Join;
pub use map::Map;
pub use product::Product;
pub use select::Select;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Operator.
pub trait Operator<'a, I, A>
where
    A: Action<I>,
{
    /// Subscribe the given subscriber.
    fn subscribe<S>(&self, subscriber: S) -> Stream<I, A::Output<'a>>
    where
        S: Into<Subscriber<'a, I, A>>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I, A, T> Operator<'a, I, A> for Stream<I, T>
where
    I: Id,
    A: Action<I, Inputs = (T,)> + 'static,
{
    /// Subscribe the given subscriber.
    #[inline]
    fn subscribe<S>(&self, subscriber: S) -> Stream<I, A::Output<'a>>
    where
        S: Into<Subscriber<'a, I, A>>,
    {
        // We can safely use expect here, as the stream interface prevents us
        // from creating subscribers with invalid nodes. Otherwise, it's a bug.
        let id = self.workflow.with(|builder| {
            builder.add([self.id], subscriber).expect("invariant")
        });
        Stream {
            id,
            workflow: self.workflow.clone(),
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements operator trait for a tuple.
macro_rules! impl_operator_for_tuple {
    ($T1:ident $(, $T:ident)+ $(,)?) => {
        impl<'a, I, A, $T1, $($T,)+> Operator<'a, I, A>
            for (Stream<I, $T1>, $(Stream<I, $T>,)+)
        where
            I: Id,
            A: Action<I, Inputs = ($T1, $($T,)+)> + 'static,
        {
            #[inline]
            fn subscribe<S>(&self, subscriber: S) -> Stream<I, A::Output<'a>>
            where
                S: Into<Subscriber<'a, I, A>>,
            {
                #[allow(non_snake_case)]
                let ($T1, $($T,)*) = self;
                // Albeit this can practically never happen, we can technically
                // have different workflows here if we somehow alter the stream
                // interface. Thus, this assertion is just a cautionary measure
                // to prevent our future selves from breaking it by accident.
                $(assert_eq!($T1.workflow, $T.workflow);)+
                let id = $T1.workflow.with(|builder| {
                    builder
                        .add([$T1.id, $($T.id,)*], subscriber)
                        .expect("invariant")
                });
                Stream {
                    id,
                    workflow: $T1.workflow.clone(),
                    marker: PhantomData,
                }
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_operator_for_tuple!(T1, T2);
impl_operator_for_tuple!(T1, T2, T3);
impl_operator_for_tuple!(T1, T2, T3, T4);
impl_operator_for_tuple!(T1, T2, T3, T4, T5);
impl_operator_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_operator_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_operator_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);
