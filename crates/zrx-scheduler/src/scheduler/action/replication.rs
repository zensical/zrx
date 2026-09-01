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

//! Replication policy for user-provided action functions.

use std::num::NonZeroUsize;
use std::ops::Deref;

use super::Concurrency;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into a replication policy with a known function type.
pub trait IntoReplication {
    /// Wrapped function type.
    type Target;

    /// Converts into an explicit function replication policy.
    fn into_replication(self) -> Replication<Self::Target>;
}

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum Policy {
    Bounded(NonZeroUsize),
    Adaptive,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// User-provided function plus its permitted replication policy.
pub struct Replication<F> {
    inner: F,
    policy: Policy,
    replica: Option<fn(&F) -> F>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<F> Replication<F> {
    /// Separates the function from its scheduler installation policy.
    ///
    /// The tuple deliberately avoids introducing another policy wrapper into
    /// stream operator bounds.
    #[allow(clippy::type_complexity)]
    #[inline]
    #[must_use]
    pub fn into_parts(self) -> (F, Option<NonZeroUsize>, Option<fn(&F) -> F>) {
        let maximum = match self.policy {
            Policy::Bounded(maximum) => Some(maximum),
            Policy::Adaptive => None,
        };
        (self.inner, maximum, self.replica)
    }

    /// Creates an independent function replica.
    ///
    /// # Panics
    ///
    /// Panics if replication is disabled.
    #[inline]
    #[must_use]
    pub fn replicate(&self) -> Self {
        let replica = self
            .replica
            .expect("sequential action was asked to create a replica");
        Self {
            inner: replica(&self.inner),
            policy: self.policy,
            replica: self.replica,
        }
    }

    /// Converts function replication into action concurrency.
    #[inline]
    #[must_use]
    pub fn concurrency<A>(&self, replicate: fn(&A) -> A) -> Concurrency<A> {
        match self.policy {
            Policy::Bounded(maximum) if self.replica.is_none() => {
                debug_assert_eq!(maximum, NonZeroUsize::MIN);
                Concurrency::default()
            }
            Policy::Bounded(maximum) => {
                Concurrency::bounded_with(maximum, replicate)
            }
            Policy::Adaptive => Concurrency::adaptive_with(replicate),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<F> Deref for Replication<F> {
    type Target = F;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// ----------------------------------------------------------------------------

impl<F> IntoReplication for F
where
    F: Clone,
{
    type Target = F;

    #[inline]
    fn into_replication(self) -> Replication<Self::Target> {
        self.into()
    }
}

// ----------------------------------------------------------------------------

impl<F> IntoReplication for Replication<F> {
    type Target = F;

    #[inline]
    fn into_replication(self) -> Replication<Self::Target> {
        self
    }
}

impl<F> From<F> for Replication<F>
where
    F: Clone,
{
    #[inline]
    fn from(inner: F) -> Self {
        Self {
            inner,
            policy: Policy::Adaptive,
            replica: Some(Clone::clone),
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Wraps a function whose invocations must not overlap.
#[inline]
#[must_use]
pub const fn sequential<F>(inner: F) -> Replication<F> {
    Replication {
        inner,
        policy: Policy::Bounded(NonZeroUsize::MIN),
        replica: None,
    }
}

/// Wraps a function that permits at most `maximum` concurrent action instances.
///
/// # Panics
///
/// Panics if `maximum` is zero.
#[inline]
#[must_use]
pub fn concurrent<F>(maximum: usize, inner: F) -> Replication<F>
where
    F: Clone,
{
    Replication {
        inner,
        policy: Policy::Bounded(
            NonZeroUsize::new(maximum)
                .expect("function replication must be non-zero"),
        ),
        replica: Some(Clone::clone),
    }
}
