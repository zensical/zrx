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

//! Session.

use crossbeam::channel::Sender;
use std::marker::PhantomData;

use super::effect::Item;
use super::executor::ToReceiver;
use super::value::Value;

mod collection;
mod connector;
mod error;
mod message;

pub use collection::Sessions;
pub use connector::Connector;
pub use error::{Error, Result};
pub use message::Message;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Session.
///
/// Sessions provide a thread-safe interface to interact with a [`Scheduler`][],
/// allowing to submit items for processing. They are asynchronous by design, so
/// inserting or removing an item is a submission, not a synchronous operation.
/// Sessions should be moved to dedicated threads to avoid deadlocks.
///
/// Since sessions can have arbitrary types, items are upcasted to [`Value`]
/// before sending, and downcasted by the [`Scheduler`][] when necessary.
///
/// [`Scheduler`]: crate::scheduler::Scheduler
#[derive(Debug)]
pub struct Session<I, T> {
    /// Identifier.
    id: usize,
    /// Item submission sender.
    sender: Sender<Message<I>>,
    /// Type marker.
    marker: PhantomData<T>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Session<I, T>
where
    T: Value,
{
    /// Inserts an item into the session.
    ///
    /// This method inserts an item with an identifier and associated data into
    /// the session, meaning it is processed by the [`Scheduler`][] the session
    /// belongs to. Note that this method might block, if the scheduler is at
    /// capacity, in order to apply backpressure.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the scheduler terminated.
    #[inline]
    pub fn insert(&self, id: I, data: T) -> Result {
        self.sender
            .send(Message::Item(self.id, Item::new(id, Some(Box::new(data)))))
            .map_err(|_| Error::Disconnected)
    }

    /// Removes an item from the session.
    ///
    /// This method removes an item associated with the given identifier from
    /// the session, meaning it is processed by the [`Scheduler`][] the session
    /// belongs to. Note that this method might block, if the scheduler is at
    /// capacity, in order to apply backpressure.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the scheduler terminated.
    #[inline]
    pub fn remove(&self, id: I) -> Result {
        self.sender
            .send(Message::Item(self.id, Item::new(id, None)))
            .map_err(|_| Error::Disconnected)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, T> Session<I, T> {
    /// Returns the identifier of the session.
    #[inline]
    pub fn id(&self) -> usize {
        self.id
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Drop for Session<I, T> {
    /// Sends a drop notification to the scheduler.
    ///
    /// Note that it's safe for us to ignore the result of the send operation,
    /// since it can only fail when the session is disconnected, which is the
    /// case when the scheduler terminated anyway. Thus, we can just swallow
    /// the error without panicking.
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Drop(self.id));
    }
}
