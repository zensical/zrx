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

//! Source.

use crossbeam::channel::Sender;

use crate::scheduler::action::graph::node::handler::{Handler, Iter};
use crate::scheduler::action::{Context, Options};
use crate::scheduler::router::traits::AnySender;
use crate::scheduler::signal::{Diff, Id, Value};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Source.
#[derive(Debug)]
pub struct Source<I> {
    /// Source handler.
    handler: Handler<I>,
    /// Source sender.
    sender: Box<dyn AnySender<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Source<I>
where
    I: Id,
{
    /// Creates a source.
    #[must_use]
    pub fn new<T>(handler: Handler<I>, sender: Sender<Diff<I, T>>) -> Self
    where
        T: Value,
    {
        Self {
            handler,
            sender: Box::new(sender),
        }
    }

    /// Executes the source handler.
    #[inline]
    pub fn execute(&mut self, ctx: Context<I>) -> Iter<'_, I> {
        self.handler.execute(ctx)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Source<I> {
    /// Returns the handler options.
    #[inline]
    pub fn options(&self) -> &Options {
        self.handler.options()
    }

    /// Returns the sender for this source.
    #[inline]
    pub fn sender(&self) -> &dyn AnySender<I> {
        self.sender.as_ref()
    }
}
