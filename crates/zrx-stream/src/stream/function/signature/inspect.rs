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

//! Inspect function.

use std::fmt::Display;

use zrx_scheduler::step::Result;
use zrx_scheduler::Scope;

use crate::stream::function::arguments::{
    ForId, ForIdSplat, ForIdValue, ForScope, ForScopeSplat, ForScopeValue,
    ForSplat, ForValue,
};
use crate::stream::function::catch;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Inspect function.
pub trait InspectFn<A, I, T>: Send + 'static {
    /// Executes the inspect function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, scope: &Scope<I>, value: &T) -> Result;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I, T> InspectFn<ForScope, I, T> for F
where
    F: Fn(&Scope<I>) -> Result + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>, _: &T) -> Result {
        catch(|| self(scope))
    }
}

impl<F, I, T> InspectFn<ForId, I, T> for F
where
    F: Fn(&I) -> Result + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>, _: &T) -> Result {
        catch(|| self(scope.try_as_id()?))
    }
}

impl<F, I, T> InspectFn<ForValue, I, T> for F
where
    F: Fn(&T) -> Result + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>, value: &T) -> Result {
        catch(|| self(value))
    }
}

impl<F, I, T> InspectFn<ForScopeValue, I, T> for F
where
    F: Fn(&Scope<I>, &T) -> Result + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>, value: &T) -> Result {
        catch(|| self(scope, value))
    }
}

impl<F, I, T> InspectFn<ForIdValue, I, T> for F
where
    F: Fn(&I, &T) -> Result + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Scope<I>, value: &T) -> Result {
        catch(|| self(scope.try_as_id()?, value))
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements inspect function trait for splat arguments.
macro_rules! impl_inspect_fn_for_splat {
    ($($T:ident),+) => {
        impl<F, I, $($T,)+> InspectFn<ForSplat, I, ($($T,)+)> for F
        where
            F: Fn($(&$T),+) -> Result + Send + 'static,
            I: Display,
        {
            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(scope = %scope)
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &Scope<I>, value: &($($T,)+)
            ) -> Result {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self($($T),+))
            }
        }
    };
}

/// Implements inspect function trait for scope and splat arguments.
macro_rules! impl_inspect_fn_for_scope_splat {
    ($($T:ident),+) => {
        impl<F, I, $($T,)+> InspectFn<ForScopeSplat, I, ($($T,)+)> for F
        where
            F: Fn(&Scope<I>, $(&$T),+) -> Result + Send + 'static,
            I: Display,
        {
            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(scope = %scope)
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &Scope<I>, value: &($($T,)+)
            ) -> Result {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope, $($T),+))
            }
        }
    };
}

/// Implements inspect function trait for identifier and splat arguments.
macro_rules! impl_inspect_fn_for_id_splat {
    ($($T:ident),+) => {
        impl<F, I, $($T,)+> InspectFn<ForIdSplat, I, ($($T,)+)> for F
        where
            F: Fn(&I, $(&$T),+) -> Result + Send + 'static,
            I: Display,
        {
            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(scope = %scope)
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &Scope<I>, value: &($($T,)+)
            ) -> Result {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope.try_as_id()?, $($T),+))
            }
        }
    };
}

/// Implements inspect function traits.
macro_rules! impl_inspect_fn {
    ($($T:ident),+) => {
        impl_inspect_fn_for_splat!($($T),+);
        impl_inspect_fn_for_scope_splat!($($T),+);
        impl_inspect_fn_for_id_splat!($($T),+);
    };
}

// ----------------------------------------------------------------------------

impl_inspect_fn!(T1);
impl_inspect_fn!(T1, T2);
impl_inspect_fn!(T1, T2, T3);
impl_inspect_fn!(T1, T2, T3, T4);
impl_inspect_fn!(T1, T2, T3, T4, T5);
impl_inspect_fn!(T1, T2, T3, T4, T5, T6);
impl_inspect_fn!(T1, T2, T3, T4, T5, T6, T7);
impl_inspect_fn!(T1, T2, T3, T4, T5, T6, T7, T8);
