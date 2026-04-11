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

//! Context.

use std::fmt::{self, Debug};
use std::marker::PhantomData;

use zrx_storage::set::{View, ViewMut};

use crate::scheduler::engine::Tag;

use super::Action;

mod builder;
mod error;
mod inputs;
mod output;
mod scopes;

pub use builder::Builder;
pub use error::{Error, Result};
use inputs::Inputs;
use output::Output;
use scopes::Scopes;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Context.
///
/// This data type holds the identifiers, as well as views for input and output
/// storages for a given action invocation, and is intended to be consumed by
/// the [`Action`]. The simplest way to use a context in [`Action::execute`] is
/// to bind it by calling [`Context::bind`].
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of effects inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
pub struct Context<'a, I, C = ()> {
    /// Scopes.
    scopes: Scopes<I>,
    /// Input storages.
    inputs: View<'a>,
    /// Output storage.
    output: ViewMut<'a>,
    /// Capture types.
    marker: PhantomData<C>,
}

// ----------------------------------------------------------------------------

/// Context binding.
///
/// Binding a [`Context`] to an [`Action`] allows us to obtain the references
/// to the action's input and output [`Storage`][] instances, which can be used
/// to retrieve and store data for processing by an action, respectively. While
/// storage references can also be obtained by invoking the [`Context::inputs`]
/// and [`Context::output`] methods, binding the context to the action allows
/// us to obtain both at the same time while omitting any issues with borrowing.
///
/// This struct is marked as non-exhaustive, so new fields can be added in the
/// future without worrying about breaking existing actions downstream. Fields
/// are all public, as this type is first and foremost intended for convenient
/// destructuring inside of actions.
///
/// [`Storage`]: zrx_storage::Storage
#[derive(Debug)]
#[non_exhaustive]
pub struct Binding<'a, I, C>
where
    C: Action<I>,
{
    /// Scopes.
    pub scopes: Scopes<I>,
    /// Input storages.
    pub inputs: Inputs<'a, I, C::Inputs>,
    /// Output storage.
    pub output: Output<'a, I, C::Output<'a>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<'a, I, C> Context<'a, I, C>
where
    C: Action<I>,
{
    /// Binds the context to the action.
    ///
    /// This method consumes the [`Context`], and returns a [`Binding`] that
    /// can be used to retrieve the action's input and output storages.
    ///
    /// Binding the context as opposed to retrieving the storages individually,
    /// we can obtain both the input and output storages at the same time. This
    /// allows us to avoid any issues with borrowing, and is optimal in terms
    /// of performance, as we only do it once per action invocation.
    #[allow(clippy::missing_panics_doc)]
    #[must_use]
    pub fn bind(self) -> Binding<'a, I, C> {
        // Note that we're panicking here, since this signals an error in the
        // operator implementation, which is not a user error.
        Binding {
            scopes: self.scopes,
            inputs: Inputs::new(self.inputs).expect("invariant"),
            output: Output::new(self.output.into_inner()).expect("invariant"),
        }
    }
}

#[allow(clippy::must_use_candidate)]
impl<I, C> Context<'_, I, C> {
    /// Returns the scopes.
    #[inline]
    pub fn scopes(&self) -> &Scopes<I> {
        &self.scopes
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I, C> Tag<I, C> for Context<'a, I, C> {
    type Target<T> = Context<'a, I, T>;

    /// Tags the context with the given type.
    #[inline]
    fn tag<T>(self) -> Self::Target<T> {
        Context {
            scopes: self.scopes,
            inputs: self.inputs,
            output: self.output,
            marker: PhantomData,
        }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Debug for Context<'_, I, C>
where
    I: Debug,
{
    /// Formats the context for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Context")
            .field("traces", &self.scopes)
            .field("inputs", &self.inputs)
            .field("output", &self.output)
            .field("marker", &self.marker)
            .finish()
    }
}
