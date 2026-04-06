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

//! Output storage.

use std::any::Any;
use std::ops::{Deref, DerefMut};

use zrx_storage::convert::TryAsStorageMut;
use zrx_storage::Storage;

use crate::scheduler::action::Action;
use crate::scheduler::signal::{Id, Scope, Value};

use super::error::Result;
use super::Context;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Output storage.
#[derive(Debug)]
pub struct Output<'a, I, S>
where
    S: TryAsStorageMut<Scope<I>>,
{
    /// Conversion target.
    target: S::Target<'a>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, C> Context<'_, I, C>
where
    C: Action<I>,
{
    /// Returns the output storage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Storage`][] if the selected output storage cannot be
    /// converted for the target type defined in [`Action::Output`].
    ///
    /// [`Error::Storage`]: crate::scheduler::action::context::Error::Storage
    #[inline]
    pub fn output(&mut self) -> Result<Output<'_, I, C::Output<'_>>> {
        Output::new(self.output.as_mut())
    }
}

// ----------------------------------------------------------------------------

impl<'a, I, S> Output<'a, I, S>
where
    S: TryAsStorageMut<Scope<I>>,
{
    /// Creates an output storage.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Storage`][] if the selected output storage cannot be
    /// converted for the target type defined in [`Action::Output`].
    ///
    /// [`Error::Storage`]: crate::scheduler::action::context::Error::Storage
    #[inline]
    pub fn new(item: &'a mut dyn Any) -> Result<Self> {
        Ok(Self {
            target: S::try_as_storage_mut(item)?,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Deref for Output<'_, I, T>
where
    I: Id,
    T: Value,
{
    type Target = Storage<Scope<I>, T>;

    /// Dereferences to the output storage.
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.target
    }
}

impl<I, T> DerefMut for Output<'_, I, T>
where
    I: Id,
    T: Value,
{
    /// Dereferences to the output storage mutably.
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.target
    }
}
