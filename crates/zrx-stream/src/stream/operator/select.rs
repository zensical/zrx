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

//! Differential dynamic selection and revision-complete snapshots.

use std::collections::{BTreeMap, BTreeSet};
use std::marker::PhantomData;

use ahash::HashMap;
use anyhow::anyhow;

use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::error::catch;
use zrx_scheduler::action::{Action, Context, Emitter};
use zrx_scheduler::{RevisionId, Value};

use crate::stream::Id;
use crate::stream::function::{Arguments, MapFn, Scope as CallbackScope};
use crate::stream::{Change, Key, Stream};

use super::{Operator, Terminal};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

trait Publication<I, T, M>: Send + 'static
where
    I: Id,
    T: Value,
{
    type Output: Value;

    fn validate(
        &self, selector: &Key<I>, candidate: &Key<I>, included: bool,
    ) -> zrx_scheduler::action::Result;

    fn upsert_member(
        &mut self, revision: RevisionId, selector: &Key<I>, candidate: &Key<I>,
        value: &T, emit: &mut Emitter<'_, Key<I>, Self::Output>,
    );

    fn remove_member(
        &mut self, revision: RevisionId, selector: &Key<I>, candidate: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, Self::Output>,
    );

    fn accept_selection(&mut self, revision: RevisionId, selector: &Key<I>);

    fn remove_selection(
        &mut self, revision: RevisionId, selector: &Key<I>, existed: bool,
    );

    fn begin(&mut self, revision: RevisionId);

    fn finish(
        &mut self, revision: RevisionId, _: &BTreeMap<Key<I>, T>,
        selections: &BTreeMap<Key<I>, Selection<I, M>>,
        emit: &mut Emitter<'_, Key<I>, Self::Output>,
    );

    fn abort(&mut self, revision: RevisionId);
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One selected candidate and the selector that owns its membership.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Membership<I, T>
where
    I: Id,
{
    selector: Key<I>,
    candidate: Key<I>,
    value: T,
}

// ----------------------------------------------------------------------------

struct Selection<I, M>
where
    I: Id,
{
    matcher: M,
    members: BTreeSet<Key<I>>,
    /// Candidates whose latest value has not been successfully evaluated.
    unresolved: BTreeSet<Key<I>>,
}

// ----------------------------------------------------------------------------

struct Select<I, T, N, F, M, A, P>
where
    I: Id,
{
    function: F,
    candidates: BTreeMap<Key<I>, T>,
    selections: BTreeMap<Key<I>, Selection<I, M>>,
    publication: P,
    scratch: Vec<Key<I>>,
    marker: PhantomData<fn(N, A)>,
}

// ----------------------------------------------------------------------------

struct SelectionEvaluation;

// ----------------------------------------------------------------------------

struct PairEvaluation;

// ----------------------------------------------------------------------------

struct MembershipPublication<I>
where
    I: Id,
{
    owners: HashMap<Key<I>, (Key<I>, Key<I>)>,
}

// ----------------------------------------------------------------------------

struct SnapshotPublication<I, T>
where
    I: Id,
    T: Value,
{
    members: BTreeMap<Key<I>, BTreeMap<Key<I>, T>>,
    terminal: Terminal<Key<I>>,
    published: BTreeSet<Key<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Membership<I, T>
where
    I: Id,
{
    /// Returns the selector that owns this membership.
    #[inline]
    pub const fn selector(&self) -> &Key<I> {
        &self.selector
    }

    /// Returns the selected candidate identity.
    #[inline]
    pub const fn candidate(&self) -> &Key<I> {
        &self.candidate
    }

    /// Returns the selected candidate value.
    #[inline]
    pub const fn value(&self) -> &T {
        &self.value
    }
}

// ----------------------------------------------------------------------------

impl<I, T> Stream<I, T>
where
    I: Id,
    T: Value,
{
    /// Selects a differential membership relation using dynamic records.
    ///
    /// Each output is keyed by the selector identity followed by the candidate
    /// identity. Matchers receive the candidate value by default and may use
    /// the standard function adapters when they need its key or identifier.
    /// Matching memberships are inserted and retracted immediately; consumers
    /// that need complete snapshots can use [`Self::select`].
    ///
    /// # Panics
    ///
    /// Panics if the streams belong to different workflows, construction has
    /// already ended, or operator construction reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn select_by<N, F, M, A>(
        &self, configuration: &Stream<I, N>, function: F,
    ) -> Stream<I, Membership<I, T>>
    where
        N: Value,
        F: Fn(&N) -> M + Send + 'static,
        M: MapFn<A, I, T, bool>,
        A: Arguments,
    {
        (self.clone(), configuration.clone()).subscribe(Select {
            function,
            candidates: BTreeMap::new(),
            selections: BTreeMap::new(),
            publication: MembershipPublication { owners: HashMap::default() },
            scratch: Vec::new(),
            marker: PhantomData,
        })
    }

    /// Selects coherent revision-complete snapshots using dynamic records.
    ///
    /// This shares the matching index of [`Self::select_by`] but uses a
    /// progress-aware publication policy. It publishes one current snapshot
    /// per affected configuration after all work derived from the corresponding
    /// source revision has reached this operator. Later revisions update or
    /// retract that snapshot at their terminal.
    ///
    /// # Panics
    ///
    /// Panics if the streams belong to different workflows, construction has
    /// already ended, or operator construction reenters the same workflow.
    #[inline]
    #[must_use]
    pub fn select<N, F, M, A>(
        &self, configuration: &Stream<I, N>, function: F,
    ) -> Stream<I, Vec<(Key<I>, T)>>
    where
        N: Value,
        F: Fn(&N) -> M + Send + 'static,
        M: MapFn<A, I, T, bool>,
        A: Arguments,
    {
        (self.clone(), configuration.clone()).subscribe_progress(Select {
            function,
            candidates: BTreeMap::new(),
            selections: BTreeMap::new(),
            publication: SnapshotPublication {
                members: BTreeMap::new(),
                terminal: Terminal::new(),
                published: BTreeSet::new(),
            },
            scratch: Vec::new(),
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------

impl<I, T, N, F, M, A, P> Select<I, T, N, F, M, A, P>
where
    I: Id,
    T: Value,
    F: Fn(&N) -> M + Send + 'static,
    M: MapFn<A, I, T, bool>,
    A: Arguments,
    P: Publication<I, T, M>,
{
    fn insert_candidate(
        &mut self, revision: RevisionId, key: &Key<I>, value: &T,
        emit: &mut Emitter<'_, Key<I>, P::Output>,
    ) {
        self.scratch.clear();
        self.scratch.extend(self.selections.keys().cloned());
        self.candidates.insert(key.clone(), value.clone());
        for index in 0..self.scratch.len() {
            let selector = self.scratch[index].clone();
            let selection = self
                .selections
                .get_mut(&selector)
                .expect("selected matcher disappeared");
            let mut scope = CallbackScope::new(key, emit);
            let included = match selection.matcher.execute(&mut scope, value) {
                Ok(included) => included,
                Err(error) => {
                    selection.unresolved.insert(key.clone());
                    emit.reject_at::<PairEvaluation>(
                        selector.concat(key),
                        error,
                    );
                    continue;
                }
            };
            if let Err(error) =
                self.publication.validate(&selector, key, included)
            {
                selection.unresolved.insert(key.clone());
                emit.reject_at::<PairEvaluation>(selector.concat(key), error);
                continue;
            }
            emit.resolve_at::<PairEvaluation>(selector.concat(key));
            selection.unresolved.remove(key);
            let previous = selection.members.contains(key);
            if included {
                selection.members.insert(key.clone());
                self.publication
                    .upsert_member(revision, &selector, key, value, emit);
            } else if previous {
                selection.members.remove(key);
                self.publication
                    .remove_member(revision, &selector, key, emit);
            }
        }
    }

    fn remove_candidate(
        &mut self, revision: RevisionId, key: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, P::Output>,
    ) {
        if self.candidates.remove(key).is_none() {
            return;
        }
        self.scratch.clear();
        for (selector, selection) in &mut self.selections {
            emit.resolve_at::<PairEvaluation>(selector.concat(key));
            selection.unresolved.remove(key);
            if selection.members.remove(key) {
                self.scratch.push(selector.clone());
            }
        }
        for index in 0..self.scratch.len() {
            let selector = self.scratch[index].clone();
            self.publication
                .remove_member(revision, &selector, key, emit);
        }
    }

    fn insert_selection(
        &mut self, revision: RevisionId, key: Key<I>, matcher: M,
        emit: &mut Emitter<'_, Key<I>, P::Output>,
    ) {
        let mut members = BTreeSet::new();
        for (candidate, value) in &self.candidates {
            let mut scope = CallbackScope::new(candidate, emit);
            let included = match matcher.execute(&mut scope, value) {
                Ok(included) => included,
                Err(error) => {
                    emit.reject_at::<SelectionEvaluation>(key, error);
                    return;
                }
            };
            if let Err(error) =
                self.publication.validate(&key, candidate, included)
            {
                emit.reject_at::<SelectionEvaluation>(key, error);
                return;
            }
            if included {
                members.insert(candidate.clone());
            }
        }
        emit.resolve_at::<SelectionEvaluation>(key.clone());
        for candidate in self.candidates.keys() {
            emit.resolve_at::<PairEvaluation>(key.concat(candidate));
        }
        let (previous, unresolved) = self
            .selections
            .remove(&key)
            .map(|selection| (selection.members, selection.unresolved))
            .unwrap_or_default();
        for candidate in previous.difference(&members) {
            self.publication
                .remove_member(revision, &key, candidate, emit);
        }
        // Successful replacement also accepts current values for retained
        // members whose prior evaluation failed. Unaffected members keep
        // their existing publication without emitting redundant changes.
        for candidate in members.iter().filter(|candidate| {
            !previous.contains(*candidate) || unresolved.contains(*candidate)
        }) {
            self.publication.upsert_member(
                revision,
                &key,
                candidate,
                self.candidates
                    .get(candidate)
                    .expect("evaluated candidate disappeared"),
                emit,
            );
        }
        self.publication.accept_selection(revision, &key);
        self.selections.insert(
            key,
            Selection {
                matcher,
                members,
                unresolved: BTreeSet::new(),
            },
        );
    }

    fn remove_selection(
        &mut self, revision: RevisionId, key: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, P::Output>,
    ) {
        emit.resolve_at::<SelectionEvaluation>(key.clone());
        for candidate in self.candidates.keys() {
            emit.resolve_at::<PairEvaluation>(key.concat(candidate));
        }
        let selection = self.selections.remove(key);
        if let Some(selection) = &selection {
            for candidate in &selection.members {
                self.publication
                    .remove_member(revision, key, candidate, emit);
            }
        }
        self.publication
            .remove_selection(revision, key, selection.is_some());
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Value for Membership<I, T>
where
    I: Id,
    T: Value,
{
}

// ----------------------------------------------------------------------------

impl<I, T, M> Publication<I, T, M> for MembershipPublication<I>
where
    I: Id,
    T: Value,
    M: Send + 'static,
{
    type Output = Membership<I, T>;

    fn validate(
        &self, selector: &Key<I>, candidate: &Key<I>, included: bool,
    ) -> zrx_scheduler::action::Result {
        if !included {
            return Ok(());
        }
        let key = selector.concat(candidate);
        if self.owners.get(&key).is_some_and(
            |(owner_selector, owner_candidate)| {
                owner_selector != selector || owner_candidate != candidate
            },
        ) {
            return Err(
                anyhow!("select_by output key has another owner").into()
            );
        }
        Ok(())
    }

    fn upsert_member(
        &mut self, _: RevisionId, selector: &Key<I>, candidate: &Key<I>,
        value: &T, emit: &mut Emitter<'_, Key<I>, Self::Output>,
    ) {
        let output = selector.concat(candidate);
        self.owners
            .insert(output.clone(), (selector.clone(), candidate.clone()));
        emit.insert(
            output,
            Membership {
                selector: selector.clone(),
                candidate: candidate.clone(),
                value: value.clone(),
            },
        );
    }

    fn remove_member(
        &mut self, _: RevisionId, selector: &Key<I>, candidate: &Key<I>,
        emit: &mut Emitter<'_, Key<I>, Self::Output>,
    ) {
        let output = selector.concat(candidate);
        self.owners.remove(&output);
        emit.remove(output);
    }

    fn accept_selection(&mut self, _: RevisionId, _: &Key<I>) {}

    fn remove_selection(&mut self, _: RevisionId, _: &Key<I>, _: bool) {}

    fn begin(&mut self, _: RevisionId) {}

    fn finish(
        &mut self, _: RevisionId, _: &BTreeMap<Key<I>, T>,
        _: &BTreeMap<Key<I>, Selection<I, M>>,
        _: &mut Emitter<'_, Key<I>, Self::Output>,
    ) {
    }

    fn abort(&mut self, _: RevisionId) {}
}

// ----------------------------------------------------------------------------

impl<I, T, M> Publication<I, T, M> for SnapshotPublication<I, T>
where
    I: Id,
    T: Value,
    M: Send + 'static,
{
    type Output = Vec<(Key<I>, T)>;

    fn validate(
        &self, _: &Key<I>, _: &Key<I>, _: bool,
    ) -> zrx_scheduler::action::Result {
        Ok(())
    }

    fn upsert_member(
        &mut self, revision: RevisionId, selector: &Key<I>, candidate: &Key<I>,
        value: &T, _: &mut Emitter<'_, Key<I>, Self::Output>,
    ) {
        self.members
            .entry(selector.clone())
            .or_default()
            .insert(candidate.clone(), value.clone());
        self.terminal.mark(revision, selector.clone());
    }

    fn remove_member(
        &mut self, revision: RevisionId, selector: &Key<I>, candidate: &Key<I>,
        _: &mut Emitter<'_, Key<I>, Self::Output>,
    ) {
        if let Some(members) = self.members.get_mut(selector) {
            members.remove(candidate);
        }
        self.terminal.mark(revision, selector.clone());
    }

    fn accept_selection(&mut self, revision: RevisionId, selector: &Key<I>) {
        self.members.entry(selector.clone()).or_default();
        self.terminal.mark(revision, selector.clone());
    }

    fn remove_selection(
        &mut self, revision: RevisionId, selector: &Key<I>, existed: bool,
    ) {
        if existed {
            self.members.remove(selector);
            self.terminal.mark(revision, selector.clone());
        }
    }

    fn begin(&mut self, revision: RevisionId) {
        self.terminal.begin(revision);
    }

    fn finish(
        &mut self, revision: RevisionId, _: &BTreeMap<Key<I>, T>,
        selections: &BTreeMap<Key<I>, Selection<I, M>>,
        emit: &mut Emitter<'_, Key<I>, Self::Output>,
    ) {
        for ticket in self.terminal.finish(revision) {
            let key = self.terminal.applied(ticket);
            if selections.contains_key(&key) {
                let values = self
                    .members
                    .get(&key)
                    .into_iter()
                    .flat_map(BTreeMap::iter)
                    .map(|(candidate, value)| {
                        (candidate.clone(), value.clone())
                    })
                    .collect();
                self.published.insert(key.clone());
                emit.insert(key, values);
            } else if self.published.remove(&key) {
                emit.remove(key);
            }
        }
    }

    fn abort(&mut self, revision: RevisionId) {
        self.terminal.abort(revision);
    }
}

// ----------------------------------------------------------------------------

impl<I, T, N, F, M, A, P> Action<Key<I>> for Select<I, T, N, F, M, A, P>
where
    I: Id,
    T: Value,
    N: Value,
    F: Fn(&N) -> M + Send + 'static,
    M: MapFn<A, I, T, bool>,
    A: Arguments,
    P: Publication<I, T, M>,
{
    type Inputs = (T, N);
    type Output = P::Output;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", name = "select", skip_all)
    )]
    fn execute(&mut self, context: Context<'_, Key<I>, Self>) {
        let Context {
            revision,
            inputs: (candidates, configurations),
            output,
            events,
        } = context;
        candidates.for_each(output, |change, emit| match change {
            Change::Insert(key, value) => {
                let value = value.into_owned();
                self.insert_candidate(revision, &key, &value, emit);
                Ok(())
            }
            Change::Remove(key) => {
                self.remove_candidate(revision, &key, emit);
                Ok(())
            }
        });
        configurations.for_each(output, |change, emit| match change {
            Change::Insert(key, value) => {
                let matcher =
                    match catch(|| Ok((self.function)(value.as_ref()))) {
                        Ok(matcher) => matcher,
                        Err(error) => {
                            emit.reject_at::<SelectionEvaluation>(key, error);
                            return Ok(());
                        }
                    };
                self.insert_selection(revision, key, matcher, emit);
                Ok(())
            }
            Change::Remove(key) => {
                self.remove_selection(revision, &key, emit);
                Ok(())
            }
        });
        events.for_each(output, |event, emit| match event {
            Event::Progress(ProgressEvent::End) => {
                self.publication.finish(
                    revision,
                    &self.candidates,
                    &self.selections,
                    emit,
                );
                Ok(())
            }
            Event::Progress(ProgressEvent::Abort) => {
                self.publication.abort(revision);
                Ok(())
            }
            Event::Progress(ProgressEvent::Begin) => {
                self.publication.begin(revision);
                Ok(())
            }
            Event::Wake { .. } => {
                unreachable!("progress-only operator received a wake")
            }
        });
    }
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use zrx_executor::strategy::Immediate;

    use crate::stream::{Change, Key, Runner, Workflow};

    type Snapshot = Vec<(Key<u64>, String)>;
    type SnapshotChange = (u64, Option<Snapshot>);

    fn snapshots(runner: &mut Runner<u64, Immediate>) -> Vec<SnapshotChange> {
        runner
            .settle()
            .unwrap()
            .output::<Vec<(Key<u64>, String)>>()
            .unwrap()
            .map(|change| match change {
                Change::Insert(key, value) => {
                    (*key.try_as_id().unwrap(), Some(value))
                }
                Change::Remove(key) => (*key.try_as_id().unwrap(), None),
            })
            .collect()
    }

    #[test]
    fn select_recomputes_membership_from_candidate_values() {
        let workflow = Workflow::<u64>::build(|workflow| {
            let candidates = workflow.input::<String>();
            let configurations = workflow.input::<String>();
            let selected =
                candidates.select(&configurations, |configuration: &String| {
                    let configuration = configuration.clone();
                    move |candidate: &String| candidate.contains(&configuration)
                });
            workflow.output(&selected);
        });
        let inputs: Vec<_> = workflow.inputs().copied().collect();
        let mut runner = workflow.runner_with(Immediate::new()).unwrap();
        let candidates = runner.input_at::<String>(inputs[0]).unwrap();
        let configurations = runner.input_at::<String>(inputs[1]).unwrap();

        let mut revision = configurations.begin().unwrap();
        revision
            .insert(Key::from(10), String::from("green"))
            .unwrap();
        let configurations = revision.seal().unwrap();
        assert_eq!(snapshots(&mut runner), [(10, Some(Vec::new()))],);

        let mut revision = candidates.begin().unwrap();
        revision
            .insert(Key::from(1), String::from("green page"))
            .unwrap();
        let mut candidates = revision.seal().unwrap();
        assert_eq!(
            snapshots(&mut runner),
            [(10, Some(vec![(Key::from(1), String::from("green page"))]),)],
        );

        let mut revision = candidates.begin().unwrap();
        revision
            .insert(Key::from(1), String::from("blue page"))
            .unwrap();
        candidates = revision.seal().unwrap();
        assert_eq!(snapshots(&mut runner), [(10, Some(Vec::new()))],);

        drop((candidates, configurations));
    }
}
