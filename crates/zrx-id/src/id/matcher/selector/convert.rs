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

//! Selector conversions.

use std::borrow::Cow;

use crate::id::{Id, Result};

use super::builder::Builder;
use super::Selector;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion to [`Selector`].
///
/// This trait allows to convert an arbitrary value into a selector, using a
/// [`Cow`] smart pointer to avoid unnecessary cloning, e.g. for references.
pub trait ToSelector {
    /// Converts to a selector.
    #[allow(clippy::missing_errors_doc)]
    fn to_selector(&self) -> Result<Cow<'_, Selector>>;
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl ToSelector for &Selector {
    /// Creates a selector from a reference.
    #[inline]
    fn to_selector(&self) -> Result<Cow<'_, Selector>> {
        Ok(Cow::Borrowed(self))
    }
}

impl ToSelector for &Id {
    /// Creates a selector from an identifier.
    ///
    /// Since all identifiers are also valid selectors, implementing this trait
    /// ensures we can also pass identifier references to [`Builder::add`][].
    ///
    /// [`Builder::add`]: crate::id::matcher::Builder::add
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{Id, Selector, ToSelector};
    ///
    /// // Create selector from identifier
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// let selector = (&id).to_selector()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_selector(&self) -> Result<Cow<'_, Selector>> {
        Builder::from(self.format.to_builder())
            .build()
            .map(Cow::Owned)
    }
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> ToSelector for T
where
    T: AsRef<str>,
{
    /// Creates a selector from a string.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Prefix`][] if the prefix isn't `zrs`. Also,
    /// low-level format errors are returned as part of [`Error::Format`][].
    ///
    /// [`Error::Format`]: crate::id::Error::Format
    /// [`Error::Prefix`]: crate::id::Error::Prefix
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{Selector, ToSelector};
    ///
    /// // Create selector from string
    /// let selector = "zrs:::::**/*.md:".to_selector()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_selector(&self) -> Result<Cow<'_, Selector>> {
        self.as_ref().parse().map(Cow::Owned)
    }
}
