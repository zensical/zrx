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

//! Matcher.

use globset::GlobSet;
use std::str::FromStr;

use super::ToId;

mod builder;
mod error;
mod selector;

use builder::Builder;
pub use error::{Error, Result};
pub use selector::{Selector, ToSelector};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Matcher.
///
/// The [`Matcher`] provides efficient [`Selector`] matching of identifiers by
/// leveraging the [`globset`] crate. Matchers can be built from an arbitrary
/// number of selectors, which are then combined into a single [`GlobSet`] for
/// each of the five components.
///
/// [`GlobSet`] implements matching using deterministic finite automata (DFA),
/// which allow for efficient matching of multiple selectors against a single
/// identifier in linear time in relation to the length of the input string,
/// and which return the set of matched selectors.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::{Id, Matcher};
///
/// // Create identifier
/// let id = Id::new("file", "docs", "index.md")?;
///
/// // Create matcher builder and add selector
/// let mut builder = Matcher::builder();
/// builder.add("zrs::::**/*.md:")?;
///
/// // Create matcher from builder
/// let matcher = builder.build()?;
///
/// // Check if the id matches the selector
/// assert!(matcher.is_match(&id)?);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Matcher {
    /// Glob set for scheme.
    scheme: GlobSet,
    /// Glob set for binding.
    binding: GlobSet,
    /// Glob set for context.
    context: GlobSet,
    /// Glob set for path.
    path: GlobSet,
    /// Glob set for fragment.
    fragment: GlobSet,
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
        Builder::new()
    }

    /// Checks if one of the underlying selectors matches an identifier.
    ///
    /// Components are compared in descending variability and their likelihood
    /// for mismatch, starting with the `path`. This approach effectively tries
    /// to short-circuits the comparison. Note that empty components must be
    /// considered wildcards, so they will always match.
    ///
    /// # Errors
    ///
    /// This method returns an error if the given identifier is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{Id, Matcher};
    ///
    /// // Create identifier
    /// let id = Id::new("file", "docs", "index.md")?;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add("zrs::::**/*.md:")?;
    ///
    /// // Create matcher from builder
    /// let matcher = builder.build()?;
    ///
    /// // Check if the id matches the selector
    /// assert!(matcher.is_match(&id)?);
    /// # Ok(())
    /// # }
    /// ```
    ///
    #[allow(clippy::needless_pass_by_value)]
    pub fn is_match<I>(&self, id: I) -> Result<bool>
    where
        I: ToId,
    {
        let id = id.to_id()?;

        // Compare components in descending variability
        Ok(compare(&self.path, Some(id.path().as_ref()))
            && compare(&self.context, Some(id.context().as_ref()))
            && compare(&self.scheme, Some(id.scheme().as_ref()))
            && compare(&self.binding, id.binding().as_deref())
            && compare(&self.fragment, id.fragment().as_deref()))
    }

    /// Returns the match set of the selectors that match an identifier.
    ///
    /// This method compares each component of the identifier against the
    /// corresponding component of a selector using the compiled globs, and
    /// returns the indexes of the matching selectors in the order they were
    /// added to the [`Matcher`].
    ///
    /// Components are compared in descending variability and their likelihood
    /// for mismatch, starting with the `path`. This approach effectively tries
    /// to short-circuits the comparison. Note that empty components must be
    /// considered wildcards, so they will always match.
    ///
    /// # Errors
    ///
    /// This method returns an error if the given identifier is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{Id, Matcher};
    ///
    /// // Create identifier
    /// let id = Id::new("file", "docs", "index.md")?;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add("zrs::::**/*.md:")?;
    ///
    /// // Create matcher from builder
    /// let matcher = builder.build()?;
    ///
    /// // Obtain selectors matched by identifier
    /// let matches = matcher.matches(&id)?;
    /// assert_eq!(matches, [0]);
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::if_not_else)]
    #[allow(clippy::needless_pass_by_value)]
    pub fn matches<I>(&self, id: I) -> Result<Vec<usize>>
    where
        I: ToId,
    {
        let id = id.to_id()?;

        // Create a vector and count the matches of each component in the slots
        // of the vector to find all selectors that match the given identifier
        let mut slots = vec![0u8; self.scheme.len()];
        for (component, value) in [
            (&self.path, Some(id.path())),
            (&self.context, Some(id.context())),
            (&self.scheme, Some(id.scheme())),
            (&self.binding, id.binding()),
            (&self.fragment, id.fragment()),
        ] {
            if let Some(value) = value {
                let matches = component.matches(value.as_ref());
                if !matches.is_empty() {
                    for index in matches {
                        slots[index] += 1;
                    }

                // Short-circuit, as the current component doesn't match, so we
                // know the result must be empty and can return immediately
                } else {
                    return Ok(Vec::new());
                }

            // Wildcard match, which means all slots must be updated
            } else {
                slots.iter_mut().for_each(|count| *count += 1);
            }
        }

        // Obtain match set by collecting the indexes of all matching selectors,
        // which are the slots that match exactly five components
        let iter = slots
            .iter()
            .enumerate()
            .filter_map(|(index, &count)| (count == 5).then_some(index));

        // Return match set
        Ok(iter.collect())
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl FromStr for Matcher {
    type Err = Error;

    /// Creates a matcher from a string.
    ///
    /// The string must adhere to the following format and include exactly five
    /// `:` separators, even if some components are empty.
    ///
    /// ``` text
    /// zrs:<scheme>:<binding>:<context>:<path>:<fragment>
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Path`], if a component value contains a
    /// backslash, or [`Error::Format`], if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Matcher;
    ///
    /// // Create matcher from string
    /// let matcher: Matcher = "zrs::::**/*.md:".parse()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from_str(value: &str) -> Result<Self> {
        let mut builder = Matcher::builder();
        builder.add(value)?;
        builder.build()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Compares a component against a value.
///
/// If the value is absent, we must consider this as a wildcard match if and
/// only if the globset was initially constructed with wildcards (i.e. `**`).
/// Unfortunately, this information is not retained in the globset, and we do
/// not want to use more space than necessary to track empty components.
///
/// However, falling back to `U+FFFE`, which is a non-character that should
/// never appear in a proper UTF-8 string should be sufficient for the check.
fn compare(component: &GlobSet, value: Option<&str>) -> bool {
    component.is_match(value.unwrap_or("\u{FFFE}"))
}
