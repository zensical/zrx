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

//! Macros for output collection creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Converts a list of expressions into an output collection.
///
/// This macro allows to return multiple outputs of different types as part of
/// a single expression, e.g., to return a [`Task`][] alongside an [`Item`][].
///
/// [`Item`]: crate::scheduler::effect::Item
/// [`Task`]: crate::scheduler::effect::Task
///
/// # Examples
///
/// ```
/// use zrx_scheduler::effect::{Item, Task};
/// use zrx_scheduler::outputs;
///
/// // Create output collection from task and item
/// let outputs = outputs![
///     Task::new(|| println!("Task")),
///     Item::new("id", Some(42)),
/// ];
/// ```
#[macro_export]
macro_rules! outputs {
    [$($item:expr),+ $(,)?] => {
        $crate::action::output::Outputs::from_iter([
            $($crate::action::output::Output::from($item)),+
        ])
    };
}
