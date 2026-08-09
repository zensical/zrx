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

//! Action.

use zrx_storage::convert::{TryAsStorageMut, TryAsStorages};

use super::signal::Key;
use super::step::IntoSteps;

pub mod context;
mod error;
pub mod graph;
pub mod options;

pub use context::Context;
pub use error::{Error, Result};
pub use options::Options;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Action.
///
/// Actions represent units of work that can be executed by a [`Scheduler`][]
/// in a type-safe manner. Each action must define two associated types:
///
/// - [`Action::Inputs`], which defines the input types of the action, sourced
///   from the outputs of other actions that are considered its dependencies.
///
/// - [`Action::Output`], which defines the output type of the action, turning
///   the action into a dependency for dependent actions downstream.
///
/// This makes actions reusable and composable, as they are fundamentally just
/// functions with typed inputs and outputs, which can be dynamically connected
/// and executed by the scheduler in topological order. The input types of an
/// action must implement [`TryAsStorages`], which ensures that the scheduler
/// can downcast the type-erased upstream [`Storage`][] instances into concrete
/// types the action can work with. The output type of an action must implement
/// [`TryAsStorageMut`], ensuring that the action can store its output in a
/// concrete [`Storage`][] to pass it to downstream actions as input.
///
/// The [`Action`] type is passed as `Self` to [`Context`] and [`IntoEffects`],
/// two types that are used to provide an action with the necessary context and
/// ability to return [`Effects`], to instruct the scheduler to perform certain
/// operations like [`Task`][] scheduling or [`Timer`][] registration. The type
/// parameter is erased by the scheduler when registering and executing actions
/// through the implementation of the [`Tag`][] trait.
///
/// Note that actions themselves cannot be dyn-compatible, as they require the
/// [`Sized`] bound to pass `Self` to [`Context`] and [`IntoEffects`].
///
/// [`Scheduler`]: crate::scheduler::Scheduler
/// [`Storage`]: zrx_storage::Storage
/// [`Tag`]: crate::scheduler::engine::Tag
/// [`Task`]: crate::scheduler::effect::Task
/// [`Timer`]: crate::scheduler::effect::Timer
pub trait Action<I>: Sized {
    /// Input types of action.
    type Inputs: TryAsStorages<Key<I>>;
    /// Output type of action.
    type Output: TryAsStorageMut<Key<I>>;

    /// Executes the action.
    ///
    /// This method executes the action, and is expected to return any number
    /// of [`Effects`] that will then be handled accordingly by the scheduler.
    ///
    /// # Errors
    ///
    /// Errors returned by the action are forwarded.
    fn execute(&mut self, ctx: Context<I, Self>) -> impl IntoSteps<I, Self>;
}
