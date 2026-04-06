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

//! Tag with a type.

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Tag with a type.
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of effects inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
///
/// This is a zero-cost operation, as it only changes the captured type, and
/// can be optimized away by the compiler in its entirety.
pub trait Tag<I, C> {
    /// Target type to tag with.
    type Target<T>;

    /// Tags with the given type.
    #[must_use]
    fn tag<T>(self) -> Self::Target<T>;
}
