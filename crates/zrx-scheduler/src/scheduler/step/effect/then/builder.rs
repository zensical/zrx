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

//! Continuation builder.

use std::marker::PhantomData;

use crate::scheduler::action::Context;
use crate::scheduler::signal::Id;
use crate::scheduler::step::effect::Effect;
use crate::scheduler::step::{Result, Scope, Step, Steps};

use super::Then;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Continuation builder.
pub struct Builder<I, C> {
    /// Scope.
    scope: Scope<I>,
    /// Capture types.
    marker: PhantomData<C>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Scope<I>
where
    I: Id,
{
    /// Creates a continuation builder.
    #[must_use]
    pub fn then<C>(&mut self) -> Builder<I, C> {
        Builder {
            scope: self.take(),
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Builder<I, C>
where
    I: Id,
{
    /// Builds the continuation with the given function.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    pub fn build<F>(self, f: F) -> Result<Steps<I, C>>
    where
        F: FnOnce(Context<I, C>) -> Result<Steps<I, C>> + Send + 'static,
        C: Send + 'static,
    {
        Ok(Steps::from(Step::new(
            self.scope,
            Effect::Then(Then {
                function: Box::new((f, PhantomData)),
                marker: PhantomData,
            }),
        )))
    }
}
