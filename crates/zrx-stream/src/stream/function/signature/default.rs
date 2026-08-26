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

//! Default function.

use std::fmt::Display;

use zrx_scheduler::Key;
use zrx_scheduler::step::error::IntoResult;
use zrx_scheduler::step::{Result, Scope};

use crate::stream::function::arguments::{ForId, ForKey, ForScope, ForValue};
use crate::stream::function::catch;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Default function.
pub trait DefaultFn<A, I, T>: Send + 'static {
    /// Executes the default function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, scope: &mut Scope<I>) -> Result<Option<T>>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I, T> DefaultFn<ForScope, I, T> for F
where
    F: Fn(&mut Scope<I>) -> R + Send + 'static,
    R: IntoResult<Option<T>>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>) -> Result<Option<T>> {
        catch(|| self(scope).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, I, T> DefaultFn<ForKey, I, T> for F
where
    F: Fn(&Key<I>) -> R + Send + 'static,
    R: IntoResult<Option<T>>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>) -> Result<Option<T>> {
        catch(|| self(scope.key()).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, I, T> DefaultFn<ForId, I, T> for F
where
    F: Fn(&I) -> R + Send + 'static,
    R: IntoResult<Option<T>>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>) -> Result<Option<T>> {
        catch(|| self(scope.key().try_as_id()?).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, I, T> DefaultFn<ForValue, I, T> for F
where
    F: Fn() -> R + Send + 'static,
    R: IntoResult<Option<T>>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>) -> Result<Option<T>> {
        catch(|| self().into_result())
    }
}
