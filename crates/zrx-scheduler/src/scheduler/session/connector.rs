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

//! Session connector.

use crossbeam::channel::{bounded, Receiver, Sender};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::mem;

use super::message::Message;
use super::{Session, ToReceiver};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Session connector.
///
/// This data type manages the communication between the scheduler and a set of
/// sessions, using a [`crossbeam`] channel to send and receive items.
///
/// Since the connector is owned by the [`Scheduler`][], it's ensured that there
/// is only a single [`Receiver`]. When the scheduler terminates, the channel is
/// automatically closed and propagates [`Error::Disconnected`][], which is the
/// termination signal, to all owners of a session.
///
/// [`Error::Disconnected`]: crate::scheduler::session::Error::Disconnected
/// [`Scheduler`]: crate::scheduler::Scheduler
#[derive(Debug)]
pub struct Connector<I> {
    /// Item submission sender.
    sender: Sender<Message<I>>,
    /// Item collection receiver.
    receiver: Receiver<Message<I>>,
    /// Next identifier.
    next: usize,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

impl<I> Connector<I> {
    /// Creates a session connector.
    ///
    /// This method creates a session connector with a capacity of 1,024 items.
    /// In case you need a different capacity, use [`Connector::with_capacity`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::session::Connector;
    ///
    /// // Create session connector
    /// let connector = Connector::new();
    /// # let _: Connector<()> = connector;
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Creates a session connector with the given capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::session::Connector;
    ///
    /// // Create session connector
    /// let connector = Connector::with_capacity(2048);
    /// # let _: Connector<()> = connector;
    /// ```
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, receiver) = bounded(capacity);
        Self { sender, receiver, next: 0 }
    }

    /// Creates a session.
    ///
    /// The session is automatically closed and cleaned up when dropped. Since
    /// the [`Scheduler`][] owns the [`Connector`], it will manage sessions and
    /// also check, whether an appropriate source has been registered, or items
    /// would just be swallowed without any effect.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::session::Connector;
    ///
    /// // Create session connector
    /// let mut connector = Connector::new();
    ///
    /// // Create session and insert item
    /// let session = connector.session();
    /// session.insert("id", 42)?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn session<T>(&mut self) -> Session<I, T> {
        let id = self.next + 1;
        Session {
            id: mem::replace(&mut self.next, id),
            sender: self.sender.clone(),
            marker: PhantomData,
        }
    }

    /// Returns the next item.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::session::Connector;
    ///
    /// // Create session connector
    /// let mut connector = Connector::new();
    ///
    /// // Create session and insert item
    /// let session = connector.session();
    /// session.insert("a", 1)?;
    /// session.insert("b", 2)?;
    /// session.insert("c", 3)?;
    ///
    /// // Obtain items from connector
    /// while let Some(item) = connector.take() {
    ///     println!("{item:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn take(&self) -> Option<Message<I>> {
        self.receiver.try_recv().ok()
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Connector<I> {
    // Returns the number of items.
    #[inline]
    pub fn len(&self) -> usize {
        self.receiver.len()
    }

    /// Returns whether there are any items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.receiver.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> ToReceiver<I> for Connector<I> {
    type Item = Message<I>;

    /// Returns the receiver of the session connector.
    #[inline]
    fn to_receiver(&self) -> Cow<'_, Receiver<Self::Item>> {
        Cow::Borrowed(&self.receiver)
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Connector<I> {
    /// Creates a session connector.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::session::Connector;
    ///
    /// // Create session connector
    /// let connector = Connector::new();
    /// # let _: Connector<()> = connector;
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
