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

//! Signal.

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Signal.
///
/// Signals allow to pass system messages to an [`Action`][], which in turn can
/// emit one or more [`Output`][] values in response, similar to how items are
/// handled, but with the added flexibility of being able to react to system
/// state changes or user-defined barriers.
///
/// This enum is marked as non-exhaustive, so new variants can be added in the
/// future without breaking existing downstream code, especially if signals are
/// ignored, which is recommended for most cases, since only some actions are
/// expected to react to signals in a meaningful way.
///
/// [`Action`]: crate::scheduler::action::Action
/// [`Output`]: crate::scheduler::action::Output
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Signal<'a, I> {
    /// Identifier submission signal.
    Submit(&'a I),
}
