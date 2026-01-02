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

use zrx_scheduler::action::report::IntoReport;
use zrx_scheduler::action::Result;
use zrx_scheduler::Value;

use crate::stream::function::adapter::{WithId, WithSplat};
use crate::stream::function::{catch, Splat};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Map function.
///
/// This trait defines a function that transforms data of type `T` into data of
/// type `U`. It's the most commonly used function in stream processing, as it
/// is accepted by multiple operators. Note that transformations are expected
/// to be immutable, meaning they can't capture mutable variables. This allows
/// operators to move function execution to a [`Task`][], which is also why
/// it expects owned data.
///
/// There's a range of different implementations of this trait, allowing you to
/// use a variety of function shapes, including support for [`Splat`], as well
/// as support for the [`WithId`] and [`WithSplat`] adapters. Furthermore, the
/// trait can be implemented for custom types to add new behaviors. Note that
/// all implementations also allow to return a [`Report`][], which makes it
/// possible to return diagnostics from the function execution.
///
/// The `'static` lifetimes is mandatory as closures must be moved into actions,
/// so requiring it here allows us to reduce the verbosity of trait bounds.
///
/// [`Report`]: zrx_scheduler::action::Report
/// [`Task`]: zrx_scheduler::effect::Task
///
/// # Examples
///
/// Transform data:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_stream::function::MapFn;
///
/// // Define and execute function
/// let f = |n: i32| n * n;
/// f.execute(&"id", 42)?;
/// # Ok(())
/// # }
/// ```
///
/// Transform data with splat argument:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_stream::function::{MapFn, Splat};
///
/// // Define and execute function
/// let f = |a: i32, b: i32| a + b;
/// f.execute(&"id", Splat::from((1, 2)))?;
/// # Ok(())
/// # }
/// ```
pub trait MapFn<I, T, U>: Send + 'static {
    /// Executes the map function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, id: &I, data: T) -> Result<U>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I, T, U> MapFn<I, T, U> for F
where
    F: Fn(T) -> R + Send + 'static,
    R: IntoReport<U>,
    I: Display,
    T: Value,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %id))
    )]
    #[inline]
    fn execute(&self, id: &I, data: T) -> Result<U> {
        catch(|| self(data).into_report())
    }
}

impl<F, R, I, T, U> MapFn<I, T, U> for WithId<F>
where
    F: Fn(&I, T) -> R + Send + 'static,
    R: IntoReport<U>,
    I: Display,
    T: Value,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %id))
    )]
    #[inline]
    fn execute(&self, id: &I, data: T) -> Result<U> {
        catch(|| self(id, data).into_report())
    }
}

impl<F, I, T, U> MapFn<I, T, U> for WithSplat<F>
where
    F: MapFn<I, Splat<T>, U>,
{
    #[inline]
    fn execute(&self, id: &I, data: T) -> Result<U> {
        F::execute(self, id, Splat::from(data))
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements map function trait for splat arguments.
macro_rules! impl_map_fn_for_splat {
    ($($T:ident),+) => {
        impl<F, R, I, $($T,)+ U> MapFn<I, Splat<($($T,)+)>, U> for F
        where
            F: Fn($($T),+) -> R + Send + 'static,
            R: IntoReport<U>,
            I: Display,
        {
            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(level = "debug", skip_all, fields(id = %id))
            )]
            #[inline]
            fn execute(
                &self, id: &I, data: Splat<($($T,)+)>
            ) -> Result<U> {
                #[allow(non_snake_case)]
                let ($($T,)+) = data.into_inner();
                catch(|| self($($T),+).into_report())
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_map_fn_for_splat!(T1);
impl_map_fn_for_splat!(T1, T2);
impl_map_fn_for_splat!(T1, T2, T3);
impl_map_fn_for_splat!(T1, T2, T3, T4);
impl_map_fn_for_splat!(T1, T2, T3, T4, T5);
impl_map_fn_for_splat!(T1, T2, T3, T4, T5, T6);
impl_map_fn_for_splat!(T1, T2, T3, T4, T5, T6, T7);
impl_map_fn_for_splat!(T1, T2, T3, T4, T5, T6, T7, T8);
