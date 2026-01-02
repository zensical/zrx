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

use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use crate::id::format::{self, Format};
use crate::id::Result;

use super::Selector;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Selector builder.
#[derive(Clone, Debug, Default)]
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
    /// let mut builder = Selector::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder<'a>() -> Builder<'a> {
        Builder {
            format: Format::builder().with(0, "zrs"),
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a> Builder<'a> {
    /// Updates the `provider` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set provider
    /// let mut builder = Selector::builder().with_provider("git");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_provider<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set_provider(value);
        self
    }

    /// Updates the `resource` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set resource
    /// let mut builder = Selector::builder().with_resource("master");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_resource<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set_resource(value);
        self
    }

    /// Updates the `variant` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set variant
    /// let mut builder = Selector::builder().with_variant("en");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_variant<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set_variant(value);
        self
    }

    /// Updates the `context` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set context
    /// let mut builder = Selector::builder().with_context("docs");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_context<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set_context(value);
        self
    }

    /// Updates the `location` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set location
    /// let mut builder = Selector::builder().with_location("docs");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_location<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set_location(value);
        self
    }

    /// Updates the `fragment` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set fragment
    /// let mut builder = Selector::builder().with_fragment("anchor");
    /// ```
    #[inline]
    #[must_use]
    pub fn with_fragment<S>(mut self, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set_fragment(value);
        self
    }

    /// Updates the `provider` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set provider
    /// let mut builder = Selector::builder();
    /// builder.set_provider("git");
    /// ```
    #[inline]
    pub fn set_provider<S>(&mut self, value: S) -> &mut Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(1, value);
        self
    }

    /// Updates the `resource` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set resource
    /// let mut builder = Selector::builder();
    /// builder.set_resource("master");
    /// ```
    #[inline]
    pub fn set_resource<S>(&mut self, value: S) -> &mut Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(2, value);
        self
    }

    /// Updates the `variant` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set variant
    /// let mut builder = Selector::builder();
    /// builder.set_variant("en");
    /// ```
    #[inline]
    pub fn set_variant<S>(&mut self, value: S) -> &mut Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(3, value);
        self
    }

    /// Updates the `context` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set context
    /// let mut builder = Selector::builder();
    /// builder.set_context("docs");
    /// ```
    #[inline]
    pub fn set_context<S>(&mut self, value: S) -> &mut Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(4, value);
        self
    }

    /// Updates the `location` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set location
    /// let mut builder = Selector::builder();
    /// builder.set_location("docs");
    /// ```
    #[inline]
    pub fn set_location<S>(&mut self, value: S) -> &mut Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.format.set(5, value);
        self
    }

    /// Updates the `fragment` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Selector;
    ///
    /// // Create selector builder and set fragment
    /// let mut builder = Selector::builder();
    /// builder.set_fragment("anchor");
    /// ```
    #[inline]
    pub fn set_fragment<S>(&mut self, value: S) -> &mut Self
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
    /// This method returns [`Error::Format`][] if the format is invalid.
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
    /// let mut builder = Selector::builder();
    /// builder.set_provider("file");
    /// builder.set_context("docs");
    /// builder.set_location("**/*.md");
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
            let mut hasher = DefaultHasher::new();
            format.hash(&mut hasher);
            hasher.finish()
        };

        // No errors occurred
        Ok(Selector { format: Arc::new(format), hash })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a> From<format::Builder<'a, 7>> for Builder<'a> {
    /// Creates a selector builder from a formatted string.
    ///
    /// This implementation is primarily provided for [`Selector::to_builder`],
    /// which allows to convert a [`Selector`] back into a builder.
    #[inline]
    fn from(builder: format::Builder<'a, 7>) -> Self {
        Self { format: builder.with(0, "zrs") }
    }
}
