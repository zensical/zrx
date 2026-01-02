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

//! Action property.

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Action property.
///
/// Properties describe certain characteristics of an [`Action`][] which affect
/// how the scheduler might optimize its execution, including handling of side
/// effects. By default, no properties are assumed, instructing the scheduler
/// to be conservative, i.e., safe but not optimal.
///
/// __Warning__: Always make sure that the assumed properties actually hold, or
/// execution can yield undefined behavior, e.g., assuming [`Property::Flush`]
/// for an action that emits deltas of items. Thus, always thoroughly check
/// the description and constraints of the property before assuming it.
///
/// [`Action`]: crate::scheduler::action::Action
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Property {
    /// Action is pure.
    ///
    /// An action is considered pure if it consistently produces the same output
    /// for the same input, which basically means the absence of internal state.
    /// It can, of course, return effects like [`Task`][] and [`Timer`][] to
    /// defer execution, as long as results are consistent.
    ///
    /// [`Task`]: crate::scheduler::effect::Task
    /// [`Timer`]: crate::scheduler::effect::Timer
    Pure,
    /// Action is stable.
    ///
    /// An action is considered stable if it always produces outputs with the
    /// same identifiers as the inputs it got. This allows the [`Scheduler`][]
    /// to optimize execution by using simpler synchronization primitives that
    /// require less memory and compute resources.
    ///
    /// [`Scheduler`]: crate::scheduler::Scheduler
    Stable,
    /// Action is flushing.
    ///
    /// An action is considered flushing if its output is always self-contained.
    /// This allows the scheduler to interrupt any active executions, as their
    /// outputs will be superseded by new inputs. Assuming this property for
    /// outputs that are not self-contained leads to lost emissions.
    Flush,
    /// Action concurrency.
    Concurrency(usize),
}
