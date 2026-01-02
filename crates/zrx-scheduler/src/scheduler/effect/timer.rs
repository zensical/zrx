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

use crate::scheduler::action::Outputs;

mod convert;

pub use convert::{IntoDuration, IntoInstant};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Timer.
///
/// This data type is used to set, reset, or clear a timer. However, it doesn't
/// set a timer directly, but rather encapsulates a set of [`Outputs`] together
/// with an instruction to the scheduler when and whether to handle the outputs.
/// Thus, a [`Timer`] must be returned to be considered for activation.
///
/// In order to make creation of timers more convenient, this module provides
/// the [`IntoInstant`] and [`IntoDuration`] conversion traits, which can be
/// used to pass something that can be converted into a duration to an action,
/// and then be passed to the [`Timer`] methods.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::action::Outputs;
/// use zrx_scheduler::effect::{Item, Timer};
///
/// // Create outputs
/// let outputs = Outputs::from(Item::new("id", Some(42)));
///
/// // Create timer to delay outputs by 100ms
/// let timer = Timer::set(100, Some(outputs));
/// ```
#[derive(Debug)]
pub enum Timer<I> {
    /// Timer should be set, but not reset.
    ///
    /// If the scheduler already received a previous timer from an action, only
    /// the [`Outputs`] will be replaced, if any, or set to nothing. This allows
    /// to implement operators like `audit` and `throttle`, where the time frame
    /// stays the same, but the [`Outputs`] can change any time.
    ///
    /// [`Timer::Set`] is special in that once its data was set to [`None`], it
    /// won't accept any further data, which is required to block emissions.
    Set {
        /// Emission deadline.
        deadline: Instant,
        /// Outputs to emit, if any.
        data: Option<Outputs<I>>,
    },

    /// Timer should always be reset.
    ///
    /// This variant will always reset an existing timer, allowing to implement
    /// operators like `debounce`, where the latest emission is only emitted if
    /// there were no new emissions within the given time frame. [`Outputs`] of
    /// previous emissions are dropped.
    Reset {
        /// Emission deadline.
        deadline: Instant,
        /// Outputs to emit, if any.
        data: Option<Outputs<I>>,
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
        /// Outputs to emit, if any.
        data: Option<Outputs<I>>,
    },

    /// Timer should be cleared.
    Clear,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Timer<I> {
    /// Creates a timer that should be set, but not reset.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::{Item, Timer};
    ///
    /// // Create outputs
    /// let outputs = Outputs::from(Item::new("id", Some(42)));
    ///
    /// // Create timer to delay outputs by 100ms
    /// let timer = Timer::set(100, Some(outputs));
    /// ```
    #[inline]
    #[must_use]
    pub fn set<T>(deadline: T, data: Option<Outputs<I>>) -> Self
    where
        T: IntoInstant,
    {
        Timer::Set {
            deadline: deadline.into_instant(),
            data,
        }
    }

    /// Creates a timer that should always be reset.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::{Item, Timer};
    ///
    /// // Create outputs
    /// let outputs = Outputs::from(Item::new("id", Some(42)));
    ///
    /// // Create timer to delay outputs by 100ms
    /// let timer = Timer::reset(100, Some(outputs));
    /// ```
    #[inline]
    #[must_use]
    pub fn reset<T>(deadline: T, data: Option<Outputs<I>>) -> Self
    where
        T: IntoInstant,
    {
        Timer::Reset {
            deadline: deadline.into_instant(),
            data,
        }
    }

    /// Creates a timer that should be repeated.
    ///
    /// Note that the given [`Outputs`] are emitted on the first activation, so
    /// after the timer becomes active for the first time, they are reset. In
    /// order to emit outputs in the next activation, a new timer has to be
    /// created. Additionally, the duration can be changed subsequently.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::{Item, Timer};
    ///
    /// // Create outputs
    /// let outputs = Outputs::from(Item::new("id", Some(42)));
    ///
    /// // Create repeating timer to delay outputs by 100ms
    /// let timer = Timer::repeat(100, Some(outputs));
    /// ```
    #[inline]
    #[must_use]
    pub fn repeat<T>(interval: T, data: Option<Outputs<I>>) -> Self
    where
        T: IntoDuration,
    {
        Timer::Repeat {
            interval: interval.into_duration(),
            data,
        }
    }

    /// Creates a timer that should be cleared.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::effect::Timer;
    ///
    /// // Create clearing timer
    /// let timer = Timer::clear();
    /// # let _: Timer<()> = timer;
    /// ```
    #[inline]
    #[must_use]
    pub fn clear() -> Self {
        Timer::Clear
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Timer<I> {
    /// Returns a reference to the outputs to emit, if any.
    #[allow(clippy::match_same_arms)]
    #[inline]
    pub fn data(&self) -> Option<&Outputs<I>> {
        match self {
            Timer::Set { data, .. } => data.as_ref(),
            Timer::Reset { data, .. } => data.as_ref(),
            Timer::Repeat { data, .. } => data.as_ref(),
            Timer::Clear => None,
        }
    }
}
