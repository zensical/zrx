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

//! Reduce function.

use std::fmt::Display;

use zrx_scheduler::action::Result;
use zrx_scheduler::action::error::{IntoResult, catch};
use zrx_store::Collection;

use crate::stream::Key;
use crate::stream::function::Scope;
use crate::stream::function::arguments::{WithScopeValue, WithValue};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Reduce function.
pub trait ReduceFn<A, I, T, U>: Send + 'static {
    /// Computes the current aggregate of one complete group.
    ///
    /// # Errors
    ///
    /// Returns a declared callback error or a caught callback panic.
    fn execute(
        &self, scope: &mut Scope<I>, members: &dyn Collection<Key<I>, T>,
    ) -> Result<Option<U>>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I, T, U> ReduceFn<WithScopeValue, I, T, U> for F
where
    F: Fn(&mut Scope<I>, &dyn Collection<Key<I>, T>) -> R + Send + 'static,
    R: IntoResult<Option<U>>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(
        &self, scope: &mut Scope<I>, members: &dyn Collection<Key<I>, T>,
    ) -> Result<Option<U>> {
        catch(|| self(scope, members).into_result())
    }
}

impl<F, R, I, T, U> ReduceFn<WithValue, I, T, U> for F
where
    F: Fn(&dyn Collection<Key<I>, T>) -> R + Send + 'static,
    R: IntoResult<Option<U>>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(
        &self, scope: &mut Scope<I>, members: &dyn Collection<Key<I>, T>,
    ) -> Result<Option<U>> {
        catch(|| self(members).into_result())
    }
}
