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

//! Output.

use std::fmt;

use crate::scheduler::effect::{Item, Task, Timer};
use crate::scheduler::value::Value;

mod collection;
mod convert;
mod macros;

pub use collection::Outputs;
pub use convert::IntoOutputs;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Output.
///
/// Outputs are the return values of [`Action::execute`][], which includes the
/// effects [`Item`], [`Task`] and [`Timer`]. Actions can also return multiple
/// [`Outputs`], which can be conveniently created from any effect through the
/// [`IntoOutputs`] conversion trait, or the [`outputs!`][] macro.
///
/// This means that this type should never need to be created explicitly, as it
/// is created automatically through the conversion traits of effects.
///
/// [`outputs!`]: crate::outputs!
/// [`Action::execute`]: crate::scheduler::action::Action::execute
pub enum Output<I> {
    /// Item.
    Item(OutputItem<I>),
    /// Task.
    Task(Task<I>),
    /// Timer.
    Timer(Timer<I>),
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> From<Item<I, Option<T>>> for Output<I>
where
    T: Value,
{
    /// Creates an output from an item.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Output;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create output from item
    /// let output = Output::from(Item::new("id", Some(42)));
    /// ```
    #[inline]
    fn from(item: Item<I, Option<T>>) -> Self {
        Output::Item(item.upcast())
    }
}

impl<I> From<Task<I>> for Output<I> {
    /// Creates an output from a task.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Output;
    /// use zrx_scheduler::effect::Task;
    ///
    /// // Create output from task
    /// let output = Output::from(Task::new(|| println!("Task")));
    /// # let _: Output<()> = output;
    /// ```
    #[inline]
    fn from(task: Task<I>) -> Self {
        Output::Task(task)
    }
}

impl<I> From<Timer<I>> for Output<I> {
    /// Creates an output from a timer.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Output;
    /// use zrx_scheduler::effect::Timer;
    ///
    /// // Create output from timer
    /// let output = Output::from(Timer::set(100, None));
    /// # let _: Output<()> = output;
    /// ```
    #[inline]
    fn from(timer: Timer<I>) -> Self {
        Output::Timer(timer)
    }
}

// ----------------------------------------------------------------------------

impl<I> fmt::Debug for Output<I>
where
    I: fmt::Debug,
{
    /// Formats the output for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Output::Item(item) => item.fmt(f),
            Output::Task(task) => task.fmt(f),
            Output::Timer(timer) => timer.fmt(f),
        }
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Output item.
pub type OutputItem<I> = Item<I, Option<Box<dyn Value>>>;
