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

//! Input storages.

use std::any::Any;
use std::ops::Deref;

use zrx_storage::convert::TryAsStorages;

use crate::scheduler::action::Action;
use crate::scheduler::signal::Scope;

use super::error::Result;
use super::Context;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Input storages.
#[derive(Debug)]
pub struct Inputs<'a, I, S>
where
    S: TryAsStorages<Scope<I>>,
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
    /// Returns the input storages.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Storage`][] if the selected input storages cannot be
    /// converted for the target type defined in [`Action::Inputs`].
    ///
    /// [`Error::Storage`]: crate::scheduler::action::context::Error::Storage
    #[inline]
    pub fn inputs(&self) -> Result<Inputs<'_, I, C::Inputs>> {
        Inputs::new(&self.inputs)
    }
}

// ----------------------------------------------------------------------------

impl<'a, I, S> Inputs<'a, I, S>
where
    S: TryAsStorages<Scope<I>>,
{
    /// Creates the input storages.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Storage`][] if the selected input storages cannot be
    /// converted for the target type defined in [`Action::Inputs`].
    ///
    /// [`Error::Storage`]: crate::scheduler::action::context::Error::Storage
    #[inline]
    pub fn new<T>(iter: T) -> Result<Self>
    where
        T: IntoIterator<Item = &'a dyn Any>,
    {
        Ok(Self {
            target: S::try_as_storages(iter)?,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, I, S> Deref for Inputs<'a, I, S>
where
    S: TryAsStorages<Scope<I>>,
{
    type Target = S::Target<'a>;

    /// Dereferences to the input storages.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.target
    }
}
