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

//! Selector.

use ahash::AHasher;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::Arc;

use crate::id::filter::Term;
use crate::id::format::Format;
use crate::id::{Error, Id, Result};

mod builder;
mod convert;
mod macros;

pub use builder::Builder;
pub use convert::TryToSelector;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Selector.
///
/// Selectors are used to match identifiers. Like identifiers, they consist of
/// six components, which can be set to specific values or left empty to act as
/// wildcards. Each components can be set to a glob as supported by [`globset`],
/// which allows for powerful matching capabilities.
///
/// Selectors are no means to an end, but rather a building block to associate
/// data or functions to identifiers via the construction of a [`Matcher`][],
/// which uses an efficient algorithm to match an arbitrary set of selectors in
/// linear time. While it's recommended to use [`Selector::builder`] together
/// with the associated methods to create a new selector, selectors can also be
/// created from a structured string representation with [`Selector::from_str`],
/// which is used internally for serializing them to persistent storage:
///
/// ``` text
/// zrs:<provider>:<resource>:<variant>:<context>:<location>:<fragment>
/// ```
///
/// This ensures blazing fast cloning and editing. Additionally, selectors are
/// guaranteed to not contain backslashes or path traversals in components. An
/// empty component is interpreted as a wildcard, and thus matches all values
/// in that component for any given selector.
///
/// [`Matcher`]: crate::id::matcher::Matcher
///
/// # Examples
///
/// Create a selector:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::Selector;
///
/// // Create selector builder
/// let builder = Selector::builder().location("**/*.md");
///
/// // Create selector from builder
/// let selector = builder.build()?;
/// assert_eq!(selector.as_str(), "zrs:::::**/*.md:");
/// # Ok(())
/// # }
/// ```
///
/// Create a selector from a string:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::Selector;
///
/// // Create selector from string
/// let selector: Selector = "zrs:::::**/*.md:".parse()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Selector {
    /// Formatted string.
    format: Arc<Format<7>>,
    /// Precomputed hash.
    hash: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Selector {
    /// Returns the string representation.
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
    /// // Obtain string representation
    /// assert_eq!(selector.as_str(), "zrs:::::**/*.md:");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.format.as_str()
    }
}

#[allow(clippy::must_use_candidate)]
impl Selector {
    /// Returns the `provider` component, if any.
    #[inline]
    pub fn provider(&self) -> Option<Cow<'_, str>> {
        Some(self.format.get(1)).filter(|value| !value.is_empty())
    }

    /// Returns the `resource` component, if any.
    #[inline]
    pub fn resource(&self) -> Option<Cow<'_, str>> {
        Some(self.format.get(2)).filter(|value| !value.is_empty())
    }

    /// Returns the `variant` component, if any.
    #[inline]
    pub fn variant(&self) -> Option<Cow<'_, str>> {
        Some(self.format.get(3)).filter(|value| !value.is_empty())
    }

    /// Returns the `context` component, if any.
    #[inline]
    pub fn context(&self) -> Option<Cow<'_, str>> {
        Some(self.format.get(4)).filter(|value| !value.is_empty())
    }

    /// Returns the `location` component, if any.
    #[inline]
    pub fn location(&self) -> Option<Cow<'_, str>> {
        Some(self.format.get(5)).filter(|value| !value.is_empty())
    }

    /// Returns the `fragment` component, if any.
    #[inline]
    pub fn fragment(&self) -> Option<Cow<'_, str>> {
        Some(self.format.get(6)).filter(|value| !value.is_empty())
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl FromStr for Selector {
    type Err = Error;

    /// Attempts to create a selector from a string.
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
    /// Returns [`Error::Prefix`] if the prefix isn't `zrs`. Low-level format
    /// errors are returned as part of [`Error::Format`].
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
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        let format = Format::from_str(value)?;

        // Ensure prefix is set
        if format.get(0) != "zrs" {
            Err(Error::Prefix)?;
        }

        // Precompute hash for fast hashing
        let hash = {
            let mut hasher = AHasher::default();
            format.hash(&mut hasher);
            hasher.finish()
        };

        // No errors occurred
        Ok(Self { format: Arc::new(format), hash })
    }
}

// ----------------------------------------------------------------------------

impl TryFrom<Id> for Selector {
    type Error = Error;

    /// Attempts to create a selector from an identifier.
    ///
    /// An [`Id`] can be converted into a [`Selector`] because all identifiers
    /// are also valid selectors, as they represent exact matches. However, the
    /// reverse is not true, as selectors can contain wildcards, as well as
    /// optional components, which identifiers cannot.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{Id, Selector};
    ///
    /// // Create selector from identifier
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// let selector: Selector = id.try_into()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from(id: Id) -> Result<Self> {
        let format = id.format.to_builder().with(0, "zrs").build()?;

        // Precompute hash for fast hashing
        let hash = {
            let mut hasher = AHasher::default();
            format.hash(&mut hasher);
            hasher.finish()
        };

        // No errors occurred
        Ok(Self { format: Arc::new(format), hash })
    }
}

impl TryFrom<Term> for Selector {
    type Error = Error;

    /// Attempts to create a selector from a term.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::filter::Term;
    /// use zrx_id::{Id, Selector};
    ///
    /// // Create selector from identifier
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// let selector: Selector = Term::from(id).try_into()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn try_from(term: Term) -> Result<Self> {
        match term {
            Term::Id(id) => id.try_into(),
            Term::Selector(selector) => Ok(selector),
        }
    }
}

// ----------------------------------------------------------------------------

impl Hash for Selector {
    /// Hashes the selector.
    ///
    /// Since selectors are immutable, we can use a precomputed hash for fast
    /// hashing. This is especially useful when selectors are used as keys in
    /// hash maps or hash sets, where hashing is a frequent operation, as the
    /// performance gains are significant.
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u64(self.hash);
    }
}

// ----------------------------------------------------------------------------

impl PartialEq for Selector {
    /// Compares two selectors for equality.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Selector;
    ///
    /// // Create and compare selectors
    /// let a: Selector = "zrs:::::**/*.md:".parse()?;
    /// let b: Selector = "zrs:::::**/*.md:".parse()?;
    /// assert_eq!(a, b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // We first compare the precomputed hashes, which is extremely fast, as
        // it saves us the comparison when the identifiers are different
        self.hash == other.hash && self.format == other.format
    }
}

impl Eq for Selector {}

// ----------------------------------------------------------------------------

impl PartialOrd for Selector {
    /// Orders two selectors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Selector;
    ///
    /// // Create and compare selectors
    /// let a: Selector = "zrs:::::**/*.md:".parse()?;
    /// let b: Selector = "zrs:::::**/*.rs:".parse()?;
    /// assert!(a < b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Selector {
    /// Orders two selectors.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Selector;
    ///
    /// // Create and compare selectors
    /// let a: Selector = "zrs:::::**/*.md:".parse()?;
    /// let b: Selector = "zrs:::::**/*.rs:".parse()?;
    /// assert!(a < b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.format.cmp(&other.format)
    }
}

// ----------------------------------------------------------------------------

impl Display for Selector {
    /// Formats the selector for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.format, f)
    }
}

impl Debug for Selector {
    /// Formats the selector for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Selector")
            .field("provider", &self.provider())
            .field("resource", &self.resource())
            .field("variant", &self.variant())
            .field("context", &self.context())
            .field("location", &self.location())
            .field("fragment", &self.fragment())
            .finish()
    }
}
