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

//! One-to-many keyed transformations.

use std::marker::PhantomData;

use ahash::{HashMap, HashSet};
use anyhow::anyhow;

use zrx_scheduler::Value;
use zrx_scheduler::action::error::catch;
use zrx_scheduler::action::{Action, Context};

use crate::stream::Id;
use crate::stream::function::{Arguments, MapFn, Scope as CallbackScope};
use crate::stream::{Change, Key, Stream};

use super::Operator;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct FlatMap<I, T, F, A, J, U>
where
    I: Id,
{
    function: F,
    members: HashMap<Key<I>, HashSet<Key<I>>>,
    owners: HashMap<Key<I>, Key<I>>,
    marker: PhantomData<fn(T, A, J) -> U>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Replaces each input's complete source-qualified output set.
    ///
    /// The callback returns suffix keys and values. Each suffix is appended to
    /// the source key, making the source's derived set independently
    /// retractable. Duplicate or competing output keys are reported without
    /// changing the previous valid set.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::{run, Change, Key};
    ///
    /// # fn main() -> Result<(), zrx_stream::Error> {
    /// let changes: Result<Vec<_>, _> = run::<u64, _>(|scope| {
    ///     scope.iter([(1, vec![10_u64, 20])]).flat_map(
    ///         |values: &Vec<u64>| {
    ///             values
    ///                 .iter()
    ///                 .map(|value| (Key::from(*value), *value))
    ///                 .collect::<Vec<_>>()
    ///         },
    ///     )
    /// })?
    /// .collect();
    ///
    /// let mut keys: Vec<_> = changes?
    ///     .into_iter()
    ///     .filter_map(|change| match change {
    ///         Change::Insert(key, _) => Some(key),
    ///         Change::Remove(_) => None,
    ///     })
    ///     .collect();
    /// keys.sort();
    /// assert_eq!(
    ///     keys,
    ///     [
    ///         [1_u64, 10].into_iter().collect(),
    ///         [1_u64, 20].into_iter().collect(),
    ///     ]
    /// );
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
    pub fn flat_map<F, A, J, U>(&self, function: F) -> Stream<I, U>
    where
        F: MapFn<A, I, T, J>,
        A: Arguments,
        J: IntoIterator<Item = (Key<I>, U)> + Value,
        U: Value,
    {
        self.subscribe(FlatMap {
            function,
            members: HashMap::default(),
            owners: HashMap::default(),
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T, F, A, J, U> Action<Key<I>> for FlatMap<I, T, F, A, J, U>
where
    I: Id,
    T: Value,
    F: MapFn<A, I, T, J>,
    A: Arguments,
    J: IntoIterator<Item = (Key<I>, U)> + Value,
    U: Value,
{
    type Inputs = (T,);
    type Output = U;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "flat_map", skip_all)
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
                    let values = match result {
                        Ok(values) => values,
                        Err(error) => {
                            emit.reject(source, error);
                            return Ok(());
                        }
                    };
                    let result = catch(|| {
                        let mut next = HashMap::default();
                        for (suffix, value) in values {
                            let derived = source.concat(suffix);
                            if next.insert(derived, value).is_some() {
                                return Err(anyhow!(
                                    "flat_map produced a duplicate output key"
                                )
                                .into());
                            }
                        }
                        Ok(next)
                    });
                    let next = match result {
                        Ok(next) => next,
                        Err(error) => {
                            emit.reject(source, error);
                            return Ok(());
                        }
                    };
                    for derived in next.keys() {
                        if self
                            .owners
                            .get(derived)
                            .is_some_and(|owner| owner != &source)
                        {
                            emit.reject(
                                source,
                                anyhow!(
                                    "flat_map output key has another owner"
                                )
                                .into(),
                            );
                            return Ok(());
                        }
                    }

                    emit.resolve(source.clone());

                    let previous = self.members.get(&source).cloned();
                    if let Some(previous) = previous {
                        for derived in previous {
                            if !next.contains_key(&derived)
                                && self.owners.get(&derived) == Some(&source)
                            {
                                self.owners.remove(&derived);
                                emit.remove(derived);
                            }
                        }
                    }

                    let mut members = HashSet::default();
                    members.reserve(next.len());
                    for (derived, value) in next {
                        members.insert(derived.clone());
                        self.owners.insert(derived.clone(), source.clone());
                        emit.insert(derived, value);
                    }
                    if members.is_empty() {
                        self.members.remove(&source);
                    } else {
                        self.members.insert(source, members);
                    }
                }
                Change::Remove(source) => {
                    emit.resolve(source.clone());
                    let Some(members) = self.members.remove(&source) else {
                        return Ok(());
                    };
                    for derived in members {
                        if self.owners.get(&derived) == Some(&source) {
                            self.owners.remove(&derived);
                            emit.remove(derived);
                        }
                    }
                }
            }
            Ok(())
        });
    }
}
