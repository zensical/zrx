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

//! Step set.

use std::fmt::{self, Debug};
use std::vec::IntoIter;

use crate::scheduler::engine::Tag;

use super::Step;

mod convert;

pub use convert::IntoSteps;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Step set.
///
/// This data type represents a collection of steps, which defines conversion
/// traits for various types, such as [`Option`], [`Vec`], and slices, allowing
/// to return multiple steps from an action, which can be of different types,
/// as long as they implement the [`Into`] conversion trait for [`Step`].
///
/// The given parameter `C` captures a specific type used during scheduling, in
/// order to pass type information through a chain of steps inside of actions.
/// When registering actions, the scheduler will erase the type information, as
/// it's only necessary during compile time, not during execution.
pub struct Steps<I, C = ()> {
    /// Vector of steps.
    inner: Vec<Step<I, C>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

#[allow(clippy::must_use_candidate)]
impl<I, C> Steps<I, C> {
    /// Returns the number of steps.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any steps.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, C> Tag<I, C> for Steps<I, C> {
    type Target<T> = Steps<I, T>;

    /// Tags the step set with the given type.
    #[inline]
    fn tag<T>(self) -> Self::Target<T> {
        Steps {
            inner: self.inner.into_iter().map(Step::tag).collect(),
        }
    }
}

// ----------------------------------------------------------------------------

impl<I> From<()> for Steps<I> {
    /// Creates a step set from the unit value.
    ///
    /// This implementation makes the API more flexible, as it allows to just
    /// return nothing from an action, which can be practical for development.
    #[inline]
    fn from((): ()) -> Self {
        Self::default()
    }
}

impl<I, C> From<Step<I, C>> for Steps<I, C> {
    /// Creates a step set from a step.
    #[inline]
    fn from(value: Step<I, C>) -> Self {
        Self::from_iter(Some(value))
    }
}

impl<I, C> From<Option<Step<I, C>>> for Steps<I, C> {
    /// Creates a step set from an optional step.
    #[inline]
    fn from(value: Option<Step<I, C>>) -> Self {
        Self::from_iter(value)
    }
}

impl<I, C, const N: usize> From<[Step<I, C>; N]> for Steps<I, C> {
    /// Creates a step set from a slice of steps.
    #[inline]
    fn from(value: [Step<I, C>; N]) -> Self {
        Self::from_iter(value)
    }
}

impl<I, C> From<Vec<Step<I, C>>> for Steps<I, C> {
    /// Creates a step set from a vector of steps.
    #[inline]
    fn from(value: Vec<Step<I, C>>) -> Self {
        Self::from_iter(value)
    }
}

// ----------------------------------------------------------------------------

impl<I, C> FromIterator<Step<I, C>> for Steps<I, C> {
    /// Creates a step set from an iterator.
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = Step<I, C>>,
    {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl<I, C> IntoIterator for Steps<I, C> {
    type Item = Step<I, C>;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates a consuming iterator over the step set.
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Default for Steps<I, C> {
    /// Creates a step set.
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::default() }
    }
}

// ----------------------------------------------------------------------------

impl<I, C> Debug for Steps<I, C>
where
    I: Debug,
    C: Debug,
{
    /// Formats the step set for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.inner.iter()).finish()
    }
}
