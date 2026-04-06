// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// Third-party contributions licensed under DCO

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

//! Subscriber.

use crossbeam::channel::unbounded;
use zrx_storage::Storage;
use zrx_store::Collection;

use crate::scheduler::action::{Action, Options};
use crate::scheduler::signal::Diff;
use crate::scheduler::signal::{Id, Scope, Value};

use super::error::Result;
use super::graph::{Descriptor, Handler, Node, Source, Worker};
use super::Builder;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Subscriber.
///
/// This data type provides a builder-like interface to associate an [`Action`]
/// with [`Options`], and specify the type of [`Storage`] for the outputs of an
/// action, as some actions might require storages with specific semantics. The
/// subscriber is consumed by the [`Builder`], adding it to a [`Sequence`][].
///
/// While a [`Subscriber`] can be constructed via [`Subscriber::new`], using it
/// in combination with [`Into`] on trait bounds is more ergonomic, allowing to
/// pass an [`Action`] directly to the [`Builder`]. The action is automatically
/// converted into a [`Subscriber`] with default options.
///
/// [`Sequence`]: crate::scheduler::action::Sequence
pub struct Subscriber<'a, I, A>
where
    A: Action<I>,
{
    /// Action.
    action: A,
    /// Action options.
    options: Options,
    /// Action storage.
    storage: Option<Storage<Scope<I>, A::Output<'a>>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, I, A> Subscriber<'a, I, A>
where
    A: Action<I>,
{
    /// Creates a subscriber from an action.
    #[inline]
    pub fn new(action: A) -> Self {
        Self {
            action,
            options: Options::default(),
            storage: None,
        }
    }

    /// Sets the options for the action.
    #[inline]
    #[must_use]
    pub fn with_options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Sets the storage for the action.
    #[inline]
    #[must_use]
    pub fn with_storage<S>(mut self, storage: S) -> Self
    where
        S: Collection<Scope<I>, A::Output<'a>>,
    {
        self.storage = Some(Storage::new(storage));
        self
    }
}

// ----------------------------------------------------------------------------

impl<I> Builder<I>
where
    I: Id,
{
    /// Adds a subscriber to the given nodes in the action graph.
    ///
    /// This method creates a new node in the action graph, attaches the given
    /// subscriber to it, and connects the given source nodes to the new node,
    /// so that the outputs of the source nodes are used as inputs for the
    /// action associated with the subscriber.
    ///
    /// When adding a [`Source`][], it's mandatory to pass [`None`], as source
    /// nodes don't need to be connected to other nodes. We do not enforce this
    /// on an API level, as the [`Builder`] is a low-level construct that should
    /// be encapsulated by a high level API like the canonical streaming API.
    ///
    /// [`Source`]: crate::scheduler::action::boundary::Source
    ///
    /// # Errors
    ///
    /// In case a source nodes does not exist, [`Error::Graph`][] is returned,
    /// to make sure the graph does not contain stale node references.
    ///
    /// [`Error::Graph`]: super::error::Error::Graph
    pub fn add<'a, N, S, A>(&mut self, nodes: N, subscriber: S) -> Result<usize>
    where
        N: IntoIterator<Item = usize>,
        S: Into<Subscriber<'a, I, A>>,
        A: Action<I> + 'static,
    {
        let Subscriber { action, options, storage } = subscriber.into();
        let handler = Handler::new(action, options);

        // Create target node for subscriber, creating a descriptor for matching
        // nodes during graph construction, and a worker for action execution
        let target = self.graph.add_node(Node::new(
            Descriptor::of::<A::Output<'a>>(),
            Worker::new(handler),
        ));

        // Obtain action storage from subscriber, if specified, or otherwise
        // create a default storage, and add it to the storage set
        let n = self.storages.insert(storage.unwrap_or_default());
        debug_assert_eq!(n, target);

        // Add an edge from each source node to the target node, to source the
        // inputs for the action obtained from the subscriber from the outputs
        // of each of the given source nodes. Thus, actions are just functions
        // embedded in a graph, which models their interdependencies.
        for source in nodes {
            self.graph.add_edge(source, target)?;
        }

        // Return target node
        Ok(target)
    }

    /// Adds a source.
    pub fn add_source<T>(&mut self) -> usize
    where
        T: Value,
    {
        let (sender, receiver) = unbounded::<Diff<I, T>>();
        let handler = Handler::new(receiver, Options::default());

        // Create target node for source
        let target = self.graph.add_node(Node::new(
            Descriptor::of::<T>(),
            Source::new(handler, sender),
        ));

        // Create action storage from subscriber
        let n = self.storages.insert(Storage::<Scope<I>, T>::default());
        debug_assert_eq!(n, target);

        // Return target node
        target
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, A> From<A> for Subscriber<'_, I, A>
where
    A: Action<I>,
{
    /// Creates a subscriber from an action.
    #[inline]
    fn from(action: A) -> Self {
        Self::new(action)
    }
}
