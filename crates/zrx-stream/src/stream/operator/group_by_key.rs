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

//! Group-qualified key transformations.

use std::marker::PhantomData;

use ahash::HashMap;
use anyhow::anyhow;

use zrx_scheduler::Value;
use zrx_scheduler::action::{Action, Context};

use crate::stream::Id;
use crate::stream::function::{Arguments, MapFn, Scope as CallbackScope};
use crate::stream::{Change, Key, Stream};

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct GroupByKey<I, T, F, A>
where
    I: Id,
{
    function: F,
    assignments: HashMap<Key<I>, Key<I>>,
    owners: HashMap<Key<I>, Key<I>>,
    marker: PhantomData<fn(T, A)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Groups each input under a derived key while preserving its value.
    ///
    /// The output key is the derived group key followed by the source key. The
    /// operator remembers successful assignments so updates and removals
    /// retract the correct group-qualified key. A competing owner is reported
    /// without changing the previous valid output.
    ///
    /// # Panics
    ///
    /// Panics if construction has already ended or operator construction
    /// reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn group_by_key<F, A>(&self, function: F) -> Stream<I, T>
    where
        F: MapFn<A, I, T, Key<I>>,
        A: Arguments,
    {
        self.subscribe(GroupByKey {
            function,
            assignments: HashMap::default(),
            owners: HashMap::default(),
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A> Action<Key<I>> for GroupByKey<I, T, F, A>
where
    I: Id,
    T: Value,
    F: MapFn<A, I, T, Key<I>>,
    A: Arguments,
{
    type Inputs = (T,);
    type Output = T;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "group_by_key", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(source, value) => {
                    let result = {
                        let mut scope = CallbackScope::new(&source, emit);
                        self.function.execute(&mut scope, value.as_ref())
                    };
                    let group = match result {
                        Ok(group) => group,
                        Err(error) => {
                            emit.reject(source, error);
                            return Ok(());
                        }
                    };
                    let derived = group.concat(&source);
                    if self
                        .owners
                        .get(&derived)
                        .is_some_and(|owner| owner != &source)
                    {
                        emit.reject(
                            source,
                            anyhow!(
                                "group_by_key output key has another owner"
                            )
                            .into(),
                        );
                        return Ok(());
                    }

                    emit.resolve(source.clone());

                    let previous = self.assignments.get(&source).cloned();
                    if previous.as_ref() != Some(&group) {
                        if let Some(previous) = previous {
                            let previous = previous.concat(&source);
                            if self.owners.get(&previous) == Some(&source) {
                                self.owners.remove(&previous);
                                emit.remove(previous);
                            }
                        }
                        self.assignments.insert(source.clone(), group);
                        self.owners.insert(derived.clone(), source);
                    }
                    emit.insert(derived, value.into_owned());
                }
                Change::Remove(source) => {
                    emit.resolve(source.clone());
                    let Some(group) = self.assignments.remove(&source) else {
                        return Ok(());
                    };
                    let derived = group.concat(&source);
                    if self.owners.get(&derived) == Some(&source) {
                        self.owners.remove(&derived);
                        emit.remove(derived);
                    }
                }
            }
            Ok(())
        });
    }
}
