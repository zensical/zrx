// Copyright (c) 2024 Zensical <contributors@zensical.org>

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
    /// Glob set builder for scheme.
    scheme: GlobSetBuilder,
    /// Glob set builder for binding.
    binding: GlobSetBuilder,
    /// Glob set builder for context.
    context: GlobSetBuilder,
    /// Glob set builder for path.
    path: GlobSetBuilder,
    /// Glob set builder for fragment.
    fragment: GlobSetBuilder,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Builder {
    /// Creates a matcher builder.
    ///
    /// Note that the canonical way to create a [`Matcher`] is to invoke the
    /// [`Matcher::builder`] method, which creates an instance of [`Builder`].
    /// This is also why we don't implement [`Default`] - the builder itself
    /// should be considered an implementation detail.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher builder
    /// let mut builder = Matcher::builder();
    /// ```
    #[allow(clippy::new_without_default)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            scheme: GlobSetBuilder::new(),
            binding: GlobSetBuilder::new(),
            context: GlobSetBuilder::new(),
            path: GlobSetBuilder::new(),
            fragment: GlobSetBuilder::new(),
        }
    }

    /// Adds a selector to the matcher.
    ///
    /// This method adds a [`Selector`][] to the matcher, creating a [`Glob`]
    /// from each component, adding it to the corresponding [`GlobSetBuilder`].
    /// If a component is empty, it is coerced to `**`, as the counts of all
    /// components must match for correct intersection in [`Matcher::matches`].
    ///
    /// [`Selector`]: crate::Selector
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
    /// builder.add("zrs::::**/*.md:")?;
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
        self.scheme.add(parse(selector.scheme().as_deref())?);
        self.binding.add(parse(selector.binding().as_deref())?);
        self.context.add(parse(selector.context().as_deref())?);
        self.path.add(parse(selector.path().as_deref())?);
        self.fragment.add(parse(selector.fragment().as_deref())?);

        // Return self for chaining
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
    /// builder.add("zrs::::**/*.md:")?;
    ///
    /// // Create matcher from builder
    /// let matcher = builder.build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Matcher> {
        Ok(Matcher {
            scheme: self.scheme.build()?,
            binding: self.binding.build()?,
            context: self.context.build()?,
            path: self.path.build()?,
            fragment: self.fragment.build()?,
        })
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
