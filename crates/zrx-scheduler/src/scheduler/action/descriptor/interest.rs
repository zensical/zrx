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

//! Action interest.

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Action interest.
///
/// Interests can be thought of as subscriptions to lifecycle events within the
/// scheduler, describing the [`Signal`][] kinds the action is interested in.
///
/// [`Signal`]: crate::scheduler::effect::Signal
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Interest {
    /// Item submitted.
    ///
    /// This interest indicates that the action wants to be notified whenever an
    /// item is submitted to the scheduler. This is typically used for actions
    /// that create barriers or other synchronization points, as they need to
    /// be aware of all items that are currently in the system.
    Submit,
}
