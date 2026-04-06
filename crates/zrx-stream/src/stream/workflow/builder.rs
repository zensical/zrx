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

//! Workflow builder.

use std::error::Error;
use std::marker::PhantomData;

use zrx_scheduler::schedule::{self, Shared};
use zrx_scheduler::{Id, Value};

use crate::stream::Stream;

use super::Workflow;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Workflow builder.
#[derive(Debug, PartialEq, Eq)]
pub struct Builder<I> {
    /// Schedule builder.
    schedule: Shared<schedule::Builder<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Workflow<I>
where
    I: Id,
{
    /// Creates a workflow builder.
    ///
    /// If possible, it's recommended to use [`Workflow::with`] instead, as it
    /// creates a [`Builder`] which is only valid for the function lifetime.
    #[inline]
    #[must_use]
    pub fn builder() -> Builder<I> {
        Builder::default()
    }

    /// Creates a workflow from the builder passed to the given function.
    ///
    /// # Errors
    ///
    /// If the given function returns an error, it is returned.
    #[allow(clippy::missing_panics_doc)]
    pub fn with<F, E>(f: F) -> Result<Self, E>
    where
        F: FnOnce(Builder<I>) -> Result<(), E>,
        E: Error,
    {
        let builder = Builder::default();
        f(builder.clone()).map(|()| {
            let Builder { schedule } = builder;
            let builder = schedule.try_into_inner().expect("invariant");
            // We can safely use expect here, since the builder can't be cloned
            // in the function, and is thus guaranteed to be the only reference
            Self { schedule: builder.build() }
        })
    }
}

// ----------------------------------------------------------------------------

impl<I> Builder<I>
where
    I: Id,
{
    /// Adds a stream to the workflow.
    #[inline]
    pub fn add<T>(&self) -> Stream<I, T>
    where
        T: Value,
    {
        Stream {
            id: self.schedule.with_mut(schedule::Builder::add_source::<T>),
            workflow: self.clone(),
            marker: PhantomData,
        }
    }

    /// Attempts to unwrap the inner value.
    ///
    /// This method tries to unwrap the inner schedule builder, which will only
    /// succeed if there's exactly one strong reference remaining.
    ///
    /// # Errors
    ///
    /// This method returns `Self` if there's more than one strong reference.
    pub fn build(self) -> Result<Workflow<I>, Self> {
        self.schedule
            .try_into_inner()
            .map(|builder| Workflow { schedule: builder.build() })
            .map_err(|schedule| Self { schedule })
    }

    /// Mutably borrows the schedule builder.
    pub(crate) fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut schedule::Builder<I>) -> R,
    {
        self.schedule.with_mut(f)
    }
}

impl<I> Builder<I> {
    /// Clones the builder.
    pub(crate) fn clone(&self) -> Self {
        Self {
            schedule: self.schedule.clone(),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Default for Builder<I> {
    /// Creates a workflow builder.
    #[inline]
    fn default() -> Self {
        Self { schedule: Shared::default() }
    }
}
