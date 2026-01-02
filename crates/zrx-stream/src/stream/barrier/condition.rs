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

//! Condition.

use std::fmt;
use std::sync::Arc;

use zrx_scheduler::{Id, Value};

mod id;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Condition function.
pub trait ConditionFn<I>: Send + Sync {
    /// Returns whether the identifier satisfies the condition.
    fn satisfies(&self, id: &I) -> bool;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Condition.
///
/// Conditions are used to determine whether a [`Barrier`][] contains a specific
/// identifier. They implement [`Value`], so they can be created and returned by
/// any [`Operator`][]. The resulting [`Stream`][] of conditions can be used in
/// any operator that expects conditions, such as [`Stream::select`][].
///
/// [`Barrier`]: crate::stream::barrier::Barrier
/// [`Operator`]: crate::stream::operator::Operator
/// [`Stream`]: crate::stream::Stream
/// [`Stream::select`]: crate::stream::Stream::select
///
/// # Examples
///
/// ```
/// use zrx_stream::barrier::Condition;
///
/// // Create condition and test identifier
/// let condition = Condition::new(|&id: &i32| id < 100);
/// assert!(condition.satisfies(&42));
/// ```
#[derive(Clone)]
pub struct Condition<I> {
    /// Condition function.
    function: Arc<dyn ConditionFn<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Condition<I> {
    /// Creates a condition.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::barrier::Condition;
    ///
    /// // Create condition
    /// let condition = Condition::new(|&id: &i32| id < 100);
    /// ```
    pub fn new<F>(f: F) -> Self
    where
        F: ConditionFn<I> + 'static,
    {
        Self { function: Arc::new(f) }
    }

    /// Returns whether the given identifier satisfies the condition.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_stream::barrier::Condition;
    ///
    /// // Create condition and test identifier
    /// let condition = Condition::new(|&id: &i32| id < 100);
    /// assert!(condition.satisfies(&42));
    /// ```
    #[inline]
    pub fn satisfies(&self, id: &I) -> bool {
        self.function.satisfies(id)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Value for Condition<I> where I: Id {}

// ----------------------------------------------------------------------------

impl<I> fmt::Debug for Condition<I> {
    /// Formats the condition for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let function = "Box<dyn ConditionFn>";
        f.debug_struct("Condition")
            .field("function", &function)
            .finish()
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<F, I> ConditionFn<I> for F
where
    F: Fn(&I) -> bool + Send + Sync,
{
    #[inline]
    fn satisfies(&self, id: &I) -> bool {
        self(id)
    }
}
