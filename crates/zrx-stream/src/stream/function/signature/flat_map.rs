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

//! Flat map function.

use std::fmt::Display;

use zrx_scheduler::step::error::IntoResult;
use zrx_scheduler::step::{Result, Scope};
use zrx_scheduler::Key;

use crate::stream::function::arguments::{
    ForId, ForIdSplat, ForIdValue, ForKey, ForKeySplat, ForKeyValue, ForScope,
    ForScopeSplat, ForScopeValue, ForSplat, ForValue,
};
use crate::stream::function::catch;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Flat map function.
pub trait FlatMapFn<A, J, I, T, U>: Send + 'static {
    /// Iterator type returned by this function.
    type Iter: IntoIterator<Item = (Key<I>, U)>;

    /// Executes the flat map function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, scope: &mut Scope<I>, value: T) -> Result<Self::Iter>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, J, I, T, U> FlatMapFn<ForScope, J, I, T, U> for F
where
    F: Fn(&mut Scope<I>) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, _: T) -> Result<J> {
        catch(|| self(scope).into_result())
    }
}

impl<F, R, J, I, T, U> FlatMapFn<ForScopeValue, J, I, T, U> for F
where
    F: Fn(&mut Scope<I>, T) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: T) -> Result<J> {
        catch(|| self(scope, value).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, J, I, T, U> FlatMapFn<ForKey, J, I, T, U> for F
where
    F: Fn(&Key<I>) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, _: T) -> Result<J> {
        catch(|| self(scope.key()).into_result())
    }
}

impl<F, R, J, I, T, U> FlatMapFn<ForKeyValue, J, I, T, U> for F
where
    F: Fn(&Key<I>, T) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: T) -> Result<J> {
        catch(|| self(scope.key(), value).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, J, I, T, U> FlatMapFn<ForId, J, I, T, U> for F
where
    F: Fn(&I) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, _: T) -> Result<J> {
        catch(|| self(scope.key().try_as_id()?).into_result())
    }
}

impl<F, R, J, I, T, U> FlatMapFn<ForIdValue, J, I, T, U> for F
where
    F: Fn(&I, T) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: T) -> Result<J> {
        catch(|| self(scope.key().try_as_id()?, value).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, J, I, T, U> FlatMapFn<ForValue, J, I, T, U> for F
where
    F: Fn(T) -> R + Send + 'static,
    R: IntoResult<J>,
    J: IntoIterator<Item = (Key<I>, U)>,
    I: Display,
{
    type Iter = J;

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: T) -> Result<J> {
        catch(|| self(value).into_result())
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

macro_rules! impl_flat_map_fn_for_scope_splat {
    ($($T:ident),+) => {
        impl<F, R, J, I, $($T,)+ U> FlatMapFn<ForScopeSplat, J, I, ($($T,)+), U>
            for F
        where
            F: Fn(&mut Scope<I>, $($T),+) -> R + Send + 'static,
            R: IntoResult<J>,
            J: IntoIterator<Item = (Key<I>, U)>,
            I: Display,
        {
            type Iter = J;

            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(key = %scope.key())
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &mut Scope<I>, value: ($($T,)+),
            ) -> Result<J> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope, $($T),+).into_result())
            }
        }
    };
}

macro_rules! impl_flat_map_fn_for_key_splat {
    ($($T:ident),+) => {
        impl<F, R, J, I, $($T,)+ U> FlatMapFn<ForKeySplat, J, I, ($($T,)+), U>
            for F
        where
            F: Fn(&Key<I>, $($T),+) -> R + Send + 'static,
            R: IntoResult<J>,
            J: IntoIterator<Item = (Key<I>, U)>,
            I: Display,
        {
            type Iter = J;

            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(key = %scope.key())
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &mut Scope<I>, value: ($($T,)+),
            ) -> Result<J> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope.key(), $($T),+).into_result())
            }
        }
    };
}

macro_rules! impl_flat_map_fn_for_id_splat {
    ($($T:ident),+) => {
        impl<F, R, J, I, $($T,)+ U> FlatMapFn<ForIdSplat, J, I, ($($T,)+), U>
            for F
        where
            F: Fn(&I, $($T),+) -> R + Send + 'static,
            R: IntoResult<J>,
            J: IntoIterator<Item = (Key<I>, U)>,
            I: Display,
        {
            type Iter = J;

            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(key = %scope.key())
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &mut Scope<I>, value: ($($T,)+),
            ) -> Result<J> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope.key().try_as_id()?, $($T),+).into_result())
            }
        }
    };
}

macro_rules! impl_flat_map_fn_for_splat {
    ($($T:ident),+) => {
        impl<F, R, J, I, $($T,)+ U> FlatMapFn<ForSplat, J, I, ($($T,)+), U>
            for F
        where
            F: Fn($($T),+) -> R + Send + 'static,
            R: IntoResult<J>,
            J: IntoIterator<Item = (Key<I>, U)>,
            I: Display,
        {
            type Iter = J;

            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(
                    level = "debug", skip_all, fields(key = %scope.key())
                )
            )]
            #[inline]
            fn execute(
                &self, scope: &mut Scope<I>, value: ($($T,)+),
            ) -> Result<J> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self($($T),+).into_result())
            }
        }
    };
}

// ----------------------------------------------------------------------------

/// Implements flat map function traits.
macro_rules! impl_flat_map_fn {
    ($($T:ident),+) => {
        impl_flat_map_fn_for_scope_splat!($($T),+);
        impl_flat_map_fn_for_key_splat!($($T),+);
        impl_flat_map_fn_for_id_splat!($($T),+);
        impl_flat_map_fn_for_splat!($($T),+);
    };
}

// ----------------------------------------------------------------------------

impl_flat_map_fn!(T1);
impl_flat_map_fn!(T1, T2);
impl_flat_map_fn!(T1, T2, T3);
impl_flat_map_fn!(T1, T2, T3, T4);
impl_flat_map_fn!(T1, T2, T3, T4, T5);
impl_flat_map_fn!(T1, T2, T3, T4, T5, T6);
impl_flat_map_fn!(T1, T2, T3, T4, T5, T6, T7);
impl_flat_map_fn!(T1, T2, T3, T4, T5, T6, T7, T8);
