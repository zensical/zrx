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

//! Worker.

use std::fmt::{self, Debug};
use std::vec::Drain;

use crate::scheduler::action::{Action, Context, Options, Result};
use crate::scheduler::engine::Tag;
use crate::scheduler::step::{IntoSteps, Steps};

mod receiver;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Handler function.
trait HandlerFn<I> {
    /// Executes the handler function.
    fn execute(&mut self, ctx: Context<I>, buffer: &mut Vec<Result<Steps<I>>>);
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Handler.
///
/// Handlers basically represent type-erased actions, and are the main unit of
/// execution in the scheduler. Each handler is created from an [`Action`], and
/// can additionally carry [`Options`] to further control its execution, e.g.,
/// to limit concurrency or to specify an execution timeout.
pub struct Handler<I> {
    /// Handler function.
    function: Box<dyn HandlerFn<I>>,
    /// Handler options.
    options: Options,
    /// Handler buffer.
    buffer: Vec<Result<Steps<I>>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Handler<I> {
    /// Creates a handler.
    #[must_use]
    pub fn new<A>(action: A, options: Options) -> Self
    where
        A: Action<I> + 'static,
    {
        Self {
            function: Box::new(action),
            options,
            buffer: Vec::new(),
        }
    }

    /// Executes the handler.
    #[inline]
    pub fn execute(&mut self, ctx: Context<I>) -> Iter<'_, I> {
        self.function.execute(ctx, &mut self.buffer);
        self.buffer.drain(..)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Handler<I> {
    /// Returns the handler options.
    #[inline]
    pub fn options(&self) -> &Options {
        &self.options
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Debug for Handler<I>
where
    I: Debug,
{
    /// Formats the handler for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let function = "Box<dyn HandlerFn>";
        f.debug_struct("Handler")
            .field("function", &function)
            .field("options", &self.options)
            .field("buffer", &self.buffer)
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<I, A> HandlerFn<I> for A
where
    A: Action<I>,
{
    #[inline]
    fn execute(&mut self, ctx: Context<I>, buffer: &mut Vec<Result<Steps<I>>>) {
        Action::execute(self, ctx.tag())
            .into_steps()
            .map(|res| res.map(Tag::tag))
            .for_each(|res| buffer.push(res.map_err(Into::into)));
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Handler iterator.
pub type Iter<'a, I> = Drain<'a, Result<Steps<I>>>;
