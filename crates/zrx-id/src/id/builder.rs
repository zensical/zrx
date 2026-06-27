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

use ahash::AHasher;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};

use super::error::{Error, Result};
use super::format::{self, Format};
use super::Id;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Identifier builder.
#[derive(Clone, Debug)]
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
    /// let builder = Id::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder<'a>() -> Builder<'a> {
        Builder::default()
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
    /// let builder = id.to_builder().location("README.md");
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
    /// Sets the `provider` component.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set provider
    /// let builder = Id::builder().provider("git");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set resource
    /// let builder = Id::builder().resource("master");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set variant
    /// let builder = Id::builder().variant("en");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set context
    /// let builder = Id::builder().context("docs");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set location
    /// let builder = Id::builder().location("docs");
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder and set fragment
    /// let builder = Id::builder().fragment("anchor");
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

    /// Builds the identifier.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Component`] if any of the `provider`, `context` or
    /// `location` components are not set. In case of low-level format errors,
    /// [`Error::Format`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier builder
    /// let builder = Id::builder()
    ///     .provider("file")
    ///     .context("docs")
    ///     .location("index.md");
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
            let mut hasher = AHasher::default();
            format.hash(&mut hasher);
            hasher.finish()
        };

        // No errors occurred
        Ok(Id { format, hash })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Default for Builder<'_> {
    /// Creates an identifier builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Builder;
    ///
    /// // Create identifier builder
    /// let builder = Builder::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Self {
            format: Format::builder().with(0, "zri"),
        }
    }
}
