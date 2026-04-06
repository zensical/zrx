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

//! Timer.

use std::time::{Duration, Instant};

use crate::scheduler::engine::Tag;
use crate::scheduler::step::Steps;

mod builder;
mod convert;

pub use builder::Builder;
pub use convert::{IntoDuration, IntoInstant};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Timer.
///
/// This data type is used to set, reset, or clear a timer. However, it doesn't
/// set a timer directly, but rather encapsulates a set of [`Effects`] together
/// with an instruction to the scheduler when and whether to handle the effects.
/// Thus, a [`Timer`] must be returned to be considered for activation.
///
/// In order to make creation of timers more convenient, this module provides
/// the [`IntoInstant`] and [`IntoDuration`] conversion traits, which can be
/// used to pass something that can be converted into a duration to an action,
/// and then be passed to the [`Timer`] methods.
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of effects inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
#[derive(Debug)]
pub enum Timer<I, C = ()> {
    /// Timer should be set, but not reset.
    ///
    /// If the scheduler already received a previous timer from an action, only
    /// the [`Effects`] will be replaced, if any, or set to nothing. This allows
    /// to implement operators like `audit` and `throttle`, where the time frame
    /// stays the same, but the [`Effects`] can change any time.
    ///
    /// [`Timer::Set`] is special in that once its data was set to [`None`], it
    /// won't accept any further data, which is required to block emissions.
    Set {
        /// Emission deadline.
        deadline: Instant,
        /// Step set to emit, if any.
        data: Option<Steps<I, C>>,
    },
    /// Timer should always be reset.
    ///
    /// This variant will always reset an existing timer, allowing to implement
    /// operators like `debounce`, where the latest emission is only emitted if
    /// there were no new emissions within the given time frame. [`Effects`] of
    /// previous emissions are dropped.
    Reset {
        /// Emission deadline.
        deadline: Instant,
        /// Step set to emit, if any.
        data: Option<Steps<I, C>>,
    },
    /// Timer should be repeated.
    ///
    /// This is equivalent to [`Timer::set`], except for that it requires the
    /// presence of a [`Duration`] instead of an [`Instant`], as the scheduler
    /// should automatically repeat the timer without interaction from actions.
    /// Additionally, the duration can be changed in a subsequent emission.
    Repeat {
        /// Emission interval.
        interval: Duration,
        /// Step set to emit, if any.
        data: Option<Steps<I, C>>,
    },
    /// Timer should be cleared.
    Clear,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, C> Timer<I, C> {
    /// Creates a timer that should be set, but not reset.
    #[inline]
    #[must_use]
    pub fn set<T>(deadline: T, data: Option<Steps<I, C>>) -> Self
    where
        T: IntoInstant,
    {
        Timer::Set {
            deadline: deadline.into_instant(),
            data,
        }
    }

    /// Creates a timer that should always be reset.
    #[inline]
    #[must_use]
    pub fn reset<T>(deadline: T, data: Option<Steps<I, C>>) -> Self
    where
        T: IntoInstant,
    {
        Timer::Reset {
            deadline: deadline.into_instant(),
            data,
        }
    }

    /// Creates a timer that should be repeated.
    #[inline]
    #[must_use]
    pub fn repeat<T>(interval: T, data: Option<Steps<I, C>>) -> Self
    where
        T: IntoDuration,
    {
        Timer::Repeat {
            interval: interval.into_duration(),
            data,
        }
    }

    /// Creates a timer that should be cleared.
    #[inline]
    #[must_use]
    pub fn clear() -> Self {
        Timer::Clear
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, C> Timer<I, C> {
    /// Returns a reference to the step set to emit, if any.
    #[inline]
    pub fn data(&self) -> Option<&Steps<I, C>> {
        match self {
            Timer::Set { data, .. } => data.as_ref(),
            Timer::Reset { data, .. } => data.as_ref(),
            Timer::Repeat { data, .. } => data.as_ref(),
            Timer::Clear => None,
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, C> Tag<I, C> for Timer<I, C> {
    type Target<T> = Timer<I, T>;

    /// Tags the timer with the given type.
    #[inline]
    fn tag<T>(self) -> Self::Target<T> {
        match self {
            Timer::Set { deadline, data } => {
                let data = data.map(Steps::tag);
                Timer::Set { deadline, data }
            }
            Timer::Reset { deadline, data } => {
                let data = data.map(Steps::tag);
                Timer::Reset { deadline, data }
            }
            Timer::Repeat { interval, data } => {
                let data = data.map(Steps::tag);
                Timer::Repeat { interval, data }
            }
            Timer::Clear => Timer::Clear,
        }
    }
}
