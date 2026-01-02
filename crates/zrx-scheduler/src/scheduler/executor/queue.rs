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

//! Execution queues.

use crossbeam::channel::Receiver;
use std::borrow::Cow;

mod task;
mod timer;

pub use task::Tasks;
pub use timer::Timers;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion to [`Receiver`].
pub trait ToReceiver<I> {
    /// Item type returned by receiver.
    type Item;

    /// Converts to a receiver.
    fn to_receiver(&self) -> Cow<'_, Receiver<Self::Item>>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Token.
///
/// Tokens are positional markers, that represent a specific frontier and node
/// within that frontier. They track the execution state of the scheduler, which
/// is necessary to resolve asynchronous effects like [`Task`][] and [`Timer`][]
/// and associate them with the correct frontier and node.
///
/// [`Task`]: crate::scheduler::effect::Task
/// [`Timer`]: crate::scheduler::effect::Timer
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Token {
    /// Frontier identifier.
    pub frontier: usize,
    /// Node identifier.
    pub node: usize,
}
