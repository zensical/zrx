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

//! Identifier.

use std::fmt::{Debug, Display};
use std::hash::Hash;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Identifier.
///
/// This trait defines the requirements for identifiers, which are the central
/// means of identifying inputs and outputs of actions, as well as frontiers in
/// the scheduler. For instance, two [`Item`][] values with the same identifier
/// are part of the same frontier, which is essential for joins and more.
///
/// Identifiers are required to implement [`Eq`], [`Hash`] and [`Ord`], as they
/// might be stored in stateful operators, whereas [`Display`] and [`Debug`] are
/// required for tracing and debugging purposes. Of course, identifiers must be
/// [`Send`] to be usable in worker threads. Every type that implements all of
/// those traits can be used as an identifier in the scheduler, as we provide
/// a blanket implementation of this trait.
///
/// We assume that identifiers are cheap to clone, so the use of [`Arc`][] is
/// strongly recommended when using string-based identifiers.
///
/// __Warning__: The `'static` lifetime which is required by this trait is a
/// deliberate design choice to simplify passing data to threads. If we would
/// not require the lifetime, we would need to add a lifetime parameter to all
/// types consuming this trait, which is cumbersome to use.
///
/// [`Arc`]: std::sync::Arc
/// [`Item`]: crate::scheduler::effect::Item
pub trait Id:
    Clone + Debug + Display + Eq + Hash + Ord + Send + 'static
{
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

#[rustfmt::skip]
impl<T> Id for T
where
    T: Clone + Debug + Display + Eq + Hash + Ord + Send + 'static {}
