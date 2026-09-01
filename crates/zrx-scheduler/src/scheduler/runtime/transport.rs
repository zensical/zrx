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

//! Ordered segment and progress transport through one input lane.

use std::collections::VecDeque;
use std::iter;
use std::num::NonZeroUsize;

use zrx_graph::Graph;

use crate::scheduler::Id;
use crate::scheduler::RevisionId;
use crate::scheduler::action::Segment;
use crate::scheduler::plan::{Destination, NodePlan, OutputBinding, Route};

use super::frame::ProgressFrame;
use super::ordered::OrderedWindow;
use super::progress::{Obligation, Obligations, ProgressIdentity};

mod egress;

use egress::Egresses;
pub use egress::{Egress, EgressIter};

// Bootstrap buffering for internal lanes. Topology-derived atomic admission
// may raise this minimum; later occupancy feedback may tune the excess.
const BOOTSTRAP_ENTRY_CAPACITY: usize = 64;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

pub enum Entry<I>
where
    I: Id,
{
    Data(Data<I>),
    Progress {
        frame: ProgressFrame,
        obligation: Obligation,
    },
}

// ----------------------------------------------------------------------------

enum ReservationPositions {
    Empty,
    One(Option<DestinationReservation>),
    Many(std::vec::IntoIter<DestinationReservation>),
}

// ----------------------------------------------------------------------------

pub enum Credit {
    Lane(Route),
    Output(usize),
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

pub struct Data<I>
where
    I: Id,
{
    pub segment: Segment<I>,
    pub obligation: Obligation,
    pub quantum: Option<NonZeroUsize>,
}

// ----------------------------------------------------------------------------

pub struct Reservation {
    order: u64,
}

// ----------------------------------------------------------------------------

pub struct DestinationReservation {
    pub destination: Destination,
    pub position: Reservation,
}

// ----------------------------------------------------------------------------

pub struct OutputReservations {
    positions: ReservationPositions,
}

// ----------------------------------------------------------------------------

pub struct TransportUpdate {
    pub ready: Option<usize>,
    pub credit: Option<Credit>,
}

// ----------------------------------------------------------------------------

pub struct Pruned {
    pub released_lanes: Vec<Route>,
    pub released_outputs: Vec<usize>,
    pub obligations: Obligations,
}

// ----------------------------------------------------------------------------

/// One FIFO input lane with ordered completion-gap repair.
pub struct Lane<I>
where
    I: Id,
{
    entries: VecDeque<Entry<I>>,
    commits: OrderedWindow<Option<Entry<I>>>,
    capacity: usize,
    occupied: usize,
    issued: u64,
    // Latest action sequence dispatched from this lane. FIFO makes it the
    // reconciliation watermark for the progress frame at the lane front.
    predecessor: Option<u64>,
}

// ----------------------------------------------------------------------------

pub struct Transport<I>
where
    I: Id,
{
    lanes: Vec<Box<[Lane<I>]>>,
    egress: Egresses<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Data<I>
where
    I: Id,
{
    const fn new(segment: Segment<I>, obligation: Obligation) -> Self {
        Self {
            segment,
            obligation,
            quantum: None,
        }
    }

    pub const fn tail(
        segment: Segment<I>, obligation: Obligation, quantum: NonZeroUsize,
    ) -> Self {
        Self {
            segment,
            obligation,
            quantum: Some(quantum),
        }
    }
}

// ----------------------------------------------------------------------------

impl<I> Entry<I>
where
    I: Id,
{
    pub const fn data(segment: Segment<I>, obligation: Obligation) -> Self {
        Self::Data(Data::new(segment, obligation))
    }

    pub const fn progress(
        frame: ProgressFrame, obligation: Obligation,
    ) -> Self {
        Self::Progress { frame, obligation }
    }

    fn revision(&self) -> RevisionId {
        match self {
            Self::Data(Data { obligation, .. })
            | Self::Progress { obligation, .. } => obligation.revision(),
        }
    }

    fn into_obligation(self) -> Obligation {
        match self {
            Self::Data(Data { obligation, .. })
            | Self::Progress { obligation, .. } => obligation,
        }
    }

    fn is_abort(&self) -> bool {
        matches!(self, Self::Progress { frame, .. } if frame.is_abort())
    }
}

// ----------------------------------------------------------------------------

impl OutputReservations {
    pub(super) const fn empty() -> Self {
        Self {
            positions: ReservationPositions::Empty,
        }
    }

    const fn one(position: DestinationReservation) -> Self {
        Self {
            positions: ReservationPositions::One(Some(position)),
        }
    }

    fn many(positions: Vec<DestinationReservation>) -> Self {
        debug_assert!(positions.len() > 1);
        Self {
            positions: ReservationPositions::Many(positions.into_iter()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ----------------------------------------------------------------------------

impl Reservation {
    pub fn commit<I>(self, lane: &mut Lane<I>, entry: Option<Entry<I>>) -> usize
    where
        I: Id,
    {
        lane.commit(self.order, entry)
    }
}

// ----------------------------------------------------------------------------

impl<I> Lane<I>
where
    I: Id,
{
    pub fn new(capacity: usize) -> Self {
        assert!(capacity != 0, "lane capacity must be non-zero");
        Self {
            entries: VecDeque::new(),
            commits: OrderedWindow::new(capacity),
            capacity,
            occupied: 0,
            issued: 0,
            predecessor: None,
        }
    }

    pub const fn progress_predecessor(&self) -> Option<u64> {
        self.predecessor
    }

    pub fn record_dispatch(&mut self, sequence: u64) {
        assert!(
            self.predecessor.is_none_or(|current| sequence > current),
            "lane dispatch sequence did not advance"
        );
        self.predecessor = Some(sequence);
    }

    pub fn has_capacity(&self, count: usize) -> bool {
        count <= self.capacity - self.occupied
    }

    pub fn front_progress(&self) -> Option<ProgressIdentity> {
        match self.entries.front() {
            Some(Entry::Progress { frame, obligation }) => {
                let (progress, sequence) = frame.identity();
                Some(ProgressIdentity::new(
                    obligation.revision(),
                    progress,
                    sequence,
                ))
            }
            Some(Entry::Data(_)) | None => None,
        }
    }

    pub fn front_progress_is_end(&self) -> bool {
        matches!(
            self.entries.front(),
            Some(Entry::Progress { frame, .. }) if frame.is_end()
        )
    }

    pub fn front_data(&self) -> Option<RevisionId> {
        match self.entries.front() {
            Some(Entry::Data(Data { obligation, .. })) => {
                Some(obligation.revision())
            }
            Some(Entry::Progress { .. }) | None => None,
        }
    }

    pub fn take_data(&mut self) -> Option<Data<I>> {
        if !matches!(self.entries.front(), Some(Entry::Data(_))) {
            return None;
        }
        let Some(Entry::Data(data)) = self.entries.pop_front() else {
            unreachable!("visible data changed before transfer");
        };
        self.release(1);
        Some(data)
    }

    pub fn take_progress(
        &mut self, identity: ProgressIdentity,
    ) -> (ProgressFrame, Obligation) {
        assert_eq!(self.front_progress(), Some(identity));
        let Some(Entry::Progress { frame, obligation }) =
            self.entries.pop_front()
        else {
            unreachable!("validated progress lane");
        };
        self.release(1);
        (frame, obligation)
    }

    pub fn restore_data(&mut self, data: Data<I>) {
        assert!(self.has_capacity(1), "lane tail lost its capacity position");
        self.occupied = self
            .occupied
            .checked_add(1)
            .expect("lane occupancy overflowed");
        self.entries.push_front(Entry::Data(data));
    }

    pub fn reserve_prechecked(&mut self) -> Reservation {
        assert!(self.has_capacity(1), "full lane was reserved");
        let order = self.issued;
        self.issued =
            self.issued.checked_add(1).expect("lane order overflowed");
        self.occupied = self
            .occupied
            .checked_add(1)
            .expect("lane occupancy overflowed");
        Reservation { order }
    }

    fn commit(&mut self, order: u64, entry: Option<Entry<I>>) -> usize {
        let mut released = 0;
        if let Some(entry) = self.commits.insert(order, entry) {
            released += self.accept(entry);
        }
        released += self.close_gap();
        self.release(released);
        released
    }

    pub fn prune_revision(
        &mut self, revision: RevisionId, obligations: &mut Obligations,
    ) -> usize {
        self.prune_revision_with(revision, obligations, true)
    }

    pub fn prune_revision_all(
        &mut self, revision: RevisionId, obligations: &mut Obligations,
    ) -> usize {
        self.prune_revision_with(revision, obligations, false)
    }

    fn prune_revision_with(
        &mut self, revision: RevisionId, obligations: &mut Obligations,
        preserve_abort: bool,
    ) -> usize {
        let mut released = 0;
        let entries = self.entries.len();
        for _ in 0..entries {
            let entry = self
                .entries
                .pop_front()
                .expect("entry count came from the visible lane");
            if entry.revision() == revision
                && (!preserve_abort || !entry.is_abort())
            {
                obligations.push(entry.into_obligation());
                released += 1;
            } else {
                self.entries.push_back(entry);
            }
        }
        for entry in self.commits.pending_mut() {
            if entry.as_ref().is_some_and(|entry| {
                entry.revision() == revision
                    && (!preserve_abort || !entry.is_abort())
            }) {
                obligations.push(
                    entry
                        .take()
                        .expect("validated pending entry")
                        .into_obligation(),
                );
            }
        }
        released += self.close_gap();
        self.release(released);
        released
    }

    pub fn abort_revision_end(&mut self, revision: RevisionId) {
        for entry in self
            .entries
            .iter_mut()
            .chain(self.commits.pending_mut().filter_map(Option::as_mut))
        {
            if entry.revision() == revision {
                match entry {
                    Entry::Progress { frame, .. } => {
                        frame.abort_end();
                    }
                    Entry::Data(_) => {}
                }
            }
        }
    }

    fn close_gap(&mut self) -> usize {
        let mut released = 0;
        while let Some(entry) = self.commits.pop_ready() {
            released += self.accept(entry);
        }
        released
    }

    fn accept(&mut self, entry: Option<Entry<I>>) -> usize {
        if let Some(entry) = entry {
            self.entries.push_back(entry);
            0
        } else {
            1
        }
    }

    fn release(&mut self, count: usize) {
        self.occupied = self
            .occupied
            .checked_sub(count)
            .expect("lane credit released more than once");
    }
}

// ----------------------------------------------------------------------------

impl<I> Transport<I>
where
    I: Id,
{
    pub fn new(
        graph: &Graph<NodePlan>, required_capacity: usize,
        outputs: Vec<OutputBinding>,
    ) -> Self {
        let lane_capacity = required_capacity.max(BOOTSTRAP_ENTRY_CAPACITY);
        let lanes = (0..graph.len())
            .map(|node| {
                (0..graph[node].inputs.len())
                    .map(|_| Lane::new(lane_capacity))
                    .collect()
            })
            .collect();
        Self {
            lanes,
            egress: Egresses::new(outputs),
        }
    }

    pub fn lane_count(&self, node: usize) -> usize {
        self.lanes[node].len()
    }

    pub fn front_progress(
        &self, node: usize, lane: usize,
    ) -> Option<ProgressIdentity> {
        self.lanes[node][lane].front_progress()
    }

    pub fn front_progress_is_end(&self, node: usize, lane: usize) -> bool {
        self.lanes[node][lane].front_progress_is_end()
    }

    pub fn progress_predecessor(
        &self, node: usize, lane: usize,
    ) -> Option<u64> {
        self.lanes[node][lane].progress_predecessor()
    }

    pub fn front_data(&self, node: usize, lane: usize) -> Option<RevisionId> {
        self.lanes[node][lane].front_data()
    }

    pub fn take_data(&mut self, node: usize, lane: usize) -> Option<Data<I>> {
        self.lanes[node][lane].take_data()
    }

    pub fn record_dispatch(&mut self, node: usize, lane: usize, sequence: u64) {
        self.lanes[node][lane].record_dispatch(sequence);
    }

    pub fn take_progress(
        &mut self, node: usize, lane: usize, identity: ProgressIdentity,
    ) -> (ProgressFrame, Obligation) {
        self.lanes[node][lane].take_progress(identity)
    }

    pub fn restore_data(&mut self, node: usize, lane: usize, data: Data<I>) {
        self.lanes[node][lane].restore_data(data);
    }

    pub fn reserve_repeated(
        &mut self, route: Route, count: usize,
    ) -> Option<OutputReservations> {
        if !self.lanes[route.node][route.lane].has_capacity(count) {
            return None;
        }
        // Keep the ubiquitous zero/one destination cases inline. This value
        // crosses worker execution, so borrowed runtime scratch is invalid;
        // only true fan-out should allocate owned reservation storage.
        Some(match count {
            0 => OutputReservations::empty(),
            1 => OutputReservations::one(
                self.reserve_prechecked(Destination::Route(route)),
            ),
            _ => OutputReservations::many(
                (0..count)
                    .map(|_| self.reserve_prechecked(Destination::Route(route)))
                    .collect(),
            ),
        })
    }

    pub fn reserve_destinations(
        &mut self, destinations: &[Destination],
    ) -> Option<OutputReservations> {
        if destinations
            .iter()
            .any(|&destination| !self.has_capacity(destination))
        {
            return None;
        }
        Some(match destinations {
            [] => OutputReservations::empty(),
            &[destination] => {
                OutputReservations::one(self.reserve_prechecked(destination))
            }
            _ => OutputReservations::many(
                destinations
                    .iter()
                    .copied()
                    .map(|destination| self.reserve_prechecked(destination))
                    .collect(),
            ),
        })
    }

    pub fn reserve_routes(
        &mut self, routes: &[Route],
    ) -> Option<OutputReservations> {
        if routes
            .iter()
            .any(|&route| !self.lanes[route.node][route.lane].has_capacity(1))
        {
            return None;
        }
        Some(match routes {
            [] => OutputReservations::empty(),
            &[route] => OutputReservations::one(
                self.reserve_prechecked(Destination::Route(route)),
            ),
            _ => OutputReservations::many(
                routes
                    .iter()
                    .copied()
                    .map(|route| {
                        self.reserve_prechecked(Destination::Route(route))
                    })
                    .collect(),
            ),
        })
    }

    pub fn reserve_action_and_progress(
        &mut self, destinations: &[Destination], routes: &[Route],
    ) -> Option<(OutputReservations, OutputReservations)> {
        for route in destinations
            .iter()
            .filter_map(|destination| destination.route())
            .chain(routes.iter().copied())
        {
            let count = destinations
                .iter()
                .filter(|destination| destination.route() == Some(route))
                .count()
                + routes
                    .iter()
                    .filter(|&&candidate| candidate == route)
                    .count();
            if !self.lanes[route.node][route.lane].has_capacity(count) {
                return None;
            }
        }
        if destinations.iter().any(|&destination| {
            matches!(destination, Destination::Output(_))
                && !self.has_capacity(destination)
        }) {
            return None;
        }
        let outputs = self.reserve_destinations_prechecked(destinations);
        let progress = self.reserve_routes_prechecked(routes);
        Some((outputs, progress))
    }

    pub fn commit(
        &mut self, reservation: DestinationReservation, entry: Option<Entry<I>>,
    ) -> TransportUpdate {
        let DestinationReservation { destination, position } = reservation;
        match destination {
            Destination::Route(route) => {
                let released = position
                    .commit(&mut self.lanes[route.node][route.lane], entry);
                TransportUpdate {
                    ready: Some(route.node),
                    credit: (released != 0).then_some(Credit::Lane(route)),
                }
            }
            Destination::Output(output) => {
                let (source, released) =
                    self.egress.commit(output, position, entry);
                TransportUpdate {
                    ready: None,
                    credit: (released != 0).then_some(Credit::Output(source)),
                }
            }
        }
    }

    pub fn egress(&mut self) -> Option<(usize, Egress<I>, Obligation)> {
        self.egress.take()
    }

    pub fn abort_revision_end(&mut self, revision: RevisionId) {
        for lanes in &mut self.lanes {
            for lane in lanes {
                lane.abort_revision_end(revision);
            }
        }
    }

    pub fn prune(&mut self, revision: RevisionId) -> Pruned {
        self.prune_with(revision, true)
    }

    pub fn prune_all(&mut self, revision: RevisionId) -> Pruned {
        self.prune_with(revision, false)
    }

    fn prune_with(
        &mut self, revision: RevisionId, preserve_abort: bool,
    ) -> Pruned {
        let mut released_lanes = Vec::new();
        let mut obligations = Obligations::for_revision(revision);
        for (node, lanes) in self.lanes.iter_mut().enumerate() {
            for (lane, entries) in lanes.iter_mut().enumerate() {
                let released = if preserve_abort {
                    entries.prune_revision(revision, &mut obligations)
                } else {
                    entries.prune_revision_all(revision, &mut obligations)
                };
                if released != 0 {
                    released_lanes.push(Route::new(node, lane));
                }
            }
        }
        let released_outputs = self.egress.prune(revision, &mut obligations);
        Pruned {
            released_lanes,
            released_outputs,
            obligations,
        }
    }

    fn has_capacity(&self, destination: Destination) -> bool {
        match destination {
            Destination::Route(route) => {
                self.lanes[route.node][route.lane].has_capacity(1)
            }
            Destination::Output(output) => self.egress.has_capacity(output),
        }
    }

    fn reserve_prechecked(
        &mut self, destination: Destination,
    ) -> DestinationReservation {
        let position = match destination {
            Destination::Route(route) => {
                self.lanes[route.node][route.lane].reserve_prechecked()
            }
            Destination::Output(output) => {
                self.egress.reserve_prechecked(output)
            }
        };
        DestinationReservation { destination, position }
    }

    fn reserve_destinations_prechecked(
        &mut self, destinations: &[Destination],
    ) -> OutputReservations {
        match destinations {
            [] => OutputReservations::empty(),
            &[destination] => {
                OutputReservations::one(self.reserve_prechecked(destination))
            }
            _ => OutputReservations::many(
                destinations
                    .iter()
                    .copied()
                    .map(|destination| self.reserve_prechecked(destination))
                    .collect(),
            ),
        }
    }

    fn reserve_routes_prechecked(
        &mut self, routes: &[Route],
    ) -> OutputReservations {
        match routes {
            [] => OutputReservations::empty(),
            &[route] => OutputReservations::one(
                self.reserve_prechecked(Destination::Route(route)),
            ),
            _ => OutputReservations::many(
                routes
                    .iter()
                    .copied()
                    .map(|route| {
                        self.reserve_prechecked(Destination::Route(route))
                    })
                    .collect(),
            ),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for OutputReservations {
    type Item = DestinationReservation;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.positions {
            ReservationPositions::Empty => None,
            ReservationPositions::One(position) => position.take(),
            ReservationPositions::Many(positions) => positions.next(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = match &self.positions {
            ReservationPositions::Empty => 0,
            ReservationPositions::One(position) => {
                usize::from(position.is_some())
            }
            ReservationPositions::Many(positions) => positions.len(),
        };
        (len, Some(len))
    }
}

impl ExactSizeIterator for OutputReservations {}

impl iter::FusedIterator for OutputReservations {}

#[cfg(test)]
mod tests {
    use super::{
        Data, DestinationReservation, Entry, Lane, OutputReservations,
        Reservation,
    };
    use crate::scheduler::Change;
    use crate::scheduler::action::Segment;
    use crate::scheduler::plan::{Destination, InputIndex, Route};
    use crate::scheduler::runtime::progress::{Obligations, Revisions};

    fn segment(value: u64) -> Segment<u64> {
        Segment::new(vec![Change::Insert(value, value)])
    }

    fn reservation(order: u64) -> DestinationReservation {
        DestinationReservation {
            destination: Destination::Route(Route::new(0, 0)),
            position: Reservation { order },
        }
    }

    #[test]
    fn reservation_storage_iterates_empty_inline_and_fanout_shapes() {
        let empty = OutputReservations::empty();
        assert_eq!(empty.len(), 0);

        let mut one = OutputReservations::one(reservation(1));
        assert_eq!(one.len(), 1);
        assert_eq!(one.next().unwrap().position.order, 1);
        assert_eq!(one.len(), 0);

        let many =
            OutputReservations::many(vec![reservation(2), reservation(3)]);
        assert_eq!(
            many.map(|reservation| reservation.position.order)
                .collect::<Vec<_>>(),
            [2, 3]
        );
    }

    #[test]
    fn pending_empty_positions_release_only_after_the_fifo_gap_closes() {
        let mut lane = Lane::<u64>::new(2);
        let first = lane.reserve_prechecked();
        let second = lane.reserve_prechecked();
        assert!(!lane.has_capacity(1));

        assert_eq!(second.commit(&mut lane, None), 0);
        assert!(!lane.has_capacity(1));
        assert_eq!(first.commit(&mut lane, None), 2);
        assert!(lane.has_capacity(2));
    }

    #[test]
    fn consuming_a_visible_entry_releases_its_credit() {
        let mut revisions = Revisions::default();
        let revision = revisions.begin(InputIndex::new(1));
        let obligation =
            revisions.admit_many(revision, 1).unwrap().next().unwrap();
        let mut lane = Lane::new(1);
        let reservation = lane.reserve_prechecked();
        assert_eq!(
            reservation
                .commit(&mut lane, Some(Entry::data(segment(1), obligation)),),
            0
        );
        assert!(!lane.has_capacity(1));

        let _ = lane.take_data().unwrap();
        assert!(lane.has_capacity(1));
    }

    #[test]
    fn pruning_a_pending_entry_retains_credit_until_order_reaches_it() {
        let mut revisions = Revisions::default();
        let revision = revisions.begin(InputIndex::new(1));
        let obligation =
            revisions.admit_many(revision, 1).unwrap().next().unwrap();
        let mut lane = Lane::new(2);
        let first = lane.reserve_prechecked();
        let second = lane.reserve_prechecked();
        assert_eq!(
            second
                .commit(&mut lane, Some(Entry::data(segment(1), obligation)),),
            0
        );

        let mut obligations = Obligations::for_revision(revision);
        assert_eq!(lane.prune_revision(revision, &mut obligations), 0);
        assert_eq!(obligations.len(), 1);
        assert!(!lane.has_capacity(1));
        assert_eq!(first.commit(&mut lane, None), 2);
        assert!(lane.has_capacity(2));
    }

    #[test]
    fn pruning_visible_entries_preserves_retained_fifo_order() {
        let mut revisions = Revisions::default();
        let retained = revisions.begin(InputIndex::new(1));
        let removed = revisions.begin(InputIndex::new(2));
        let mut lane = Lane::new(3);
        for (revision, value) in [(retained, 1), (removed, 2), (retained, 3)] {
            let obligation =
                revisions.admit_many(revision, 1).unwrap().next().unwrap();
            let reservation = lane.reserve_prechecked();
            assert_eq!(
                reservation.commit(
                    &mut lane,
                    Some(Entry::data(segment(value), obligation)),
                ),
                0
            );
        }

        let mut obligations = Obligations::for_revision(removed);
        assert_eq!(lane.prune_revision(removed, &mut obligations), 1);
        assert_eq!(obligations.len(), 1);

        let values = [lane.take_data().unwrap(), lane.take_data().unwrap()]
            .map(|Data { segment, obligation, .. }| {
                assert_eq!(obligation.revision(), retained);
                let mut value = None;
                segment.drain::<u64>(|change| {
                    let Change::Insert(_, current) = change else {
                        panic!("test segment contained a removal");
                    };
                    value = Some(*current.as_ref());
                });
                value.unwrap()
            });
        assert_eq!(values, [1, 3]);
        assert!(lane.take_data().is_none());
    }
}
