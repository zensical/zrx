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

use super::signal::{Diff, Scope, Value};

mod error;

pub use error::{Error, Result};

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
/// [`Scheduler`]: crate::scheduler::Scheduler
#[derive(Debug)]
pub struct Session<I, T> {
    /// Message submission sender.
    pub(crate) sender: Sender<Diff<I, T>>,
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
    /// This method inserts an item with a scope and associated value into the
    /// session, queueing it for processing by the [`Scheduler`][] the session
    /// belongs to. Note that calling this method might block in order to apply
    /// backpressure, which happens if the scheduler is at capacity.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the scheduler terminated.
    #[inline]
    pub fn insert<S>(&self, scope: S, value: T) -> Result
    where
        S: Into<Scope<I>>,
    {
        self.sender
            .send(Diff::Insert(scope.into(), value))
            .map_err(|_| Error::Disconnected)
    }

    /// Removes an item from the session.
    ///
    /// This method removes the item associated with the given scope from the
    /// session, queueing it for processing by the [`Scheduler`][] the session
    /// belongs to. Note that calling this method might block in order to apply
    /// backpressure, which happens if the scheduler is at capacity.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    ///
    /// # Errors
    ///
    /// Returns [`Error::Disconnected`] if the scheduler terminated.
    #[inline]
    pub fn remove<S>(&self, scope: S) -> Result
    where
        S: Into<Scope<I>>,
    {
        self.sender
            .send(Diff::Remove(scope.into()))
            .map_err(|_| Error::Disconnected)
    }
}
