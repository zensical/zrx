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

//! Inspect operator.

use std::marker::PhantomData;

use zrx_scheduler::action::context::Binding;
use zrx_scheduler::action::{Action, Context};
use zrx_scheduler::step::IntoSteps;
use zrx_scheduler::{Id, Value};

use crate::stream::function::{Arguments, InspectFn};
use crate::stream::Stream;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Inspect operator.
pub struct Inspect<T, F, A> {
    /// Operator function.
    function: F,
    /// Capture types.
    marker: PhantomData<(T, A)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Inspects the stream using the provided function.
    #[inline]
    pub fn inspect<F, A>(&self, f: F) -> Stream<I, T>
    where
        F: InspectFn<A, I, T>,
        A: Arguments,
    {
        self.subscribe(Inspect {
            function: f,
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A> Action<I> for Inspect<T, F, A>
where
    I: Id,
    T: Value,
    F: InspectFn<A, I, T>,
    A: Arguments,
{
    type Inputs = (T,);
    type Output<'a> = T;

    /// Executes the operator.
    fn execute(&mut self, ctx: Context<I, Self>) -> impl IntoSteps<I, Self> {
        let Binding { scopes, inputs, mut output, .. } = ctx.bind();
        scopes.into_iter().map(move |mut scope| {
            let Some(value) = inputs.get(scope.key()).cloned() else {
                output.remove(scope.key());
                return scope.done();
            };
            self.function.execute(&mut scope, &value)?;
            output.insert(scope.key().clone(), value.clone());
            scope.done()
        })
    }
}
