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

//! Matcher builder.

use globset::{Glob, GlobBuilder};

use super::component;
use super::error::Result;
use super::selector::TryToSelector;
use super::Matcher;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Matcher builder.
#[derive(Clone, Debug, Default)]
pub struct Builder {
    /// Component builder for provider.
    provider: component::Builder,
    /// Component builder for resource.
    resource: component::Builder,
    /// Component builder for variant.
    variant: component::Builder,
    /// Component builder for context.
    context: component::Builder,
    /// Component builder for location.
    location: component::Builder,
    /// Component builder for fragment.
    fragment: component::Builder,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Matcher {
    /// Creates a matcher builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher builder
    /// let mut builder = Matcher::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder() -> Builder {
        Builder::default()
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Extends the matcher with the given selector.
    ///
    /// This method adds a [`Selector`][] to the matcher, creating a [`Glob`]
    /// for each component and adding it to a [`GlobSetBuilder`][].
    ///
    /// [`GlobSetBuilder`]: globset::GlobSetBuilder
    /// [`Selector`]: crate::id::matcher::selector::Selector
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`] if the selector is invalid, and [`Error::Glob`]
    /// if a component can't be successfully parsed.
    ///
    /// [`Error::Id`]: crate::id::matcher::error::Error::Id
    /// [`Error::Glob`]: crate::id::matcher::error::Error::Glob
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher builder with selector
    /// let mut builder = Matcher::builder().with(&"zrs:::::**/*.md:")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with<T>(mut self, selector: &T) -> Result<Self>
    where
        T: TryToSelector,
    {
        self.add(selector)?;
        Ok(self)
    }

    /// Adds a selector to the matcher.
    ///
    /// Note that [`Builder::with`] offers better ergonomics to create matchers
    /// from fixed sets of selectors, as it simplifies construction by chaining.
    /// However, sometimes matchers are owned by other data types, which makes
    /// it necessary to provide this implementation as well.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`] if the selector is invalid, and [`Error::Glob`]
    /// if a component can't be successfully parsed.
    ///
    /// [`Error::Id`]: crate::id::matcher::error::Error::Id
    /// [`Error::Glob`]: crate::id::matcher::error::Error::Glob
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add(&"zrs:::::**/*.md:")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add<T>(&mut self, selector: &T) -> Result<&mut Self>
    where
        T: TryToSelector,
    {
        let selector = selector.try_to_selector()?;

        // Compile and add each component of the given selector
        self.provider.add(compile(selector.provider().as_deref())?);
        self.resource.add(compile(selector.resource().as_deref())?);
        self.variant.add(compile(selector.variant().as_deref())?);
        self.context.add(compile(selector.context().as_deref())?);
        self.location.add(compile(selector.location().as_deref())?);
        self.fragment.add(compile(selector.fragment().as_deref())?);

        // Return builder for chaining
        Ok(self)
    }

    /// Builds the matcher.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Glob`][] if a component's [`GlobSet`][] can't be built.
    ///
    /// [`Error::Glob`]: crate::id::matcher::error::Error::Glob
    /// [`GlobSet`]: globset::GlobSet
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add(&"zrs:::::**/*.md:")?;
    ///
    /// // Create matcher from builder
    /// let matcher = builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Matcher> {
        Ok(Matcher {
            provider: self.provider.build()?,
            resource: self.resource.build()?,
            variant: self.variant.build()?,
            context: self.context.build()?,
            location: self.location.build()?,
            fragment: self.fragment.build()?,
        })
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Compiles a component for addition to the matcher.
fn compile(opt: Option<&str>) -> Result<Option<Glob>> {
    if let Some(pattern) = opt {
        let mut builder = GlobBuilder::new(pattern);
        // We enable empty alternates to support patterns like "{,**/}*.md",
        // which is a sensible default as it makes glob patterns more flexible
        Ok(Some(builder.empty_alternates(true).build()?))
    } else {
        Ok(None)
    }
}
