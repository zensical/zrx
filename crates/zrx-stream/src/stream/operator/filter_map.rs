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

//! Optional same-key transformations.

use std::marker::PhantomData;

use zrx_scheduler::Value;
use zrx_scheduler::action::{Action, Context};

use crate::stream::function::{Arguments, MapFn, Scope as CallbackScope};
use crate::stream::{Change, Id, Key, Stream};

use super::{IntoReplication, subscribe_function};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct FilterMap<T, F, A, U> {
    function: F,
    marker: PhantomData<fn(T, A) -> U>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Applies a same-key transformation that may omit its output.
    ///
    /// A raw cloneable Rust callback permits adaptive concurrency. Use
    /// [`super::sequential`] for non-overlapping invocation or
    /// [`super::concurrent`] to impose a concurrency bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::function::with_value;
    /// use zrx_stream::{run, Change};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     scope
    ///         .iter([(1, 2), (2, 3), (3, 4)])
    ///         .filter_map(with_value(|value: &u64| {
    ///             value.is_multiple_of(2).then_some(*value * 10)
    ///         }))
    /// })?
    /// .collect();
    ///
    /// let values: Vec<_> = changes?
    ///     .into_iter()
    ///     .filter_map(|change| match change {
    ///         Change::Insert(_, value) => Some(value),
    ///         Change::Remove(_) => None,
    ///     })
    ///     .collect();
    /// assert_eq!(values, [20_u64, 40]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if construction has already ended or operator construction
    /// reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn filter_map<F, A, U>(&self, f: F) -> Stream<I, U>
    where
        F: IntoReplication,
        F::Target: MapFn<A, I, T, Option<U>>,
        A: Arguments,
        U: Value,
    {
        subscribe_function(
            self,
            f,
            |function| FilterMap { function, marker: PhantomData },
            |action: &FilterMap<T, F::Target, A, U>| &action.function,
        )
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A, U> Action<Key<I>> for FilterMap<T, F, A, U>
where
    I: Id,
    T: Value,
    F: MapFn<A, I, T, Option<U>>,
    A: Arguments,
    U: Value,
{
    type Inputs = (T,);
    type Output = U;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "filter_map", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context { inputs: input, output, .. } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    let result = {
                        let mut scope = CallbackScope::new(&key, emit);
                        self.function.execute(&mut scope, value.as_ref())
                    };
                    let value = match result {
                        Ok(value) => value,
                        Err(error) => {
                            emit.reject(key, error);
                            return Ok(());
                        }
                    };
                    emit.resolve(key.clone());
                    match value {
                        Some(value) => emit.insert(key, value),
                        None => emit.remove(key),
                    }
                }
                Change::Remove(key) => {
                    emit.resolve(key.clone());
                    emit.remove(key);
                }
            }
            Ok(())
        });
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use zrx_executor::strategy::Immediate;

    use crate::stream::{Change, Key, Workflow};

    #[test]
    fn failed_replacement_preserves_the_last_optional_projection() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<i64>();
            workflow.output(&input.filter_map(|value: &i64| {
                anyhow::ensure!(*value >= 0, "invalid value");
                Ok::<_, anyhow::Error>(
                    (value.rem_euclid(2) == 0).then_some(*value * 10),
                )
            }));
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let input = runner.input::<i64>().unwrap();
        let key = Key::from(7_u64);
        let mut initial = input.begin().unwrap();
        initial.insert(key.clone(), 2).unwrap();
        let input = initial.seal().unwrap();
        let mut run = runner.settle().unwrap();
        let changes = run.output::<i64>().unwrap().collect::<Vec<_>>();
        assert!(matches!(
            changes.as_slice(),
            [Change::Insert(actual, 20)] if actual == &key
        ));

        let mut failing = input.begin().unwrap();
        failing.insert(key.clone(), -1).unwrap();
        let input = failing.seal().unwrap();
        let mut run = runner.settle().unwrap();
        assert!(run.output::<i64>().unwrap().next().is_none());
        assert_eq!(run.report().invocations()[0].outcomes.error_count(), 1);
        assert_eq!(runner.errors().len(), 1);
        assert_eq!(runner.errors()[0].key(), &key);

        let mut repair = input.begin().unwrap();
        repair.insert(key.clone(), 3).unwrap();
        let _input = repair.seal().unwrap();
        let mut run = runner.settle().unwrap();
        let changes = run.output::<i64>().unwrap().collect::<Vec<_>>();
        assert!(matches!(
            changes.as_slice(),
            [Change::Remove(actual)] if actual == &key
        ));
        assert!(run.report().invocations().is_empty());
        assert!(runner.errors().is_empty());
    }
}
