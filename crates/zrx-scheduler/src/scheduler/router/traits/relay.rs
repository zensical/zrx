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

//! Generic relay.

use crossbeam::channel::Select;
use std::any::Any;
use std::fmt::Debug;

use crate::scheduler::engine::Token;
use crate::scheduler::router::traits::AnySender;
use crate::scheduler::router::transport::{Relay, Result};
use crate::scheduler::signal::{Diff, Id, Value};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Generic relay.
pub trait AnyRelay<I>: Any + Debug {
    /// Adds a sender to the relay.
    fn add(&mut self, token: Token, sender: &dyn AnySender<I>) -> Result;

    /// Adds the relay's receiver to a select.
    fn add_to_select<'a>(&'a self, select: &mut Select<'a>) -> usize;

    /// Polls the relay, sending any received messages to all senders.
    fn poll(&mut self) -> Result<Vec<Token>>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> AnyRelay<I> for Relay<Diff<I, T>>
where
    I: Id,
    T: Value,
{
    /// Adds a sender to the relay.
    fn add(&mut self, token: Token, sender: &dyn AnySender<I>) -> Result {
        let sender = sender.downcast_ref()?;
        self.add(token, sender.clone());
        Ok(())
    }

    /// Adds the relay's receiver to a select.
    fn add_to_select<'a>(&'a self, select: &mut Select<'a>) -> usize {
        self.add_to_select(select)
    }

    /// Polls the relay, sending any received messages to all senders.
    fn poll(&mut self) -> Result<Vec<Token>> {
        self.poll()
    }
}
