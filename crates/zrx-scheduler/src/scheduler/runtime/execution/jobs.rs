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

//! Per-node job ownership and ordered reconciliation.

use crate::scheduler::Id;
use crate::scheduler::action::Job;
use crate::scheduler::runtime::ordered::OrderedWindow;

use super::{Reconciliation, Started, Ticket};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub enum Access {
    Shared,
    Exclusive,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Replicas<I>
where
    I: Id,
{
    available: Vec<Job<I>>,
    total: usize,
    limit: usize,
    exclusive: bool,
}

// ----------------------------------------------------------------------------

struct Reconciler<I>
where
    I: Id,
{
    issued: u64,
    returns: OrderedWindow<Reconciliation<I>>,
}

// ----------------------------------------------------------------------------

struct Node<I>
where
    I: Id,
{
    replicas: Replicas<I>,
    reconciler: Reconciler<I>,
}

// ----------------------------------------------------------------------------

pub struct Jobs<I>
where
    I: Id,
{
    nodes: Vec<Node<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Access {
    #[cfg(feature = "tracing")]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Exclusive => "exclusive",
        }
    }
}

// ----------------------------------------------------------------------------

impl<I> Replicas<I>
where
    I: Id,
{
    fn new(job: Job<I>, limit: usize) -> Self {
        Self {
            available: vec![job],
            total: 1,
            limit,
            exclusive: false,
        }
    }

    fn ready(&self, access: Access) -> bool {
        !self.exclusive
            && match access {
                Access::Shared => !self.available.is_empty(),
                Access::Exclusive => self.available.len() == self.total,
            }
    }

    const fn parallelism(&self) -> usize {
        self.limit
    }

    fn take(&mut self, access: Access) -> Job<I> {
        assert!(self.ready(access), "node has no admissible replica");
        match access {
            Access::Shared => self.expand(),
            Access::Exclusive => self.exclusive = true,
        }
        self.available.pop().expect("ready job is resident")
    }

    /// Creates at most one replica immediately before concurrent demand can
    /// use it. Installation retains only the original job, so plans do not pay
    /// for dormant worker parallelism.
    fn expand(&mut self) {
        if self.total == self.limit {
            return;
        }
        self.available.push(self.available[0].replica());
        self.total += 1;
    }

    fn put(&mut self, access: Access, job: Job<I>) {
        assert!(
            self.available.len() < self.total,
            "completed job was not in flight"
        );
        match access {
            Access::Shared => {
                assert!(
                    !self.exclusive,
                    "shared job completed during exclusive access"
                );
            }
            Access::Exclusive => {
                assert!(self.exclusive, "exclusive job was not in flight");
                self.exclusive = false;
            }
        }
        self.available.push(job);
    }
}

// ----------------------------------------------------------------------------

impl<I> Reconciler<I>
where
    I: Id,
{
    fn new(capacity: usize) -> Self {
        Self {
            issued: 0,
            returns: OrderedWindow::new(capacity),
        }
    }

    fn can_issue(&self) -> bool {
        let outstanding = self
            .issued
            .checked_sub(self.returns.next())
            .expect("node reconciled work that was not issued");
        outstanding < self.returns.capacity() as u64
    }

    fn is_idle(&self) -> bool {
        self.issued == self.returns.next() && self.returns.is_pending_empty()
    }

    fn reconciled(&self, sequence: u64) -> bool {
        debug_assert!(sequence < self.issued, "unissued work cannot reconcile");
        sequence < self.returns.next()
    }

    fn issue(&mut self) -> u64 {
        assert!(self.can_issue(), "node exceeded its reconciliation window");
        let sequence = self.issued;
        self.issued = sequence
            .checked_add(1)
            .expect("node dispatch sequence overflowed");
        sequence
    }

    fn complete(
        &mut self, sequence: u64, reconciliation: Reconciliation<I>,
    ) -> Option<Reconciliation<I>> {
        self.returns.insert(sequence, reconciliation)
    }

    fn pop(&mut self) -> Option<Reconciliation<I>> {
        self.returns.pop_ready()
    }
}

// ----------------------------------------------------------------------------

impl<I> Node<I>
where
    I: Id,
{
    fn new(job: Job<I>, shards: usize) -> Self {
        let parallelism = job.parallelism(shards);
        let replicas = Replicas::new(job, parallelism);
        let reconciler = Reconciler::new(parallelism);
        Self { replicas, reconciler }
    }

    fn ready(&self, access: Access) -> bool {
        self.replicas.ready(access)
            && match access {
                Access::Shared => self.reconciler.can_issue(),
                Access::Exclusive => self.reconciler.is_idle(),
            }
    }

    fn parallelism(&self) -> usize {
        self.replicas.parallelism()
    }

    fn reconciled(&self, sequence: u64) -> bool {
        self.reconciler.reconciled(sequence)
    }

    fn start(&mut self, node: usize, access: Access) -> Started<I> {
        assert!(
            self.ready(access),
            "node started work that was not admissible"
        );
        let sequence = self.reconciler.issue();
        let job = self.replicas.take(access);
        Started {
            ticket: Ticket { node, sequence, access },
            job,
        }
    }

    fn complete(
        &mut self, sequence: u64, access: Access, job: Job<I>,
        reconciliation: Reconciliation<I>,
    ) -> Option<Reconciliation<I>> {
        self.replicas.put(access, job);
        self.reconciler.complete(sequence, reconciliation)
    }

    fn pop(&mut self) -> Option<Reconciliation<I>> {
        self.reconciler.pop()
    }
}

// ----------------------------------------------------------------------------

impl<I> Jobs<I>
where
    I: Id,
{
    pub fn new(jobs: Vec<Job<I>>, shards_per_node: usize) -> Self {
        let nodes = jobs
            .into_iter()
            .map(|job| Node::new(job, shards_per_node))
            .collect();
        Self { nodes }
    }

    pub fn ready(&self, node: usize, access: Access) -> bool {
        self.nodes[node].ready(access)
    }

    pub fn parallelism(&self, node: usize) -> usize {
        self.nodes[node].parallelism()
    }

    pub fn reconciled(&self, node: usize, sequence: u64) -> bool {
        self.nodes[node].reconciled(sequence)
    }

    pub fn start(&mut self, node: usize, access: Access) -> Started<I> {
        self.nodes[node].start(node, access)
    }

    #[expect(
        clippy::needless_pass_by_value,
        reason = "completion consumes the one-shot execution ticket"
    )]
    pub(super) fn complete(
        &mut self, ticket: Ticket, job: Job<I>,
        reconciliation: Reconciliation<I>,
    ) -> Option<Reconciliation<I>> {
        self.nodes[ticket.node].complete(
            ticket.sequence,
            ticket.access,
            job,
            reconciliation,
        )
    }

    pub fn pop_ready(&mut self, node: usize) -> Option<Reconciliation<I>> {
        self.nodes[node].pop()
    }
}
