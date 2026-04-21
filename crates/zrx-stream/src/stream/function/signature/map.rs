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

//! Map function.

use std::fmt::Display;

use zrx_scheduler::step::Result;
use zrx_scheduler::Key;

use crate::stream::function::arguments::{
    ForId, ForIdSplat, ForIdValue, ForKey, ForKeySplat, ForKeyValue, ForSplat,
    ForValue,
};
use crate::stream::function::catch;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Map function.
pub trait MapFn<A, I, T, U>: Send + 'static {
    /// Executes the map function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, scope: &Key<I>, value: T) -> Result<U>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I, T, U> MapFn<ForKey, I, T, U> for F
where
    F: Fn(&Key<I>) -> Result<U> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Key<I>, _: T) -> Result<U> {
        catch(|| self(scope))
    }
}

impl<F, I, T, U> MapFn<ForId, I, T, U> for F
where
    F: Fn(&I) -> Result<U> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Key<I>, _: T) -> Result<U> {
        catch(|| self(scope.try_as_id()?))
    }
}

impl<F, I, T, U> MapFn<ForValue, I, T, U> for F
where
    F: Fn(T) -> Result<U> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Key<I>, value: T) -> Result<U> {
        catch(|| self(value))
    }
}

impl<F, I, T, U> MapFn<ForKeyValue, I, T, U> for F
where
    F: Fn(&Key<I>, T) -> Result<U> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Key<I>, value: T) -> Result<U> {
        catch(|| self(scope, value))
    }
}

impl<F, I, T, U> MapFn<ForIdValue, I, T, U> for F
where
    F: Fn(&I, T) -> Result<U> + Send + 'static,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(scope = %scope))
    )]
    #[inline]
    fn execute(&self, scope: &Key<I>, value: T) -> Result<U> {
        catch(|| self(scope.try_as_id()?, value))
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements map function trait for splat arguments.
macro_rules! impl_map_fn_for_splat {
    ($($T:ident),+) => {
        impl<F, I, $($T,)+ U> MapFn<ForSplat, I, ($($T,)+), U> for F
        where
            F: Fn($($T),+) -> Result<U> + Send + 'static,
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
                &self, scope: &Key<I>, value: ($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self($($T),+))
            }
        }
    };
}

/// Implements map function trait for scope and splat arguments.
macro_rules! impl_map_fn_for_scope_splat {
    ($($T:ident),+) => {
        impl<F, I, $($T,)+ U> MapFn<ForKeySplat, I, ($($T,)+), U> for F
        where
            F: Fn(&Key<I>, $($T),+) -> Result<U> + Send + 'static,
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
                &self, scope: &Key<I>, value: ($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope, $($T),+))
            }
        }
    };
}

/// Implements map function trait for identifier and splat arguments.
macro_rules! impl_map_fn_for_id_splat {
    ($($T:ident),+) => {
        impl<F, I, $($T,)+ U> MapFn<ForIdSplat, I, ($($T,)+), U> for F
        where
            F: Fn(&I, $($T),+) -> Result<U> + Send + 'static,
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
                &self, scope: &Key<I>, value: ($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope.try_as_id()?, $($T),+))
            }
        }
    };
}

/// Implements map function traits.
macro_rules! impl_map_fn {
    ($($T:ident),+) => {
        impl_map_fn_for_splat!($($T),+);
        impl_map_fn_for_scope_splat!($($T),+);
        impl_map_fn_for_id_splat!($($T),+);
    };
}

// ----------------------------------------------------------------------------

impl_map_fn!(T1);
impl_map_fn!(T1, T2);
impl_map_fn!(T1, T2, T3);
impl_map_fn!(T1, T2, T3, T4);
impl_map_fn!(T1, T2, T3, T4, T5);
impl_map_fn!(T1, T2, T3, T4, T5, T6);
impl_map_fn!(T1, T2, T3, T4, T5, T6, T7);
impl_map_fn!(T1, T2, T3, T4, T5, T6, T7, T8);
