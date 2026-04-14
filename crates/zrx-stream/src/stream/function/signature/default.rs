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

//! Filter map function.

use std::fmt::Display;

use zrx_scheduler::step::Result;
use zrx_scheduler::Scope;

use crate::stream::function::arguments::{ForId, ForScope, ForValue};
use crate::stream::function::catch;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Default function.
pub trait DefaultFn<A, I, T>: Send + 'static {
    /// Executes the map function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, scope: &Scope<I>) -> Result<Option<T>>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I, T> DefaultFn<ForScope, I, T> for F
where
    F: Fn(&Scope<I>) -> Result<Option<T>> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>) -> Result<Option<T>> {
        catch(|| self(scope))
    }
}

impl<F, I, T> DefaultFn<ForId, I, T> for F
where
    F: Fn(&I) -> Result<Option<T>> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>) -> Result<Option<T>> {
        catch(|| self(scope.try_as_id()?))
    }
}

impl<F, I, T> DefaultFn<ForValue, I, T> for F
where
    F: Fn() -> Result<Option<T>> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>) -> Result<Option<T>> {
        catch(self)
    }
}
