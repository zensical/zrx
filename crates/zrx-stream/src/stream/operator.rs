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

//! Stream operators.

use zrx_scheduler::Value;
use zrx_scheduler::action::replication::IntoReplication;
pub use zrx_scheduler::action::replication::{
    Replication, concurrent, sequential,
};
use zrx_scheduler::action::{Action, Inputs, Job};

use crate::stream::Id;

use super::{Key, Stream};

mod barrier;
mod coalesce;
mod currency;
mod filter;
mod filter_map;
mod flat_map;
mod group_by_key;
mod join;
mod map;
mod product;
mod publication;
mod reduce;
mod reduce_by_key;
mod select;
mod terminal;
mod unique_by_key;
mod window;

pub(in crate::stream) use coalesce::Coalesce;
pub(in crate::stream) use join::{Anti, Full, Inner, Join, Left, Semi};
pub use select::Membership;
use terminal::{Terminal, Tickets};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Typed construction seam shared by individual streams and stream tuples.
pub trait Operator<I>
where
    I: Id,
{
    /// Typed input lanes supplied by this stream shape.
    type Inputs: Inputs<Key<I>>;

    /// Subscribes an action to these input streams.
    fn subscribe<A>(&self, subscriber: A) -> Stream<I, A::Output>
    where
        A: Action<Key<I>, Inputs = Self::Inputs>,
    {
        self.subscribe_job(Job::new(subscriber))
    }

    /// Subscribes an action and revision progress to these input streams.
    fn subscribe_progress<A>(&self, subscriber: A) -> Stream<I, A::Output>
    where
        A: Action<Key<I>, Inputs = Self::Inputs>,
    {
        self.subscribe_job(Job::new(subscriber).with_progress())
    }

    /// Subscribes an already installed action job to these input streams.
    fn subscribe_job<U>(&self, job: Job<Key<I>>) -> Stream<I, U>
    where
        U: Value;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Operator<I> for Stream<I, T>
where
    I: Id,
    T: Value,
{
    type Inputs = (T,);

    #[inline]
    fn subscribe_job<U>(&self, job: Job<Key<I>>) -> Stream<I, U>
    where
        U: Value,
    {
        let workflow = self
            .workflow
            .upgrade()
            .expect("stream construction has ended");
        let node = workflow
            .try_borrow_mut()
            .expect("stream construction reentered")
            .job([self.node()], job);
        Stream::new(node, self.workflow.clone())
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

macro_rules! impl_operator_for_tuple {
    ($T1:ident $(, $T:ident)+ $(,)?) => {
        impl<I, $T1, $($T,)+> Operator<I>
            for (Stream<I, $T1>, $(Stream<I, $T>,)+)
        where
            I: Id,
            $T1: Value,
            $($T: Value,)+
        {
            type Inputs = ($T1, $($T,)+);

            #[inline]
            fn subscribe_job<U>(
                &self, job: Job<Key<I>>,
            ) -> Stream<I, U>
            where
                U: Value,
            {
                #[allow(non_snake_case)]
                let ($T1, $($T,)+) = self;
                $(
                    assert!(
                        $T1.workflow.ptr_eq(&$T.workflow),
                        "operator inputs belong to different workflows"
                    );
                )+
                let workflow = $T1
                    .workflow
                    .upgrade()
                    .expect("stream construction has ended");
                let node = workflow
                    .try_borrow_mut()
                    .expect("stream construction reentered")
                    .job(
                        [$T1.node(), $($T.node(),)+],
                        job,
                    );
                Stream::new(node, $T1.workflow.clone())
            }
        }
    };
}

impl_operator_for_tuple!(T1, T2);
impl_operator_for_tuple!(T1, T2, T3);
impl_operator_for_tuple!(T1, T2, T3, T4);
impl_operator_for_tuple!(T1, T2, T3, T4, T5);
impl_operator_for_tuple!(T1, T2, T3, T4, T5, T6);
impl_operator_for_tuple!(T1, T2, T3, T4, T5, T6, T7);
impl_operator_for_tuple!(T1, T2, T3, T4, T5, T6, T7, T8);

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

fn subscribe_function<I, O, F, A, C, P>(
    operator: &O, function: F, construct: C, project: P,
) -> Stream<I, A::Output>
where
    I: Id,
    O: Operator<I> + ?Sized,
    F: IntoReplication,
    F::Target: 'static,
    A: Action<Key<I>, Inputs = O::Inputs>,
    C: Fn(F::Target) -> A + Clone + Send + 'static,
    P: for<'a> Fn(&'a A) -> &'a F::Target + Clone + Send + 'static,
{
    let (function, maximum, replica) = function.into_replication().into_parts();
    let action = construct(function);
    let replica = replica.map(|replica| {
        let construct = construct.clone();
        move |action: &A| construct(replica(project(action)))
    });
    let job = match replica {
        Some(replica) => Job::replicated(action, maximum, replica),
        None => Job::new(action),
    };
    operator.subscribe_job(job)
}

#[cfg(test)]
fn test_revisions(count: usize) -> Vec<zrx_scheduler::RevisionId> {
    use zrx_executor::strategy::Immediate;
    use zrx_scheduler::Settlement;

    let workflow = super::Workflow::<u64>::build(|workflow| {
        let input = workflow.input::<u64>();
        workflow.output(&input);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let mut input = runner.input::<u64>().unwrap();
    let mut revisions = Vec::with_capacity(count);
    for _ in 0..count {
        let open = input.begin().unwrap();
        input = open.seal().unwrap();
        let run = runner.settle().unwrap();
        let [Settlement::Complete(revision)] = run.report().settlements()
        else {
            panic!("empty test revision did not settle exactly once")
        };
        revisions.push(*revision);
    }
    revisions
}
