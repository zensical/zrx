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

//! Context builder.

use std::marker::PhantomData;

use zrx_storage::set::{View, ViewMut};

use crate::scheduler::signal::Id;
use crate::scheduler::step::Scope;

use super::{Context, Event};

mod error;

pub use error::{Error, Result};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Context builder.
#[derive(Debug)]
pub struct Builder<'a, I> {
    /// Events.
    events: Vec<Event<I>>,
    /// Input storages.
    inputs: Option<View<'a>>,
    /// Output storage.
    output: Option<ViewMut<'a>>,
    /// Capture types.
    marker: PhantomData<I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, I> Context<'a, I> {
    /// Creates a context builder.
    #[inline]
    #[must_use]
    pub fn builder() -> Builder<'a, I> {
        Builder::default()
    }
}

// ----------------------------------------------------------------------------

impl<'a, I> Builder<'a, I>
where
    I: Id,
{
    /// Sets the events.
    #[must_use]
    pub fn events(mut self, events: Vec<Event<I>>) -> Self {
        self.events = events;
        self
    }

    /// Sets the view to obtain the input storages.
    #[must_use]
    pub fn inputs(mut self, view: View<'a>) -> Self {
        self.inputs = Some(view);
        self
    }

    /// Sets the view to obtain the output storage.
    #[must_use]
    pub fn output(mut self, view: ViewMut<'a>) -> Self {
        self.output = Some(view);
        self
    }

    /// Builds the context.
    ///
    /// # Errors
    ///
    /// Returns an error if the input or output view is not set.
    pub fn build<T, C>(self, scopes: T) -> Result<Context<'a, I, C>>
    where
        T: IntoIterator<Item = Scope<I>>,
    {
        Ok(Context {
            events: self.events,
            scopes: scopes.into_iter().collect(),
            inputs: self.inputs.ok_or(Error::Inputs)?,
            output: self.output.ok_or(Error::Output)?,
            marker: PhantomData,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Default for Builder<'_, I> {
    /// Creates a context builder.
    #[inline]
    fn default() -> Self {
        Self {
            events: Vec::default(),
            inputs: None,
            output: None,
            marker: PhantomData,
        }
    }
}
