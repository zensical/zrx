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

use zrx_scheduler::action::Result;
use zrx_scheduler::action::error::{IntoResult, catch};

use crate::stream::Key;
use crate::stream::function::Scope;
use crate::stream::function::arguments::{
    Arguments, WithId, WithIdSplat, WithIdValue, WithKey, WithKeySplat,
    WithKeyValue, WithScope, WithScopeSplat, WithScopeValue, WithSplat,
    WithValue,
};

use super::Signature;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Map function.
pub trait MapFn<A, I, T, U = ()>: Send + 'static {
    /// Invokes one user callback for one input value.
    ///
    /// # Errors
    ///
    /// Returns a declared callback error or a caught callback panic.
    fn execute(&self, scope: &mut Scope<I>, value: &T) -> Result<U>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<F, A, I, T, U> MapFn<A, I, T, U> for Signature<F, A>
where
    F: MapFn<A, I, T, U>,
    A: Arguments,
{
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: &T) -> Result<U> {
        (**self).execute(scope, value)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I, T, U> MapFn<WithScope, I, T, U> for F
where
    F: Fn(&mut Scope<I>) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, _: &T) -> Result<U> {
        catch(|| self(scope).into_result())
    }
}

impl<F, R, I, T, U> MapFn<WithScopeValue, I, T, U> for F
where
    F: Fn(&mut Scope<I>, &T) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: &T) -> Result<U> {
        catch(|| self(scope, value).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, I, T, U> MapFn<WithKey, I, T, U> for F
where
    F: Fn(&Key<I>) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, _: &T) -> Result<U> {
        catch(|| self(scope.key()).into_result())
    }
}

impl<F, R, I, T, U> MapFn<WithKeyValue, I, T, U> for F
where
    F: Fn(&Key<I>, &T) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: &T) -> Result<U> {
        catch(|| self(scope.key(), value).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, I, T, U> MapFn<WithId, I, T, U> for F
where
    F: Fn(&I) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, _: &T) -> Result<U> {
        catch(|| self(scope.key().try_as_id()?).into_result())
    }
}

impl<F, R, I, T, U> MapFn<WithIdValue, I, T, U> for F
where
    F: Fn(&I, &T) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: &T) -> Result<U> {
        catch(|| self(scope.key().try_as_id()?, value).into_result())
    }
}

// ----------------------------------------------------------------------------

impl<F, R, I, T, U> MapFn<WithValue, I, T, U> for F
where
    F: Fn(&T) -> R + Send + 'static,
    R: IntoResult<U>,
    I: Display,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(
            level = "debug", skip_all, fields(key = %scope.key())
        )
    )]
    #[inline]
    fn execute(&self, scope: &mut Scope<I>, value: &T) -> Result<U> {
        catch(|| self(value).into_result())
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements map function trait for scope and splat arguments.
macro_rules! impl_map_fn_for_scope_splat {
    ($($T:ident),+) => {
        impl<F, R, I, $($T,)+ U> MapFn<WithScopeSplat, I, ($($T,)+), U> for F
        where
            F: Fn(&mut Scope<I>, $(&$T),+) -> R + Send + 'static,
            R: IntoResult<U>,
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
                &self, scope: &mut Scope<I>, value: &($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope, $($T),+).into_result())
            }
        }
    };
}

/// Implements map function trait for key and splat arguments.
macro_rules! impl_map_fn_for_key_splat {
    ($($T:ident),+) => {
        impl<F, R, I, $($T,)+ U> MapFn<WithKeySplat, I, ($($T,)+), U> for F
        where
            F: Fn(&Key<I>, $(&$T),+) -> R + Send + 'static,
            R: IntoResult<U>,
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
                &self, scope: &mut Scope<I>, value: &($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope.key(), $($T),+).into_result())
            }
        }
    };
}

/// Implements map function trait for identifier and splat arguments.
macro_rules! impl_map_fn_for_id_splat {
    ($($T:ident),+) => {
        impl<F, R, I, $($T,)+ U> MapFn<WithIdSplat, I, ($($T,)+), U> for F
        where
            F: Fn(&I, $(&$T),+) -> R + Send + 'static,
            R: IntoResult<U>,
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
                &self, scope: &mut Scope<I>, value: &($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self(scope.key().try_as_id()?, $($T),+).into_result())
            }
        }
    };
}

/// Implements map function trait for splat arguments.
macro_rules! impl_map_fn_for_splat {
    ($($T:ident),+) => {
        impl<F, R, I, $($T,)+ U> MapFn<WithSplat, I, ($($T,)+), U> for F
        where
            F: Fn($(&$T),+) -> R + Send + 'static,
            R: IntoResult<U>,
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
                &self, scope: &mut Scope<I>, value: &($($T,)+)
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = value;
                catch(|| self($($T),+).into_result())
            }
        }
    };
}

// ----------------------------------------------------------------------------

/// Implements map function traits.
macro_rules! impl_map_fn {
    ($($T:ident),+) => {
        impl_map_fn_for_scope_splat!($($T),+);
        impl_map_fn_for_key_splat!($($T),+);
        impl_map_fn_for_id_splat!($($T),+);
        impl_map_fn_for_splat!($($T),+);
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
