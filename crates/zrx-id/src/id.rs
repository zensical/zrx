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

//! Identifier.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use zrx_path::PathExt;

mod builder;
mod convert;
mod error;
pub mod filter;
pub mod format;
mod macros;
pub mod matcher;
pub mod uri;

pub use builder::Builder;
pub use convert::TryIntoId;
pub use error::{Error, Result};
use format::Format;
use uri::Uri;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Identifier.
///
/// Identifiers are structured string-based representations which are used to
/// uniquely identify artifacts as they move through streams and stores. They
/// use a compact, yet human-readable format that is easy to generate and
/// parse, and consists of the following six components:
///
/// - `provider`, e.g., file or git.
/// - `resource`, e.g., volume, branch or tag.
/// - `variant`, e.g., language, version or format.
/// - `context`, e.g., source or output directory.
/// - `location`, e.g., file or folder.
/// - `fragment`, e.g., line number or anchor.
///
/// Identifiers implement [`Eq`], [`PartialEq`] and [`Hash`], as well as [`Ord`]
/// and [`PartialOrd`], so they can be stored in ordered and unordered storages,
/// as well as efficiently compared with each other. The structured string-based
/// representation is defined as follows:
///
/// ``` text
/// zri:<provider>:<resource>:<variant>:<context>:<location>:<fragment>
/// ```
///
/// This ensures blazing fast cloning and editing. Additionally, identifiers are
/// guaranteed to not contain backslashes or path traversals in components. An
/// empty component, for those that are allowed to remain empty, is equal to the
/// default in the context set by the given provider.
///
/// # Examples
///
/// Create an identifier:
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
///
/// Create an identifier from a string:
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::Id;
///
/// // Create identifier from string
/// let id: Id = "zri:file:::docs:index.md:".parse()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Id {
    /// Formatted string.
    format: Arc<Format<7>>,
    /// Precomputed hash.
    hash: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Id {
    /// Converts the identifier to a relative file system path.
    ///
    /// This method creates a relative [`PathBuf`] from both, the `context` and
    /// `location` components of the identifier, using platform-dependent path
    /// separators. The resulting path is always relative, and never absolute,
    /// since both, `context` and `location`, are always relative.
    ///
    /// In order to resolve the path, the [`Id::resource`] needs to be taken
    /// into account, which is of course provider-specific. Note that for use
    /// of paths in URLs, [`Id::as_uri`] must be used, which guarantees that
    /// all path separators are forward slashes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use std::path::Path;
    /// use zrx_id::Id;
    ///
    /// // Create identifier from string
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    ///
    /// // Create path from identifier
    /// let path = id.to_path();
    /// assert_eq!(path, Path::new("docs/index.md"));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn to_path(&self) -> PathBuf {
        let mut path = PathBuf::from(self.context().as_ref());
        path.push(self.location().as_ref());
        path.relative_to(".")
    }

    /// Returns the string representation.
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
    /// // Obtain string representation
    /// assert_eq!(id.as_str(), "zri:file:::docs:index.md:");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.format.as_str()
    }

    /// Returns the URI representation.
    ///
    /// This method creates a URI from [`Id::location`], which is necessary for
    /// using the identifier in URLs, e.g., to construct relative links.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::uri::Uri;
    /// use zrx_id::Id;
    ///
    /// // Create identifier from string
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    ///
    /// // Obtain URI representation
    /// assert_eq!(id.as_uri(), Uri::from("index.md"));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn as_uri(&self) -> Uri<'_> {
        Uri::from(self.location())
    }
}

#[allow(clippy::must_use_candidate)]
impl Id {
    /// Returns the `provider` component.
    #[inline]
    pub fn provider(&self) -> Cow<'_, str> {
        self.format.get(1)
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

    /// Returns the `context` component.
    #[inline]
    pub fn context(&self) -> Cow<'_, str> {
        self.format.get(4)
    }

    /// Returns the `location` component.
    #[inline]
    pub fn location(&self) -> Cow<'_, str> {
        self.format.get(5)
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

impl FromStr for Id {
    type Err = Error;

    /// Attempts to create an identifier from a string.
    ///
    /// The string must adhere to the following format and include exactly six
    /// `:` separators, even in case some components are omitted. The optional
    /// components are `resource`, `variant` and `fragment`, and can be left
    /// empty, which is represented as empty strings internally.
    ///
    /// ``` text
    /// zri:<provider>:<resource>:<variant>:<context>:<location>:<fragment>
    /// ```
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Prefix`] if the prefix isn't `zri`, and
    /// [`Error::Component`] if any of the `provider`, `context` or `location`
    /// components are not set. Also, low-level format errors are returned as
    /// part of [`Error::Format`].
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
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        let format = Format::from_str(value)?;

        // Ensure prefix is set
        if format.get(0) != "zri" {
            Err(Error::Prefix)?;
        }

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
        Ok(Self { format: Arc::new(format), hash })
    }
}

// ----------------------------------------------------------------------------

impl Hash for Id {
    /// Hashes the identifier.
    ///
    /// Since identifiers are immutable, we can use a precomputed hash for fast
    /// hashing. This is especially useful when identifiers are used as keys in
    /// hash maps or hash sets, where hashing is a frequent operation, as the
    /// performance gains are significant with constant time.
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u64(self.hash);
    }
}

// ----------------------------------------------------------------------------

impl PartialEq for Id {
    /// Compares two identifiers for equality.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create and compare identifiers
    /// let a: Id = "zri:file:::docs:index.md:".parse()?;
    /// let b: Id = "zri:file:::docs:index.md:".parse()?;
    /// assert_eq!(a, b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Id {}

// ----------------------------------------------------------------------------

impl PartialOrd for Id {
    /// Orders two identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create and compare identifiers
    /// let a: Id = "zri:file:::docs:index.md:".parse()?;
    /// let b: Id = "zri:file:::docs:about.md:".parse()?;
    /// assert!(a > b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Id {
    /// Orders two identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create and compare identifiers
    /// let a: Id = "zri:file:::docs:index.md:".parse()?;
    /// let b: Id = "zri:file:::docs:about.md:".parse()?;
    /// assert!(a > b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        // Fast path - first, we compare for equality by using the precomputed
        // hashes, as it's a simple and extremely fast integer comparison
        if self.eq(other) {
            Ordering::Equal
        } else {
            self.format.cmp(&other.format)
        }
    }
}

// ----------------------------------------------------------------------------

impl Display for Id {
    /// Formats the identifier for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.format, f)
    }
}

impl Debug for Id {
    /// Formats the identifier for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Id")
            .field("provider", &self.provider())
            .field("resource", &self.resource())
            .field("variant", &self.variant())
            .field("context", &self.context())
            .field("location", &self.location())
            .field("fragment", &self.fragment())
            .finish()
    }
}
