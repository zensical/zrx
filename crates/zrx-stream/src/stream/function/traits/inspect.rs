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

use zrx_scheduler::action::report::IntoReport;
use zrx_scheduler::action::Result;
use zrx_scheduler::Value;

use crate::stream::function::adapter::{WithId, WithSplat};
use crate::stream::function::{catch, Splat};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Inspect function.
///
/// This trait defines a function that can be used to inspect borrowed data in
/// a stream without modifying it, and is not expected to return new data.
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
/// [`Report`]: zrx_scheduler::action::report::Report
///
/// # Examples
///
/// Inspect data:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_stream::function::InspectFn;
///
/// // Define and execute function
/// let f = |&n: &i32| println!("{n}");
/// f.execute(&"id", &42)?;
/// # Ok(())
/// # }
/// ```
///
/// Inspect data with splat argument:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_stream::function::{InspectFn, Splat};
///
/// // Define and execute function
/// let f = |&a: &i32, &b: &i32| println!("{a} < {b}");
/// f.execute(&"id", Splat::from_ref(&(1, 2)))?;
/// # Ok(())
/// # }
/// ```
pub trait InspectFn<I, T>: Send + 'static
where
    T: ?Sized,
{
    /// Executes the inspect function.
    ///
    /// # Errors
    ///
    /// This method returns an error if the function fails to execute.
    fn execute(&self, id: &I, data: &T) -> Result;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, R, I, T> InspectFn<I, T> for F
where
    F: Fn(&T) -> R + Send + 'static,
    R: IntoReport,
    I: Display,
    T: Value + ?Sized,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %id))
    )]
    #[inline]
    fn execute(&self, id: &I, data: &T) -> Result {
        catch(|| self(data).into_report())
    }
}

impl<F, R, I, T> InspectFn<I, T> for WithId<F>
where
    F: Fn(&I, &T) -> R + Send + 'static,
    R: IntoReport,
    I: Display,
    T: Value + ?Sized,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(level = "debug", skip_all, fields(id = %id))
    )]
    #[inline]
    fn execute(&self, id: &I, data: &T) -> Result {
        catch(|| self(id, data).into_report())
    }
}

impl<F, I, T> InspectFn<I, T> for WithSplat<F>
where
    F: InspectFn<I, Splat<T>>,
{
    #[inline]
    fn execute(&self, id: &I, data: &T) -> Result {
        F::execute(self, id, Splat::from_ref(data))
    }
}

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Implements inspect function trait for splat arguments.
macro_rules! impl_inspect_fn_for_splat {
    ($($T:ident),+) => {
        impl<F, R, I, $($T,)+> InspectFn<I, Splat<($($T,)+)>> for F
        where
            F: Fn($(&$T),+) -> R + Send + 'static,
            R: IntoReport,
            I: Display,
        {
            #[cfg_attr(
                feature = "tracing",
                tracing::instrument(level = "debug", skip_all, fields(id = %id))
            )]
            #[inline]
            fn execute(
                &self, id: &I, data: &Splat<($($T,)+)>
            ) -> Result {
                #[allow(non_snake_case)]
                let ($($T,)+) = data.inner();
                catch(|| self($($T),+).into_report())
            }
        }
    };
}

// ----------------------------------------------------------------------------

impl_inspect_fn_for_splat!(T1);
impl_inspect_fn_for_splat!(T1, T2);
impl_inspect_fn_for_splat!(T1, T2, T3);
impl_inspect_fn_for_splat!(T1, T2, T3, T4);
impl_inspect_fn_for_splat!(T1, T2, T3, T4, T5);
impl_inspect_fn_for_splat!(T1, T2, T3, T4, T5, T6);
impl_inspect_fn_for_splat!(T1, T2, T3, T4, T5, T6, T7);
impl_inspect_fn_for_splat!(T1, T2, T3, T4, T5, T6, T7, T8);
