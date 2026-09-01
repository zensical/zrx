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

//! Unique secondary indexes.

use std::marker::PhantomData;

use ahash::HashMap;
use anyhow::anyhow;

use zrx_scheduler::Value;
use zrx_scheduler::action::{Action, Context, Emitter};

use crate::stream::Id;
use crate::stream::function::{Arguments, MapFn, Scope as CallbackScope};
use crate::stream::{Change, Key, Stream};

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct UniqueByKey<I, T, F, A>
where
    I: Id,
{
    function: F,
    assignments: HashMap<Key<I>, Key<I>>,
    claims: HashMap<Key<I>, HashMap<Key<I>, T>>,
    marker: PhantomData<fn(A)>,
}

// ----------------------------------------------------------------------------

struct Uniqueness;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Builds a unique secondary index while preserving input values.
    ///
    /// The operator retains every live source claim for each derived key. A
    /// key with one claim publishes that value, while a key with competing
    /// claims is absent until exactly one claimant remains.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::{run, Change, Key};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     scope
    ///         .iter([(1, 10_u32), (2, 20)])
    ///         .unique_by_key(|value: &u32| Key::from(u64::from(*value)))
    /// })?
    /// .collect();
    ///
    /// let keys: Vec<_> = changes?
    ///     .into_iter()
    ///     .filter_map(|change| match change {
    ///         Change::Insert(key, _) => Some(key),
    ///         Change::Remove(_) => None,
    ///     })
    ///     .collect();
    /// assert_eq!(keys, [Key::from(10_u64), Key::from(20_u64)]);
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
    pub fn unique_by_key<F, A>(&self, function: F) -> Stream<I, T>
    where
        F: MapFn<A, I, T, Key<I>>,
        A: Arguments,
    {
        self.subscribe(UniqueByKey {
            function,
            assignments: HashMap::default(),
            claims: HashMap::default(),
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------

impl<I, T, F, A> UniqueByKey<I, T, F, A>
where
    I: Id,
    T: Value,
{
    fn insert_claim(
        &mut self, derived: Key<I>, source: Key<I>, value: T,
        emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        let claims = self.claims.entry(derived.clone()).or_default();
        let previous = claims.len();
        debug_assert!(!claims.contains_key(&source));
        claims.insert(source, value.clone());
        match previous {
            0 => {
                emit.resolve_at::<Uniqueness>(derived.clone());
                emit.insert(derived, value);
            }
            1 => {
                emit.reject_at::<Uniqueness>(
                    derived.clone(),
                    anyhow!("unique_by_key derived key is not unique").into(),
                );
                emit.remove(derived);
            }
            _ => {}
        }
    }

    fn replace_claim(
        &mut self, derived: &Key<I>, source: Key<I>, value: T,
        emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        let claims = self
            .claims
            .get_mut(derived)
            .expect("accepted assignment must retain its claim set");
        let previous = claims.insert(source, value.clone());
        debug_assert!(previous.is_some());
        if claims.len() == 1 {
            emit.resolve_at::<Uniqueness>(derived.clone());
            emit.insert(derived.clone(), value);
        }
    }

    fn remove_claim(
        &mut self, derived: &Key<I>, source: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, T>,
    ) {
        let Some(claims) = self.claims.get_mut(derived) else {
            debug_assert!(false, "accepted assignment lost its claim set");
            return;
        };
        let previous = claims.len();
        let removed = claims.remove(source);
        debug_assert!(removed.is_some());
        match (previous, claims.len()) {
            (1, 0) => {
                emit.resolve_at::<Uniqueness>(derived.clone());
                emit.remove(derived.clone());
            }
            (2, 1) => {
                emit.resolve_at::<Uniqueness>(derived.clone());
                let value = claims
                    .values()
                    .next()
                    .expect("one remaining claim must retain a value")
                    .clone();
                emit.insert(derived.clone(), value);
            }
            _ => {}
        }
        if claims.is_empty() {
            self.claims.remove(derived);
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A> Action<Key<I>> for UniqueByKey<I, T, F, A>
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
        tracing::instrument(level = "debug", name = "unique_by_key", skip_all)
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
                    let derived = match result {
                        Ok(derived) => derived,
                        Err(error) => {
                            emit.reject(source, error);
                            return Ok(());
                        }
                    };
                    emit.resolve(source.clone());
                    let value = value.into_owned();
                    let previous = self.assignments.get(&source).cloned();
                    if previous.as_ref() == Some(&derived) {
                        self.replace_claim(&derived, source, value, emit);
                    } else {
                        if let Some(previous) = previous {
                            self.remove_claim(&previous, &source, emit);
                        }
                        self.assignments
                            .insert(source.clone(), derived.clone());
                        self.insert_claim(derived, source, value, emit);
                    }
                }
                Change::Remove(source) => {
                    emit.resolve(source.clone());
                    let Some(derived) = self.assignments.remove(&source) else {
                        return Ok(());
                    };
                    self.remove_claim(&derived, &source, emit);
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
    use crate::stream::{Key, Workflow};

    #[test]
    fn conflict_failure_is_sparse_and_historical() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let input = workflow.input::<u64>();
            workflow.output(&input.unique_by_key(with_value(|value: &u64| {
                Key::from(*value % 10)
            })));
        });
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let input = runner.input::<u64>().unwrap();
        let mut first = input.begin().unwrap();
        first.insert(Key::from(1_u64), 11).unwrap();
        let input = first.seal().unwrap();
        let _run = runner.settle().unwrap();

        let mut conflict = input.begin().unwrap();
        conflict.insert(Key::from(2_u64), 21).unwrap();
        let input = conflict.seal().unwrap();
        let run = runner.settle().unwrap();
        assert_eq!(run.report().invocations()[0].outcomes.error_count(), 1);
        assert_eq!(runner.errors().len(), 1);
        assert_eq!(runner.errors()[0].key(), &Key::from(1_u64));

        let mut unrelated = input.begin().unwrap();
        unrelated.insert(Key::from(3_u64), 32).unwrap();
        let input = unrelated.seal().unwrap();
        let run = runner.settle().unwrap();
        assert!(run.report().invocations().is_empty());
        assert_eq!(runner.errors().len(), 1);

        let mut repair = input.begin().unwrap();
        repair.remove(Key::from(2_u64)).unwrap();
        let _input = repair.seal().unwrap();
        let run = runner.settle().unwrap();
        assert!(run.report().invocations().is_empty());
        assert!(runner.errors().is_empty());
    }
}
