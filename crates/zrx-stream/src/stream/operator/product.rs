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

//! Product operator.

use std::marker::PhantomData;

use zrx_scheduler::action::context::Binding;
use zrx_scheduler::action::{Action, Context};
use zrx_scheduler::step::{IntoSteps, Scope};
use zrx_scheduler::{Id, Value};

use crate::stream::Stream;
use crate::stream::combinator::convert::IntoStreamTupleCons;

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Product operator.
pub struct Product<T, U> {
    /// Capture types.
    marker: PhantomData<(T, U)>,
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
    pub fn product<U>(&self, stream: &Stream<I, U>) -> Stream<I, (T, U)>
    where
        U: Value,
    {
        stream
            .into_stream_tuple_cons(self.clone())
            .subscribe(Product { marker: PhantomData })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, U> Action<I> for Product<T, U>
where
    I: Id,
    T: Value,
    U: Value,
{
    type Inputs = (T, U);
    type Output = (T, U);

    /// Executes the operator.
    fn execute(&mut self, ctx: Context<I, Self>) -> impl IntoSteps<I, Self> {
        let Binding { scopes, inputs, mut output, .. } = ctx.bind();
        scopes.into_iter().flat_map(move |scope| {
            let (left, right) = *inputs;
            let mut scoped = vec![];

            // If the key exists in the left scope,
            if let Some(l_value) = left.get(scope.key()) {
                for (r_scope, r_value) in right.iter() {
                    let combined = scope.key().concat(r_scope);
                    output.insert(
                        combined.clone(),
                        (l_value.clone(), r_value.clone()),
                    );
                    scoped.push(Scope::from(combined).done());
                }
            } else {
                for r_scope in right.keys() {
                    let combined = scope.key().concat(r_scope);
                    output.remove(&combined);
                    scoped.push(Scope::from(combined).done());
                }
            }

            // If the key exists in the left scope,
            if let Some(r_value) = right.get(scope.key()) {
                // omit double-emit
                for (l_scope, l_value) in
                    left.iter().filter(|(k, _)| *k != scope.key())
                {
                    let combined = scope.key().concat(l_scope);
                    output.insert(
                        combined.clone(),
                        (l_value.clone(), r_value.clone()),
                    );
                    scoped.push(Scope::from(combined).done());
                }
            } else {
                for l_scope in left.keys() {
                    let combined = l_scope.concat(scope.key());
                    output.remove(&combined);
                    scoped.push(Scope::from(combined).done());
                }
            }
            scoped
        })
    }
}
