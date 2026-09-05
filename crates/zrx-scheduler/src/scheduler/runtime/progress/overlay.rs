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

//! Shared key-free revision progress and partial graph convergence.

use ahash::HashMap;
use std::collections::VecDeque;

use crate::scheduler::RevisionId;
use crate::scheduler::action::control::ProgressEvent;
use crate::scheduler::plan::{
    InputIndex, Progress, ProgressIndex, ProgressNode,
};
use crate::scheduler::runtime::frame::ProgressFrame;

use super::{Obligation, Obligations};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct State {
    nodes: Box<[Option<ProgressNode>]>,
    sequence: u64,
}

/// Runtime owner of shared progress overlays.
pub struct Progresses {
    overlays: Vec<State>,
    by_input: Vec<Option<ProgressIndex>>,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct ProgressIdentity {
    revision: RevisionId,
    progress: ProgressIndex,
    sequence: u64,
}

// ----------------------------------------------------------------------------

struct Partial {
    frame: ProgressFrame,
    obligations: Obligations,
    lanes: u64,
    expected: usize,
}

// ----------------------------------------------------------------------------

/// Partial shared progress at graph convergence.
pub struct ProgressBranches {
    nodes: Vec<Branches>,
}

// ----------------------------------------------------------------------------

struct Branches {
    partials: HashMap<ProgressIdentity, Partial>,
    ready: VecDeque<ProgressIdentity>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Progresses {
    pub fn new(
        overlays: Vec<Progress>, by_input: Vec<Option<ProgressIndex>>,
    ) -> Self {
        Self {
            overlays: overlays
                .into_iter()
                .map(|progress| State {
                    nodes: progress.nodes,
                    sequence: 0,
                })
                .collect(),
            by_input,
        }
    }

    pub fn contains(&self, input: InputIndex) -> bool {
        self.by_input[input.get()].is_some()
    }

    pub fn node(&self, progress: ProgressIndex, node: usize) -> &ProgressNode {
        self.overlays[progress.get()].nodes[node]
            .as_ref()
            .expect("progress reached a node outside its compiled overlay")
    }

    pub fn boundary(
        &mut self, input: InputIndex, event: ProgressEvent,
    ) -> Option<ProgressFrame> {
        let progress = self.by_input[input.get()]?;
        Some(self.frame(progress, event))
    }

    fn frame(
        &mut self, progress: ProgressIndex, event: ProgressEvent,
    ) -> ProgressFrame {
        let sequence = &mut self.overlays[progress.get()].sequence;
        let current = *sequence;
        *sequence = sequence
            .checked_add(1)
            .expect("progress sequence overflowed");
        ProgressFrame::new(progress, current, event)
    }
}

// ----------------------------------------------------------------------------

impl ProgressIdentity {
    pub const fn new(
        revision: RevisionId, progress: ProgressIndex, sequence: u64,
    ) -> Self {
        Self { revision, progress, sequence }
    }

    pub const fn revision(self) -> RevisionId {
        self.revision
    }

    pub const fn progress(self) -> ProgressIndex {
        self.progress
    }
}

// ----------------------------------------------------------------------------

impl Partial {
    fn new(
        lane: usize, expected: usize, frame: ProgressFrame,
        obligation: Obligation,
    ) -> Self {
        let mut obligations = Obligations::for_revision(obligation.revision());
        obligations.push(obligation);
        Self {
            frame,
            obligations,
            lanes: lane_bit(lane),
            expected,
        }
    }

    fn insert(&mut self, lane: usize, obligation: Obligation) {
        let bit = lane_bit(lane);
        assert_eq!(self.lanes & bit, 0, "progress lane arrived twice");
        self.lanes |= bit;
        self.obligations.push(obligation);
    }

    fn is_complete(&self) -> bool {
        self.lanes.count_ones() as usize == self.expected
    }
}

// ----------------------------------------------------------------------------

impl ProgressBranches {
    pub fn new(nodes: usize) -> Self {
        Self {
            nodes: (0..nodes)
                .map(|_| Branches {
                    partials: HashMap::default(),
                    ready: VecDeque::new(),
                })
                .collect(),
        }
    }

    pub fn arrive(
        &mut self, node: usize, lane: usize, expected: usize,
        frame: ProgressFrame, obligation: Obligation,
    ) {
        let (progress, sequence) = frame.identity();
        let identity =
            ProgressIdentity::new(obligation.revision(), progress, sequence);
        let branches = &mut self.nodes[node];
        let complete = match branches.partials.entry(identity) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let partial = entry.get_mut();
                assert_eq!(partial.expected, expected);
                partial.frame.merge(frame);
                partial.insert(lane, obligation);
                partial.is_complete()
            }
            std::collections::hash_map::Entry::Vacant(entry) => entry
                .insert(Partial::new(lane, expected, frame, obligation))
                .is_complete(),
        };
        if complete {
            // Each contributing lane is FIFO, so a later frame cannot complete
            // before every earlier frame in the same overlay. Preserve that
            // completion frontier instead of recovering order from the map.
            branches.ready.push_back(identity);
        }
    }

    pub fn ready(&self, node: usize) -> Option<ProgressIdentity> {
        self.nodes[node].ready.front().copied()
    }

    pub fn is_end(&self, node: usize, identity: ProgressIdentity) -> bool {
        self.nodes[node].partials[&identity].frame.is_end()
    }

    pub fn take(
        &mut self, node: usize, identity: ProgressIdentity,
    ) -> (ProgressFrame, Obligations) {
        let branches = &mut self.nodes[node];
        assert_eq!(
            branches.ready.pop_front(),
            Some(identity),
            "ready progress convergence was taken out of order"
        );
        let partial = branches
            .partials
            .remove(&identity)
            .expect("ready progress convergence disappeared");
        assert!(partial.is_complete(), "incomplete progress was selected");
        (partial.frame, partial.obligations)
    }

    pub fn abort_revision_end(&mut self, revision: RevisionId) {
        for branches in &mut self.nodes {
            for (identity, partial) in &mut branches.partials {
                if identity.revision() == revision {
                    partial.frame.abort_end();
                }
            }
        }
    }

    pub fn prune(&mut self, revision: RevisionId) -> Obligations {
        self.prune_with(revision, true)
    }

    pub fn prune_all(&mut self, revision: RevisionId) -> Obligations {
        self.prune_with(revision, false)
    }

    fn prune_with(
        &mut self, revision: RevisionId, preserve_abort: bool,
    ) -> Obligations {
        let mut pruned = Obligations::for_revision(revision);
        for branches in &mut self.nodes {
            branches.partials.retain(|identity, partial| {
                if identity.revision() != revision
                    || (preserve_abort && partial.frame.is_abort())
                {
                    return true;
                }
                for obligation in partial.obligations.by_ref() {
                    pruned.push(obligation);
                }
                false
            });
            branches
                .ready
                .retain(|identity| branches.partials.contains_key(identity));
        }
        pruned
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn lane_bit(lane: usize) -> u64 {
    1_u64
        .checked_shl(u32::try_from(lane).expect("lane index fits in u32"))
        .expect("progress convergence exceeds 64 input lanes")
}

#[cfg(test)]
mod tests {
    use crate::scheduler::action::control::ProgressEvent;
    use crate::scheduler::plan::{InputIndex, ProgressIndex};
    use crate::scheduler::runtime::frame::ProgressFrame;
    use crate::scheduler::runtime::progress::Revisions;

    use super::{ProgressBranches, ProgressIdentity};

    #[test]
    fn completed_convergences_are_taken_in_completion_order() {
        let mut revisions = Revisions::new(3);
        let revision = revisions.begin(InputIndex::new(0));
        let mut obligations = revisions.admit_many(revision, 10).unwrap();
        let progress = ProgressIndex::new(0);
        let events = [
            ProgressEvent::Begin,
            ProgressEvent::End,
            ProgressEvent::Begin,
            ProgressEvent::End,
            ProgressEvent::Abort,
        ];
        let mut branches = ProgressBranches::new(1);

        for (sequence, event) in events.into_iter().enumerate() {
            let frame = ProgressFrame::new(progress, sequence as u64, event);
            branches.arrive(
                0,
                0,
                2,
                frame.clone(),
                obligations.next().unwrap(),
            );
            branches.arrive(0, 1, 2, frame, obligations.next().unwrap());
        }

        for sequence in 0..5 {
            let identity = ProgressIdentity::new(revision, progress, sequence);
            assert_eq!(branches.ready(0), Some(identity));
            let _ = branches.take(0, identity);
        }
        assert_eq!(branches.ready(0), None);
    }
}
