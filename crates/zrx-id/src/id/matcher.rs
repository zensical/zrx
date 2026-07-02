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

//! Matcher.

use std::str::FromStr;

use super::convert::TryToId;

mod builder;
mod component;
mod error;
pub mod matches;

pub use builder::Builder;
use component::Component;
pub use error::{Error, Result};
pub use matches::Matches;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Matcher.
///
/// Matchers provide efficient matching of identifiers against an arbitrary set
/// of selectors in linear time, implemented through the use of the [`globset`]
/// crate, which compiles globs into deterministic finite automata (DFA). Each
/// [`Component`] of the matcher receives its own distinct [`GlobSet`][].
///
/// While components are matched one after another, all registered identifiers
/// in a [`Component`] are matched in linear time, i.e., O(n), where n is the
/// length of the component value. The [`Matches`] returned by each component
/// are intersected, leaving only selectors that match all components. There
/// are theoretical limits on the number of selectors that can be added to a
/// [`Component`], so it can be necessary to split across multiple matchers if
/// the number of selectors is high, i.e., 10,000 or more.
///
/// [`GlobSet`]: globset::GlobSet
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::matcher::Matcher;
/// use zrx_id::Id;
///
/// // Create matcher builder and add selector
/// let mut builder = Matcher::builder();
/// builder.add(&"zrs:::::**/*.md:")?;
///
/// // Create matcher from builder
/// let matcher = builder.build()?;
///
/// // Create identifier and match selector
/// let id: Id = "zri:file:::docs:index.md:".parse()?;
/// assert!(matcher.is_match(&id)?);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default)]
pub struct Matcher {
    /// Component for provider.
    provider: Component,
    /// Component for resource.
    resource: Component,
    /// Component for variant.
    variant: Component,
    /// Component for context.
    context: Component,
    /// Component for location.
    location: Component,
    /// Component for selector.
    fragment: Component,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Matcher {
    /// Returns whether the given identifier matches any selector.
    ///
    /// Components are compared in descending variability and their likelihood
    /// for mismatch, starting with the `location`. This approach effectively
    /// tries to short-circuits the comparison. Note that empty components are
    /// considered wildcards, so they will always match.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`] if the identifier is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::matcher::Matcher;
    /// use zrx_id::Id;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add(&"zrs:::::**/*.md:")?;
    ///
    /// // Create matcher from builder
    /// let matcher = builder.build()?;
    ///
    /// // Create identifier and match selector
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// assert!(matcher.is_match(&id)?);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn is_match<T>(&self, id: &T) -> Result<bool>
    where
        T: TryToId,
    {
        self.matches(id).map(|matches| !matches.is_empty())
    }

    /// Returns the indices of selectors that match the identifier.
    ///
    /// This method compares each component of the identifier against the
    /// corresponding component of a selector using the compiled globs, and
    /// returns the indices of the matching selectors in the order they were
    /// added to the [`Matcher`].
    ///
    /// Components are compared in descending variability and their likelihood
    /// for mismatch, starting with the `location`. This approach effectively
    /// tries to short-circuit the comparison. Note that empty components are
    /// considered wildcards, so they will always match.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`] if the identifier is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::matcher::{Matcher, Matches};
    /// use zrx_id::Id;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add(&"zrs:::::**/*.md:")?;
    ///
    /// // Create matcher from builder
    /// let matcher = builder.build()?;
    ///
    /// // Create identifier and obtain matched selectors
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// assert_eq!(matcher.matches(&id)?, Matches::from_iter([0]));
    /// # Ok(())
    /// # }
    /// ```
    #[allow(clippy::missing_panics_doc)]
    pub fn matches<T>(&self, id: &T) -> Result<Matches>
    where
        T: TryToId,
    {
        let id = id.try_to_id()?;

        // Query all components from highest to lowest variability, and
        // intersect the resulting match sets, keeping only full matches
        let mut opt: Option<Matches> = None;
        for (component, value) in [
            (&self.location, Some(id.location())),
            (&self.context, Some(id.context())),
            (&self.provider, Some(id.provider())),
            (&self.resource, id.resource()),
            (&self.fragment, id.fragment()),
            (&self.variant, id.variant()),
        ] {
            // If the component doesn't have a value, we could theoretically
            // ignore all non-empty patterns and only match the empty ones,
            // but we would then miss selectors that use explicit `*` or `**`
            // wildcards. We use the unlikely `U+FFFE` to test for those.
            let path = value.as_deref().unwrap_or("\u{FFFE}");
            let matches = component.matches(path);

            // Intersect with or set as tracking match set
            if let Some(tracked) = &mut opt {
                tracked.intersect(&matches);
            } else {
                opt = Some(matches);
            }
        }

        // Return matches
        Ok(opt.expect("invariant"))
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl FromStr for Matcher {
    type Err = Error;

    /// Attempts to create a matcher from a string.
    ///
    /// The string must adhere to the following format and include exactly six
    /// `:` separators, even if some components are empty. All components are
    /// optional, which means they can be left empty, which is equivalent to
    /// setting them to a `**` wildcard.
    ///
    /// ``` text
    /// zrs:<provider>:<resource>:<variant>:<context>:<location>:<fragment>
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::Id`] if the given string can't be parsed into a valid
    /// selector, from which the matcher is then constructed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::matcher::Matcher;
    ///
    /// // Create matcher from string
    /// let matcher: Matcher = "zrs:::::**/*.md:".parse()?;
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        Matcher::builder().with(&value)?.build()
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod is_match {
        use crate::id::matcher::{Matcher, Result};

        #[test]
        fn handles_selectors() -> Result {
            for selector in &[
                "zrs:file:::docs:index.md:",
                "zrs::::docs:index.md:",
                "zrs:::::index.md:",
                "zrs::::::",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert!(matcher.is_match(&"zri:file:::docs:index.md:")?);
            }
            Ok(())
        }

        #[test]
        fn handles_wildcards() -> Result {
            for selector in &[
                "zrs:file:::docs:*.md:",
                "zrs:::::*.md:",
                "zrs:*::::*.md:",
                "zrs:*:*:*:*:*:",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert!(matcher.is_match(&"zri:file:::docs:index.md:")?);
            }
            Ok(())
        }

        #[test]
        fn handles_optionals() -> Result {
            for selector in &[
                "zrs:{git,file}:::{docs}:index.md:",
                "zrs::::docs:{index,about}.md:",
                "zrs:::::index.{md,rst}:",
                "zrs:::::{*}:",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert!(matcher.is_match(&"zri:file:::docs:index.md:")?);
            }
            Ok(())
        }

        #[test]
        fn handles_non_matches() -> Result {
            for selector in &[
                "zrs:file:::{docs}:index.md:anchor",
                "zrs:{git,file}:master::::",
                "zrs:::::about.md:",
                "zrs::::::anchor",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert!(!matcher.is_match(&"zri:file:::docs:index.md:")?);
            }
            Ok(())
        }
    }

    mod matches {
        use crate::id::matcher::{Matcher, Matches, Result};

        #[test]
        fn handles_selectors() -> Result {
            for selector in &[
                "zrs:file:::docs:index.md:",
                "zrs::::docs:index.md:",
                "zrs:::::index.md:",
                "zrs::::::",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert_eq!(
                    matcher.matches(&"zri:file:::docs:index.md:")?,
                    Matches::from_iter([0])
                );
            }
            Ok(())
        }

        #[test]
        fn handles_wildcards() -> Result {
            for selector in &[
                "zrs:file:::docs:*.md:",
                "zrs:::::*.md:",
                "zrs:*::::*.md:",
                "zrs:*:*:*:*:*:",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert_eq!(
                    matcher.matches(&"zri:file:::docs:index.md:")?,
                    Matches::from_iter([0])
                );
            }
            Ok(())
        }

        #[test]
        fn handles_optionals() -> Result {
            for selector in &[
                "zrs:{git,file}:::{docs}:index.md:",
                "zrs::::docs:{index,about}.md:",
                "zrs:::::index.{md,rst}:",
                "zrs:::::{*}:",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert_eq!(
                    matcher.matches(&"zri:file:::docs:index.md:")?,
                    Matches::from_iter([0])
                );
            }
            Ok(())
        }

        #[test]
        fn handles_non_matches() -> Result {
            for selector in &[
                "zrs:file:::{docs}:index.md:anchor",
                "zrs:{git,file}:master::::",
                "zrs:::::about.md:",
                "zrs::::::anchor",
            ] {
                let matcher: Matcher = selector.parse()?;
                assert_eq!(
                    matcher.matches(&"zri:file:::docs:index.md:")?,
                    Matches::new()
                );
            }
            Ok(())
        }
    }
}
