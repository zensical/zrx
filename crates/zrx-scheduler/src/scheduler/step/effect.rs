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

//! Effect.

use std::fmt::{self, Debug};

use crate::scheduler::engine::Tag;

pub mod task;
pub mod then;
pub mod timer;

pub use task::Task;
pub use then::Then;
pub use timer::Timer;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Effect.
///
/// Effects are the return values of [`Action::execute`][], which includes the
/// values [`Then`], [`Timer`] and [`Task`]. Actions can also return multiple
/// [`Steps`][], which can be conveniently created from any effect through
/// the [`IntoSteps`][] conversion trait.
///
/// This means that this type should never need to be created explicitly, as it
/// is created automatically through the conversion traits of effects.
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of effects inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
///
/// [`Action::execute`]: crate::scheduler::action::Action::execute
/// [`Steps`]: crate::scheduler::step::Steps
/// [`IntoSteps`]: crate::scheduler::step::IntoSteps
pub enum Effect<I, C = ()> {
    /// Continuation.
    Then(Then<I, C>),
    /// Timer.
    Timer(Timer<I, C>),
    /// Task.
    Task(Task<I, C>),
    /// Done.
    Done,
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, C> Tag<I, C> for Effect<I, C> {
    type Target<T> = Effect<I, T>;

    /// Tags the effect with the given type.
    #[inline]
    fn tag<T>(self) -> Self::Target<T> {
        match self {
            Effect::Then(then) => Effect::Then(then.tag()),
            Effect::Timer(timer) => Effect::Timer(timer.tag()),
            Effect::Task(task) => Effect::Task(task.tag()),
            Effect::Done => Effect::Done,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Debug for Effect<I, C>
where
    I: Debug,
    C: Debug,
{
    /// Formats the effect kind for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Effect::Then(then) => Debug::fmt(then, f),
            Effect::Timer(timer) => Debug::fmt(timer, f),
            Effect::Task(task) => Debug::fmt(task, f),
            Effect::Done => f.write_str("Done"),
        }
    }
}
