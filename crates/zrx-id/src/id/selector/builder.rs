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

//! Selector builder.

use ahash::AHasher;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

use crate::id::format::{self, Format};
use crate::id::Result;

use super::Selector;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Selector builder.
#[derive(Clone, Debug)]
pub struct Builder<'a> {
    /// Format builder.
    format: format::Builder<'a, 7>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Selector {
    /// Creates a selector builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder
    /// let builder = Selector::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder<'a>() -> Builder<'a> {
        Builder::default()
    }

    /// Creates a builder from the selector.
    ///
    /// This method creates a builder from the current selector, which allows
    /// to modify components and build a new selector from an existing one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Selector;
    ///
    /// // Create selector from string
    /// let selector: Selector = "zrs:::::**/*.md:".parse()?;
    ///
    /// // Create selector builder
    /// let builder = selector.to_builder().location("index.md");
    ///
    /// // Create selector from builder
    /// let selector = builder.build()?;
    /// assert_eq!(selector.as_str(), "zrs:::::index.md:");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn to_builder(&self) -> Builder<'_> {
        Builder {
            format: self.format.to_builder().with(0, "zrs"),
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a> Builder<'a> {
    /// Sets the `provider` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set provider
    /// let builder = Selector::builder().provider("git");
    /// ```
    #[inline]
    #[must_use]
    pub fn provider<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(1, value);
        self
    }

    /// Sets the `resource` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set resource
    /// let builder = Selector::builder().resource("master");
    /// ```
    #[inline]
    #[must_use]
    pub fn resource<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(2, value);
        self
    }

    /// Sets the `variant` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set variant
    /// let builder = Selector::builder().variant("en");
    /// ```
    #[inline]
    #[must_use]
    pub fn variant<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(3, value);
        self
    }

    /// Sets the `context` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set context
    /// let builder = Selector::builder().context("docs");
    /// ```
    #[inline]
    #[must_use]
    pub fn context<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(4, value);
        self
    }

    /// Sets the `location` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set location
    /// let builder = Selector::builder().location("docs");
    /// ```
    #[inline]
    #[must_use]
    pub fn location<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(5, value);
        self
    }

    /// Sets the `fragment` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set fragment
    /// let builder = Selector::builder().fragment("anchor");
    /// ```
    #[inline]
    #[must_use]
    pub fn fragment<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(6, value);
        self
    }

    /// Builds the selector.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Format`][] if the format is invalid.
    ///
    /// [`Error::Format`]: crate::id::Error::Format
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder
    /// let mut builder = Selector::builder()
    ///     .provider("file")
    ///     .context("docs")
    ///     .location("**/*.md");
    ///
    /// // Create selector from builder
    /// let selector = builder.build()?;
    /// assert_eq!(selector.as_str(), "zrs:file:::docs:**/*.md:");
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Selector> {
        let format = self.format.build()?;

        // Precompute hash for fast hashing
        let hash = {
            let mut hasher = AHasher::default();
            format.hash(&mut hasher);
            hasher.finish()
        };

        // No errors occurred
        Ok(Selector { format, hash })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Default for Builder<'_> {
    /// Creates a selector builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::selector::Builder;
    ///
    /// // Create selector builder
    /// let builder = Builder::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Self {
            format: Format::builder().with(0, "zrs"),
        }
    }
}
