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

//! Continuation.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use crate::scheduler::action::Context;
use crate::scheduler::engine::Tag;
use crate::scheduler::step::{Result, Steps};

mod builder;

pub use builder::Builder;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Continuation function.
trait ThenFn<I>: Send {
    /// Executes the continuation function.
    ///
    /// # Errors
    ///
    /// Returns [`Error`][] if the function fails to execute.
    ///
    /// [`Error`]: crate::scheduler::action::Error
    fn execute(self: Box<Self>, ctx: Context<I>) -> Result<Steps<I>>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Continuation.
///
/// Continuations are intended to be run on the scheduler thread, since they're
/// passed the [`Context`] of the [`Action`][] being executed. They can capture
/// variables from their environment, such as (and most likely) from the task
/// in order to integrate the results back into the action's flow.
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of effects inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
///
/// [`Action`]: crate::scheduler::action::Action
pub struct Then<I, C = ()> {
    /// Continuation function.
    function: Box<dyn ThenFn<I>>,
    /// Capture types.
    marker: PhantomData<C>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, C> Then<I, C> {
    /// Executes the continuation.
    ///
    /// # Errors
    ///
    /// Errors returned by the task are forwarded. Note that panics are not
    /// caught, as this should happen on higher levels for better control.
    #[inline]
    pub fn execute(self, ctx: Context<I, C>) -> Result<Steps<I, C>> {
        self.function.execute(ctx.tag()).map(Steps::tag)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, C> Tag<I, C> for Then<I, C> {
    type Target<T> = Then<I, T>;

    /// Tags the continuation with the given type.
    #[inline]
    fn tag<T>(self) -> Self::Target<T> {
        Then {
            function: self.function,
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Debug for Then<I, C>
where
    I: Debug,
{
    /// Formats the continuation for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let function = "Box<dyn ThenFn>";
        f.debug_struct("Then")
            .field("function", &function)
            .field("marker", &self.marker)
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I, C> ThenFn<I> for (F, PhantomData<C>)
where
    F: FnOnce(Context<I, C>) -> Result<Steps<I, C>> + Send,
    C: Send,
{
    #[inline]
    fn execute(self: Box<Self>, ctx: Context<I>) -> Result<Steps<I>> {
        let (f, _) = *self;
        f(ctx.tag()).map(Steps::tag)
    }
}
