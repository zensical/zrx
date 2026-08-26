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

//! Router.

use crossbeam::channel::{Select, bounded};
use std::ops::{Index, IndexMut, Range};

use crate::scheduler::engine::Token;
use crate::scheduler::session::Session;
use crate::scheduler::signal::{Id, Value};

pub mod traits;
pub mod transport;

use traits::{AnyRelay, AnySender};
use transport::{Error, Relay};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Router.
#[derive(Debug)]
pub struct Router<I> {
    /// Relays.
    relays: Vec<Box<dyn AnyRelay<I>>>,
    /// Senders.
    senders: Vec<(Box<dyn AnySender<I>>, Token)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Router<I>
where
    I: Id,
{
    /// Creates a session.
    pub fn session<T>(&mut self) -> Session<I, T>
    where
        T: Value,
    {
        let (sender, receiver) = bounded(1024);

        // Create relay from receiver and add senders to relay.
        let mut relay = Relay::from(receiver);
        for (inner, token) in &self.senders {
            // Ignore, since we attach to as many relays as possible.
            let _ = AnyRelay::add(&mut relay, *token, inner.as_ref());
        }

        // Add relay to router and return session
        self.relays.push(Box::new(relay));
        Session { sender }
    }

    /// Adds a sender to the router.
    pub fn add(&mut self, token: Token, sender: &dyn AnySender<I>) {
        self.senders.push((sender.clone_box(), token));
        for relay in &mut self.relays {
            // If this fails, sender doesn't match relay
            let _ = relay.add(token, sender);
        }
    }

    /// Adds the relay's receiver to a select.
    #[inline]
    pub fn add_to_select<'a>(
        &'a self, select: &mut Select<'a>,
    ) -> Range<usize> {
        if self.relays.is_empty() {
            return 0..0;
        }
        let start = self.relays[0].add_to_select(select);
        for relay in &self.relays[1..] {
            relay.add_to_select(select);
        }
        start..start + self.relays.len()
    }

    /// Polls the router.
    pub fn poll(&mut self, index: usize) -> Vec<Token> {
        match self.relays[index].poll() {
            Ok(tokens) => return tokens,
            Err(Error::Disconnected) => {
                self.relays.swap_remove(index);
            }
            _ => unreachable!(),
        }

        // Return nothing
        vec![]
    }
}

#[allow(clippy::must_use_candidate)]
impl<T> Router<T> {
    /// Returns the number of relays.
    #[inline]
    pub fn len(&self) -> usize {
        self.relays.len()
    }

    /// Returns whether there are any relays.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.relays.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Index<usize> for Router<I> {
    type Output = dyn AnyRelay<I>;

    /// Returns a reference to the relay at the given index.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        self.relays[index].as_ref()
    }
}

impl<I> IndexMut<usize> for Router<I> {
    /// Returns a mutable reference to the relay at the given index.
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.relays[index].as_mut()
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Router<I> {
    /// Creates a router.
    #[inline]
    fn default() -> Self {
        Self {
            relays: Vec::default(),
            senders: Vec::default(),
        }
    }
}
