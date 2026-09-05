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

//! Fuzzy dynamic stream barriers.

use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;

use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::error::catch;
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_scheduler::{RevisionId, Value};

use crate::stream::{Change, Id, Key, Stream};

use super::{Operator, Terminal, Tickets};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Selection<I, M>
where
    I: Id,
{
    matcher: M,
    required: BTreeSet<Key<I>>,
    /// Discovered keys whose match decision is not currently known.
    unresolved: BTreeSet<Key<I>>,
}

// ----------------------------------------------------------------------------

struct Barrier<I, D, T, N, F, M>
where
    I: Id,
{
    function: F,
    discovered: BTreeSet<Key<I>>,
    completed: BTreeMap<Key<I>, T>,
    selections: BTreeMap<Key<I>, Selection<I, M>>,
    terminal: Terminal<Key<I>>,
    published: BTreeSet<Key<I>>,
    affected: Vec<Key<I>>,
    marker: PhantomData<fn(D, N)>,
}

// ----------------------------------------------------------------------------

struct SelectionEvaluation;

// ----------------------------------------------------------------------------

struct PairEvaluation;

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, D> Stream<I, D>
where
    I: Id,
    D: Value,
{
    /// Waits for every discovered matching key to produce a completed value.
    ///
    /// The configuration callback builds an arbitrary fuzzy matcher. This
    /// stream supplies the discovered key relation, while `completed` supplies
    /// the values that fulfill matching members. The barrier publishes under
    /// each configuration key when every currently discovered match is
    /// fulfilled, and retracts if a member becomes pending again.
    ///
    /// A later matching discovery may reopen a published barrier. Publication
    /// occurs only at the relevant revision terminal, which proves that no
    /// further member can be discovered in that revision.
    /// Independently admitted input revisions remain independently visible;
    /// the barrier does not make separate root revisions atomic.
    ///
    /// # Panics
    ///
    /// Panics if the streams belong to different workflows, construction has
    /// already ended, or operator construction reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn barrier<T, N, F, M>(
        &self, completed: &Stream<I, T>, configuration: &Stream<I, N>,
        function: F,
    ) -> Stream<I, Vec<(Key<I>, T)>>
    where
        T: Value,
        N: Value,
        F: Fn(&N) -> M + Send + 'static,
        M: Fn(&Key<I>) -> bool + Send + 'static,
    {
        (self.clone(), completed.clone(), configuration.clone())
            .subscribe_progress(Barrier {
                function,
                discovered: BTreeSet::new(),
                completed: BTreeMap::new(),
                selections: BTreeMap::new(),
                terminal: Terminal::new(),
                published: BTreeSet::new(),
                affected: Vec::new(),
                marker: PhantomData,
            })
    }
}

// ----------------------------------------------------------------------------

impl<I, D, T, N, F, M> Barrier<I, D, T, N, F, M>
where
    I: Id,
    T: Value,
    F: Fn(&N) -> M + Send + 'static,
    M: Fn(&Key<I>) -> bool + Send + 'static,
{
    fn insert_discovered(
        &mut self, revision: RevisionId, key: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, Vec<(Key<I>, T)>>,
    ) {
        self.discovered.insert(key.clone());
        for (selection, state) in &mut self.selections {
            let included = match catch(|| Ok((state.matcher)(key))) {
                Ok(included) => {
                    state.unresolved.remove(key);
                    included
                }
                Err(error) => {
                    emit.reject_at::<PairEvaluation>(
                        selection.concat(key),
                        error,
                    );
                    state.unresolved.insert(key.clone());
                    self.terminal.mark(revision, selection.clone());
                    continue;
                }
            };
            emit.resolve_at::<PairEvaluation>(selection.concat(key));
            if included {
                state.required.insert(key.clone());
            } else {
                state.required.remove(key);
            }
            self.terminal.mark(revision, selection.clone());
        }
    }

    fn remove_discovered(
        &mut self, revision: RevisionId, key: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, Vec<(Key<I>, T)>>,
    ) {
        if !self.discovered.remove(key) {
            return;
        }
        self.affected.clear();
        self.affected.extend(self.selections.iter_mut().filter_map(
            |(selection, state)| {
                emit.resolve_at::<PairEvaluation>(selection.concat(key));
                let required = state.required.remove(key);
                let unresolved = state.unresolved.remove(key);
                (required || unresolved).then(|| selection.clone())
            },
        ));
        while let Some(selection) = self.affected.pop() {
            self.terminal.mark(revision, selection);
        }
    }

    fn insert_completed(
        &mut self, revision: RevisionId, key: &Key<I>, value: T,
    ) {
        self.completed.insert(key.clone(), value);
        self.mark_required(revision, key);
    }

    fn remove_completed(&mut self, revision: RevisionId, key: &Key<I>) {
        if self.completed.remove(key).is_some() {
            self.mark_required(revision, key);
        }
    }

    fn mark_required(&mut self, revision: RevisionId, key: &Key<I>) {
        self.affected.clear();
        self.affected.extend(
            self.selections
                .iter()
                .filter(|(_, state)| state.required.contains(key))
                .map(|(selection, _)| selection.clone()),
        );
        while let Some(selection) = self.affected.pop() {
            self.terminal.mark(revision, selection);
        }
    }

    fn insert_selection(
        &mut self, revision: RevisionId, key: Key<I>, value: &N,
        emit: &mut Emitter<'_, Key<I>, Vec<(Key<I>, T)>>,
    ) {
        let matcher = match catch(|| Ok((self.function)(value))) {
            Ok(matcher) => matcher,
            Err(error) => {
                emit.reject_at::<SelectionEvaluation>(key, error);
                return;
            }
        };
        let mut required = BTreeSet::new();
        for candidate in &self.discovered {
            match catch(|| Ok(matcher(candidate))) {
                Ok(true) => {
                    required.insert(candidate.clone());
                }
                Ok(false) => {}
                Err(error) => {
                    emit.reject_at::<SelectionEvaluation>(key, error);
                    return;
                }
            }
        }
        emit.resolve_at::<SelectionEvaluation>(key.clone());
        for candidate in &self.discovered {
            emit.resolve_at::<PairEvaluation>(key.concat(candidate));
        }
        self.selections.insert(
            key.clone(),
            Selection {
                matcher,
                required,
                unresolved: BTreeSet::new(),
            },
        );
        self.terminal.mark(revision, key);
    }

    fn remove_selection(
        &mut self, revision: RevisionId, key: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, Vec<(Key<I>, T)>>,
    ) {
        emit.resolve_at::<SelectionEvaluation>(key.clone());
        for candidate in &self.discovered {
            emit.resolve_at::<PairEvaluation>(key.concat(candidate));
        }
        if self.selections.remove(key).is_some() {
            self.terminal.mark(revision, key.clone());
        }
    }

    fn flush(
        &mut self, selections: Tickets<Key<I>>,
        emit: &mut Emitter<'_, Key<I>, Vec<(Key<I>, T)>>,
    ) {
        for ticket in selections {
            let key = self.terminal.applied(ticket);
            let Some(selection) = self.selections.get(&key) else {
                if self.published.remove(&key) {
                    emit.remove(key);
                }
                continue;
            };
            if !selection.unresolved.is_empty() {
                continue;
            }
            let ready = selection
                .required
                .iter()
                .all(|candidate| self.completed.contains_key(candidate));
            if ready {
                let values = selection
                    .required
                    .iter()
                    .map(|candidate| {
                        (
                            candidate.clone(),
                            self.completed
                                .get(candidate)
                                .expect("fulfilled barrier member disappeared")
                                .clone(),
                        )
                    })
                    .collect();
                self.published.insert(key.clone());
                emit.insert(key, values);
            } else if self.published.remove(&key) {
                emit.remove(key);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, D, T, N, F, M> Action<Key<I>> for Barrier<I, D, T, N, F, M>
where
    I: Id,
    D: Value,
    T: Value,
    N: Value,
    F: Fn(&N) -> M + Send + 'static,
    M: Fn(&Key<I>) -> bool + Send + 'static,
{
    type Inputs = (D, T, N);
    type Output = Vec<(Key<I>, T)>;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "barrier", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context {
            revision,
            inputs: (discovered, completed, configurations),
            output,
            events,
        } = context;
        discovered.for_each(output, |change, emit| match change {
            Change::Insert(key, _) => {
                self.insert_discovered(revision, &key, emit);
                Ok(())
            }
            Change::Remove(key) => {
                self.remove_discovered(revision, &key, emit);
                Ok(())
            }
        });
        completed.for_each(output, |change, _| {
            match change {
                Change::Insert(key, value) => {
                    self.insert_completed(revision, &key, value.into_owned());
                }
                Change::Remove(key) => {
                    self.remove_completed(revision, &key);
                }
            }
            Ok(())
        });
        configurations.for_each(output, |change, emit| match change {
            Change::Insert(key, value) => {
                self.insert_selection(revision, key, value.as_ref(), emit);
                Ok(())
            }
            Change::Remove(key) => {
                self.remove_selection(revision, &key, emit);
                Ok(())
            }
        });
        events.for_each(output, |event, emit| match event {
            Event::Progress(ProgressEvent::End) => {
                let selections = self.terminal.finish(revision);
                self.flush(selections, emit);
                Ok(())
            }
            Event::Progress(ProgressEvent::Abort) => {
                self.terminal.abort(revision);
                Ok(())
            }
            Event::Progress(ProgressEvent::Begin) => {
                self.terminal.begin(revision);
                Ok(())
            }
            Event::Wake { .. } => {
                unreachable!("progress-only operator received a wake")
            }
        });
    }
}
