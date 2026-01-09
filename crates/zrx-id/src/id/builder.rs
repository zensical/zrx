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

//! Identifier builder.

use std::borrow::Cow;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use super::error::{Error, Result};
use super::format::{self, Format};
use super::Id;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Identifier builder.
#[derive(Clone, Debug, Default)]
pub struct Builder<'a> {
    /// Format builder.
    format: format::Builder<'a, 7>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Id {
    /// Creates an identifier builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder
    /// let mut builder = Id::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder<'a>() -> Builder<'a> {
        Builder {
            format: Format::builder().with(0, "zri"),
        }
    }

    /// Creates a builder from the identifier.
    ///
    /// This method creates a builder from the current identifier, which allows
    /// to modify components and build a new identifier from an existing one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier from string
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    ///
    /// // Create identifier builder
    /// let mut builder = id.to_builder();
    /// builder.set_location("README.md");
    ///
    /// // Create identifier from builder
    /// let id = builder.build()?;
    /// assert_eq!(id.as_str(), "zri:file:::docs:README.md:");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn to_builder(&self) -> Builder<'_> {
        Builder {
            format: self.format.to_builder().with(0, "zri"),
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set provider
    /// let mut builder = Id::builder().with_provider("git");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set resource
    /// let mut builder = Id::builder().with_resource("master");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set variant
    /// let mut builder = Id::builder().with_variant("en");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set context
    /// let mut builder = Id::builder().with_context("docs");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set location
    /// let mut builder = Id::builder().with_location("docs");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set fragment
    /// let mut builder = Id::builder().with_fragment("anchor");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set provider
    /// let mut builder = Id::builder();
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set resource
    /// let mut builder = Id::builder();
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set variant
    /// let mut builder = Id::builder();
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set context
    /// let mut builder = Id::builder();
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set location
    /// let mut builder = Id::builder();
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set fragment
    /// let mut builder = Id::builder();
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

    /// Builds the identifier.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Component`] if the `provider`, `context`
    /// or `location` components are not set. Additionally, low-level format
    /// errors are returned as part of [`Error::Format`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder
    /// let mut builder = Id::builder();
    /// builder.set_provider("file");
    /// builder.set_context("docs");
    /// builder.set_location("index.md");
    ///
    /// // Create identifier from builder
    /// let id = builder.build()?;
    /// assert_eq!(id.as_str(), "zri:file:::docs:index.md:");
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Id> {
        let format = self.format.build()?;

        // Ensure provider is set
        if format.get(1).is_empty() {
            Err(Error::Component("provider"))?;
        }

        // Ensure context is set
        if format.get(4).is_empty() {
            Err(Error::Component("context"))?;
        }

        // Ensure location is set
        if format.get(5).is_empty() {
            Err(Error::Component("location"))?;
        }

        // Precompute hash for fast hashing
        let hash = {
            let mut hasher = DefaultHasher::new();
            format.hash(&mut hasher);
            hasher.finish()
        };

        // No errors occurred
        Ok(Id { format: Arc::new(format), hash })
    }
}
