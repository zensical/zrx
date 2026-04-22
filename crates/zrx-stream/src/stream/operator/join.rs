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

//! Join operator.

use std::marker::PhantomData;

use zrx_scheduler::action::context::Binding;
use zrx_scheduler::action::{Action, Context};
use zrx_scheduler::step::IntoSteps;
use zrx_scheduler::{Id, Value};
use zrx_storage::accessor::Join as _;
use zrx_storage::borrow::IntoOwned;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Join operator.
#[derive(Debug)]
pub struct Join<T> {
    /// Capture types.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Join<T> {
    /// Creates a join operator.
    #[must_use]
    pub fn new() -> Self {
        Self { marker: PhantomData }
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements action trait.
macro_rules! impl_action {
    ($($T:ident),+ $(,)?) => {
        impl<I, $($T),+> Action<I> for Join<($($T),+)>
        where
            I: Id,
            $($T: Value,)+
        {
            type Inputs = ($($T,)+);
            type Output<'a> = ($($T,)+);

            /// Executes the operator.
            fn execute(
                &mut self, ctx: Context<I, Self>
            ) -> impl IntoSteps<I, Self> {
                let Binding { scopes, inputs, mut output, .. } = ctx.bind();
                scopes.into_iter().map(move |mut scope| {
                    match inputs.join(scope.key()) {
                        Some(value) => {
                            output.insert(scope.key().clone(), value.into_owned());
                        }
                        None => {
                            output.remove(scope.key());
                        }
                    }
                    scope.done()
                })
            }
        }
    }
}

// ----------------------------------------------------------------------------

impl_action!(T1, T2);
impl_action!(T1, T2, T3);
impl_action!(T1, T2, T3, T4);
impl_action!(T1, T2, T3, T4, T5);
impl_action!(T1, T2, T3, T4, T5, T6);
impl_action!(T1, T2, T3, T4, T5, T6, T7);
impl_action!(T1, T2, T3, T4, T5, T6, T7, T8);
