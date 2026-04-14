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

//! Map operator.

use std::marker::PhantomData;

use zrx_scheduler::action::context::Binding;
use zrx_scheduler::action::{Action, Context};
use zrx_scheduler::step::IntoSteps;
use zrx_scheduler::{Id, Value};

use crate::stream::function::{Arguments, MapFn};
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Map operator.
pub struct Map<T, F, A, U> {
    /// Operator function.
    function: F,
    /// Capture types.
    marker: PhantomData<(T, A, U)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Maps the stream using the provided function.
    #[inline]
    #[must_use]
    pub fn map<F, A, U>(&self, f: F) -> Stream<I, U>
    where
        F: MapFn<A, I, T, U> + Clone,
        A: Arguments,
        U: Value,
    {
        self.subscribe(Map {
            function: f,
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A, U> Action<I> for Map<T, F, A, U>
where
    I: Id,
    T: Value,
    F: MapFn<A, I, T, U> + Clone,
    A: Arguments,
    U: Value,
{
    type Inputs = (T,);
    type Output<'a> = U;

    /// Executes the operator.
    fn execute(&mut self, ctx: Context<I, Self>) -> impl IntoSteps<I, Self> {
        let Binding { scopes, inputs, mut output, .. } = ctx.bind();
        scopes.into_iter().map(move |scope| {
            let Some(value) = inputs.get(&scope).cloned() else {
                output.remove(&scope);
                return scope.done();
            };
            scope.task().build({
                let function = self.function.clone();
                move || {
                    let value = function.execute(&scope, value)?;
                    scope.then().build(move |mut ctx| {
                        let mut output = ctx.output().expect("invariant");
                        output.insert((*scope).clone(), value);
                        scope.done()
                    })
                }
            })
        })
    }
}
