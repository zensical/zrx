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

//! Timer conversions.

use std::time::{Duration, Instant};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Instant`].
///
/// This is a convenience trait that allows for various types to be converted
/// into an [`Instant`], facilitating flexible timer specifications.
pub trait IntoInstant: 'static {
    /// Converts into an instant.
    fn into_instant(self) -> Instant;
}

// ----------------------------------------------------------------------------

/// Conversion into [`Duration`].
///
/// This is a convenience trait that allows for various types to be converted
/// into an [`Duration`], facilitating flexible timer specifications.
pub trait IntoDuration: 'static {
    /// Converts into a duration.
    fn into_duration(self) -> Duration;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl IntoInstant for Instant {
    /// Returns the instant as is.
    #[inline]
    fn into_instant(self) -> Instant {
        self
    }
}

// ----------------------------------------------------------------------------

impl IntoDuration for Duration {
    /// Returns the duration as is.
    #[inline]
    fn into_duration(self) -> Duration {
        self
    }
}

impl IntoDuration for u64 {
    /// Converts milliseconds into a duration.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::Duration;
    /// use zrx_scheduler::step::effect::timer::IntoDuration;
    ///
    /// // Create duration from milliseconds
    /// let duration = 250.into_duration();
    /// assert_eq!(duration, Duration::from_millis(250));
    /// ```
    #[inline]
    fn into_duration(self) -> Duration {
        Duration::from_millis(self)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> IntoInstant for T
where
    T: IntoDuration,
{
    /// Converts a duration into an instant.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::time::{Duration, Instant};
    /// use zrx_scheduler::step::effect::timer::IntoInstant;
    ///
    /// // Create instant from duration
    /// let instant = Duration::from_secs(1).into_instant();
    /// assert!(instant > Instant::now());
    /// ```
    #[inline]
    fn into_instant(self) -> Instant {
        Instant::now() + self.into_duration()
    }
}
