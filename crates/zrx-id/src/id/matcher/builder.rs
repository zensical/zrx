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

use globset::{Glob, GlobSetBuilder};

use super::error::Result;
use super::selector::ToSelector;
use super::Matcher;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Matcher builder.
#[derive(Clone, Debug)]
pub struct Builder {
    /// Glob set builder for provider.
    provider: GlobSetBuilder,
    /// Glob set builder for resource.
    resource: GlobSetBuilder,
    /// Glob set builder for variant.
    variant: GlobSetBuilder,
    /// Glob set builder for context.
    context: GlobSetBuilder,
    /// Glob set builder for location.
    location: GlobSetBuilder,
    /// Glob set builder for fragment.
    fragment: GlobSetBuilder,
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
        Builder {
            provider: GlobSetBuilder::new(),
            resource: GlobSetBuilder::new(),
            variant: GlobSetBuilder::new(),
            context: GlobSetBuilder::new(),
            location: GlobSetBuilder::new(),
            fragment: GlobSetBuilder::new(),
        }
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Extends the matcher with the given selector.
    ///
    /// This method adds a [`Selector`][] to the matcher, creating a [`Glob`]
    /// from each component, adding it to the corresponding [`GlobSetBuilder`].
    /// If a component is empty, it is coerced to `**`, as the counts of all
    /// components must match for correct intersection in [`Matcher::matches`].
    ///
    /// [`Selector`]: crate::id::matcher::Selector
    ///
    /// # Errors
    ///
    /// This method returns an error if the given selector is invalid, or if a
    /// component cannot successfully be parsed into a valid [`Glob`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher builder with selector
    /// let mut builder = Matcher::builder().with("zrs:::::**/*.md:")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn with<S>(mut self, selector: S) -> Result<Self>
    where
        S: ToSelector,
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
    /// This method returns an error if the given selector is invalid, or if a
    /// component cannot successfully be parsed into a valid [`Glob`].
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
    /// builder.add("zrs:::::**/*.md:")?;
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::needless_pass_by_value)]
    pub fn add<S>(&mut self, selector: S) -> Result<&mut Self>
    where
        S: ToSelector,
    {
        let selector = selector.to_selector()?;

        // Compile and add each component of the given selector
        self.provider.add(parse(selector.provider().as_deref())?);
        self.resource.add(parse(selector.resource().as_deref())?);
        self.variant.add(parse(selector.variant().as_deref())?);
        self.context.add(parse(selector.context().as_deref())?);
        self.location.add(parse(selector.location().as_deref())?);
        self.fragment.add(parse(selector.fragment().as_deref())?);

        // Return builder for chaining
        Ok(self)
    }

    /// Builds the matcher.
    ///
    /// # Errors
    ///
    /// This method returns an error if the [`GlobSet`][] that is associated
    /// with a component cannot be successfully built.
    ///
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
    /// builder.add("zrs:::::**/*.md:")?;
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
// Trait implementations
// ----------------------------------------------------------------------------

impl Default for Builder {
    /// Creates a matcher builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::matcher::Builder;
    ///
    /// // Create matcher builder
    /// let mut builder = Builder::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Matcher::builder()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Parses a component into a glob.
///
/// Note that wildcards are implicit, which means that empty components are
/// coerced to `**` to provide an ergonomic API for creating selectors. We must
/// create a selector for each component, or the component count of selectors
/// will not be coherent, which is essential for correct matching.
fn parse(component: Option<&str>) -> Result<Glob> {
    Ok(Glob::new(component.unwrap_or("**"))?)
}
