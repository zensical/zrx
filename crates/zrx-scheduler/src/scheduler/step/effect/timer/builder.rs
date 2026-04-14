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

//! Timer builder.

use crate::scheduler::signal::Id;
use crate::scheduler::step::effect::timer::{IntoDuration, IntoInstant};
use crate::scheduler::step::effect::{Effect, Timer};
use crate::scheduler::step::{Result, Scoped, Step, Steps};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Timer builder.
pub struct Builder<I, C> {
    /// Scope.
    scope: Scoped<I>,
    /// Timer steps.
    data: Option<Steps<I, C>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Scoped<I>
where
    I: Id,
{
    /// Creates a timer builder.
    #[must_use]
    pub fn timer<C>(&self) -> Builder<I, C> {
        Builder {
            scope: self.clone(),
            data: None,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Builder<I, C>
where
    I: Id,
{
    /// Sets the timer data.
    #[must_use]
    pub fn data(mut self, data: Steps<I, C>) -> Self {
        self.data = Some(data);
        self
    }

    /// Builds the timer with the given deadline.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    pub fn set<T>(self, deadline: T) -> Result<Steps<I, C>>
    where
        T: IntoInstant,
    {
        Ok(Steps::from(Step::new(
            self.scope,
            Effect::Timer(Timer::Set {
                deadline: deadline.into_instant(),
                data: self.data,
            }),
        )))
    }

    /// Builds the timer with the given deadline.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    pub fn reset<T>(self, deadline: T) -> Result<Steps<I, C>>
    where
        T: IntoInstant,
    {
        Ok(Steps::from(Step::new(
            self.scope,
            Effect::Timer(Timer::Reset {
                deadline: deadline.into_instant(),
                data: self.data,
            }),
        )))
    }

    /// Builds the timer with the given interval.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    pub fn repeat<T>(self, interval: T) -> Result<Steps<I, C>>
    where
        T: IntoDuration,
    {
        Ok(Steps::from(Step::new(
            self.scope,
            Effect::Timer(Timer::Repeat {
                interval: interval.into_duration(),
                data: self.data,
            }),
        )))
    }

    /// Clears the timer.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    pub fn clear(self) -> Result<Steps<I, C>> {
        Ok(Steps::from(Step::new(
            self.scope,
            Effect::Timer(Timer::Clear),
        )))
    }
}
