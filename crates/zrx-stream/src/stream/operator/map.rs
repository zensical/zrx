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

use zrx_scheduler::Value;
use zrx_scheduler::action::{Action, Context};

use crate::stream::function::{Arguments, MapFn, Scope as CallbackScope};
use crate::stream::{Change, Id, Key, Stream};

use super::{IntoReplication, subscribe_function};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Map<T, F, A, U> {
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
    /// Applies a same-key transformation.
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
    ///         .iter([(1, 20), (2, 21)])
    ///         .map(with_value(|value: &u64| *value * 2))
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
    /// assert_eq!(values, [40_u64, 42]);
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
    pub fn map<F, A, U>(&self, f: F) -> Stream<I, U>
    where
        F: IntoReplication,
        F::Target: MapFn<A, I, T, U>,
        A: Arguments,
        U: Value,
    {
        subscribe_function(
            self,
            f,
            |function| Map { function, marker: PhantomData },
            |action: &Map<T, F::Target, A, U>| &action.function,
        )
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A, U> Action<Key<I>> for Map<T, F, A, U>
where
    I: Id,
    T: Value,
    F: MapFn<A, I, T, U>,
    A: Arguments,
    U: Value,
{
    type Inputs = (T,);
    type Output = U;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "map", skip_all)
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
                    emit.insert(key, value);
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

    use crate::stream::function::with_value;
    use crate::stream::{Change, Key, Workflow};

    #[test]
    fn failure_preserves_output_and_repairs_by_source_identity() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<u64>();
            workflow.output(&input.map(with_value(|value: &u64| {
                anyhow::ensure!(*value != 0, "invalid value");
                Ok::<_, anyhow::Error>(*value * 2)
            })));
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let mut input = runner.input::<u64>().unwrap();
        let key = Key::from(7_u64);

        let mut initial = input.begin().unwrap();
        initial.insert(key.clone(), 5).unwrap();
        input = initial.seal().unwrap();
        let mut run = runner.settle().unwrap();
        let changes = run.output::<u64>().unwrap().collect::<Vec<_>>();
        assert!(matches!(
            changes.as_slice(),
            [Change::Insert(actual, 10)] if actual == &key
        ));
        assert!(runner.errors().is_empty());

        let mut failing = input.begin().unwrap();
        failing.insert(key.clone(), 0).unwrap();
        input = failing.seal().unwrap();
        let mut run = runner.settle().unwrap();
        assert!(run.output::<u64>().unwrap().next().is_none());
        let failures: Vec<_> = run
            .report()
            .invocations()
            .iter()
            .flat_map(|invocation| invocation.outcomes.failures())
            .collect();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].to_string(), "invalid value");
        assert_eq!(runner.errors().len(), 1);
        assert_eq!(runner.errors()[0].key(), &key);
        assert_eq!(runner.errors()[0].error().to_string(), "invalid value");

        let mut unrelated = input.begin().unwrap();
        unrelated.insert(Key::from(8_u64), 3).unwrap();
        input = unrelated.seal().unwrap();
        let mut run = runner.settle().unwrap();
        let changes = run.output::<u64>().unwrap().collect::<Vec<_>>();
        assert!(matches!(
            changes.as_slice(),
            [Change::Insert(actual, 6)]
                if actual == &Key::from(8_u64)
        ));
        assert!(run.report().invocations().is_empty());
        assert_eq!(runner.errors().len(), 1);
        assert_eq!(runner.errors()[0].key(), &key);

        let mut repair = input.begin().unwrap();
        repair.insert(key.clone(), 6).unwrap();
        let _input = repair.seal().unwrap();
        let mut run = runner.settle().unwrap();
        let changes = run.output::<u64>().unwrap().collect::<Vec<_>>();
        assert!(matches!(
            changes.as_slice(),
            [Change::Insert(actual, 12)] if actual == &key
        ));
        assert!(run.report().invocations().is_empty());
        assert!(runner.errors().is_empty());
    }

    #[test]
    fn panic_is_reported_and_source_removal_is_ordinary() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<u64>();
            workflow.output(&input.map(with_value(|value: &u64| {
                assert_ne!(*value, 0, "invalid value");
                *value
            })));
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let input = runner.input::<u64>().unwrap();
        let key = Key::from(7_u64);
        let mut failing = input.begin().unwrap();
        failing.insert(key.clone(), 0).unwrap();
        let input = failing.seal().unwrap();
        let run = runner.settle().unwrap();
        let failure = run
            .report()
            .invocations()
            .iter()
            .flat_map(|invocation| invocation.outcomes.failures())
            .next()
            .expect("panic reports one historical error");
        let failure = failure.to_string();
        assert!(failure.starts_with("caught panic: "));
        assert_eq!(runner.errors().len(), 1);
        assert_eq!(runner.errors()[0].error().to_string(), failure);

        let mut removal = input.begin().unwrap();
        removal.remove(key.clone()).unwrap();
        let _input = removal.seal().unwrap();
        let run = runner.settle().unwrap();
        assert!(run.report().invocations().is_empty());
        assert!(runner.errors().is_empty());
    }

    #[test]
    fn one_invocation_retains_distinct_historical_errors() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<i64>();
            workflow.output(&input.map(|value: &i64| -> anyhow::Result<i64> {
                anyhow::bail!("invalid {value}")
            }));
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let input = runner.input::<i64>().unwrap();
        let mut revision = input.begin().unwrap();
        revision.insert(Key::from(1_u64), -1).unwrap();
        revision.insert(Key::from(2_u64), -2).unwrap();
        let _input = revision.seal().unwrap();
        let run = runner.settle().unwrap();
        let errors: Vec<_> = run
            .report()
            .invocations()
            .iter()
            .flat_map(|invocation| invocation.outcomes.failures())
            .map(ToString::to_string)
            .collect();
        assert_eq!(errors, ["invalid -1", "invalid -2"]);
        assert_eq!(runner.errors().len(), 2);
    }
}
