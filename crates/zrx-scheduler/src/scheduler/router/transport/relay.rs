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

//! Relay.

use crossbeam::channel::{Receiver, Select, Sender, TryRecvError};

use crate::scheduler::engine::Token;

use super::error::{Error, Result};
use super::senders::Senders;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Relay.
#[derive(Debug)]
pub struct Relay<T> {
    /// Receiver.
    receiver: Receiver<T>,
    /// Sender set.
    senders: Senders<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Relay<T> {
    /// Adds a sender to the relay.
    pub fn add(&mut self, token: Token, sender: Sender<T>) {
        self.senders.add(token, sender);
    }

    /// Adds the relay's receiver to a select.
    #[inline]
    pub fn add_to_select<'a>(&'a self, select: &mut Select<'a>) -> usize {
        select.recv(&self.receiver)
    }

    /// Polls the relay, sending any received messages to all senders.
    pub fn poll(&mut self) -> Result<Vec<Token>>
    where
        T: Clone,
    {
        // Errors in case all senders have been dropped - nothing to poll
        match self.receiver.try_recv() {
            Ok(value) => Ok(self.senders.send(value)),
            Err(TryRecvError::Disconnected) => Err(Error::Disconnected)?,
            Err(TryRecvError::Empty) => Ok(vec![]),
        }
    }
}

// #[allow(clippy::must_use_candidate)]
// impl<T> Relay<T> {
//     /// Returns the number of senders.
//     #[inline]
//     pub fn len(&self) -> usize {
//         self.senders.len()
//     }

//     /// Returns whether there are any senders.
//     #[inline]
//     pub fn is_empty(&self) -> bool {
//         self.senders.is_empty()
//     }
// }

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<Receiver<T>> for Relay<T> {
    /// Creates a relay from a receiver.
    #[inline]
    fn from(receiver: Receiver<T>) -> Self {
        Self {
            receiver,
            senders: Senders::default(),
        }
    }
}
