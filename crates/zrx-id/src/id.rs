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

//! Identifier.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

mod error;
pub mod format;
pub mod matcher;
pub mod path;

pub use error::{Error, Result};
use format::encoding::encode;
use format::Format;
use path::validate;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Convert to [`Id`].
///
/// This trait allows to convert an arbitrary value into an identifier, using a
/// [`Cow`] smart pointer to avoid unnecessary cloning, e.g. for references.
pub trait ToId {
    /// Creates an identifier.
    #[allow(clippy::missing_errors_doc)]
    fn to_id(&self) -> Result<Cow<Id>>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Identifier.
///
/// Identifiers are structured representations to uniquely identify resources
/// within the system, and are modelled using a structured, yet human-readable,
/// compact string representation. Every identifier consists of the following
/// five components:
///
/// - `scheme`: The scheme of the resource , e.g., file or git.
/// - `binding`: The binding of the resource , e.g., volume, branch or tag.
/// - `context`: The context of the resource, e.g., source or output directory.
/// - `path`: The path to the resource, e.g., file or folder to resolve.
/// - `fragment`: The fragment of the resource, e.g., line number or anchor.
///
/// Slashes can be used inside each component to model hierarchical concepts
/// like paths. Backslashes must first be normalized to slashes by the caller
/// to unify behavior among different operating system, ensuring that caches
/// are portable. This is also why every method that takes a value will return
/// [`Error::Backslash`][] if a backslash is found. Thus, the caller should
/// use a library like [`path-slash`][] for consistent normalization.
///
/// Identifiers implement [`Eq`], [`PartialEq`] and [`Hash`], as well as [`Ord`]
/// and [`PartialOrd`], as they are used in events that move through the system,
/// which are stored in hash maps and similar constructs. The structured string
/// representation is defined as follows:
///
/// ``` text
/// zri:<scheme>:<binding>:<context>:<path>:<fragment>
/// ```
///
/// The decision to use a structured string representation as a data model was
/// made to allow for blazing fast cloning and derivation of new identifiers.
///
/// [`Error::Backslash`]: crate::path::Error::Backslash
/// [`path-slash`]: https://crates.io/crates/path-slash
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
/// // Create identifier
/// let id = Id::new("file", "docs", "index.md")?;
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
/// let id: Id = "zri:file::docs:index.md:".parse()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id {
    /// Formatted string.
    format: Format<6>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Id {
    /// Creates an identifier.
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier
    /// let id = Id::new("file", "docs", "index.md")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new<S, C, P>(scheme: S, context: C, path: P) -> Result<Self>
    where
        S: AsRef<[u8]>,
        C: AsRef<[u8]>,
        P: AsRef<[u8]>,
    {
        // We must check if any of the values contains a `:` separator, so pass
        // them through the encoder first, which will be a no-op in most cases
        let scheme = encode(validate(scheme.as_ref())?);
        let context = encode(validate(context.as_ref())?);
        let path = encode(validate(path.as_ref())?);

        // Compute required capacity
        let capacity = 8 + scheme.len() + context.len() + path.len();

        // Create identifier by appending each component with `:` separators
        // to a string buffer instead of using the `format!` macro, and parse
        // the string, instead of setting the components on the formatted
        // string one after another, yielding a 5x performance increase
        let mut buffer = String::with_capacity(capacity);
        buffer.push_str("zri:");
        buffer.push_str(scheme.as_ref());
        buffer.push_str("::");
        buffer.push_str(context.as_ref());
        buffer.push(':');
        buffer.push_str(path.as_ref());
        buffer.push(':');

        // Return identifier after parsing formatted string
        buffer
            .parse()
            .map_err(Into::into)
            .map(|format| Self { format })
    }

    /// Updates the `scheme` component.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Path`], if the component value contains a
    /// backslash, or [`Error::Format`], if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier and set scheme
    /// let mut id = Id::new("file", "docs", "index.md")?;
    /// id.set_scheme("git")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_scheme<S>(&mut self, scheme: S) -> Result<&mut Self>
    where
        S: AsRef<[u8]>,
    {
        self.format
            .set(1, validate(scheme)?)
            .map_err(Into::into)
            .map(|()| self)
    }

    /// Updates the `binding` component.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Path`], if the component value contains a
    /// backslash, or [`Error::Format`], if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier and set binding
    /// let mut id = Id::new("git", "docs", "index.md")?;
    /// id.set_binding("master")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_binding<S>(&mut self, binding: S) -> Result<&mut Self>
    where
        S: AsRef<[u8]>,
    {
        self.format
            .set(2, validate(binding)?)
            .map_err(Into::into)
            .map(|()| self)
    }

    /// Updates the `context` component.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Path`], if the component value contains a
    /// backslash, or [`Error::Format`], if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier and set context
    /// let mut id = Id::new("file", "docs", "index.md")?;
    /// id.set_context("examples")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_context<S>(&mut self, context: S) -> Result<&mut Self>
    where
        S: AsRef<[u8]>,
    {
        self.format
            .set(3, validate(context)?)
            .map_err(Into::into)
            .map(|()| self)
    }

    /// Updates the `path` component.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Path`], if the component value contains a
    /// backslash, or [`Error::Format`], if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier and set path
    /// let mut id = Id::new("file", "docs", "index.md")?;
    /// id.set_path("README.md")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_path<S>(&mut self, path: S) -> Result<&mut Self>
    where
        S: AsRef<[u8]>,
    {
        self.format
            .set(4, validate(path)?)
            .map_err(Into::into)
            .map(|()| self)
    }

    /// Updates the `fragment` component.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Path`], if the component value contains a
    /// backslash, or [`Error::Format`], if the format is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Id;
    ///
    /// // Create identifier and set fragment
    /// let mut id = Id::new("file", "docs", "index.md")?;
    /// id.set_fragment("anchor")?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn set_fragment<S>(&mut self, fragment: S) -> Result<&mut Self>
    where
        S: AsRef<[u8]>,
    {
        self.format
            .set(5, validate(fragment)?)
            .map_err(Into::into)
            .map(|()| self)
    }
}

#[allow(clippy::must_use_candidate)]
impl Id {
    /// Returns the `scheme` component.
    #[inline]
    pub fn scheme(&self) -> Cow<str> {
        self.format.get(1)
    }

    /// Returns the `binding` component, if any.
    #[inline]
    pub fn binding(&self) -> Option<Cow<str>> {
        Some(self.format.get(2)).filter(|value| !value.is_empty())
    }

    /// Returns the `context` component.
    #[inline]
    pub fn context(&self) -> Cow<str> {
        self.format.get(3)
    }

    /// Returns the `path` component.
    #[inline]
    pub fn path(&self) -> Cow<str> {
        self.format.get(4)
    }

    /// Returns the `fragment` component, if any.
    #[inline]
    pub fn fragment(&self) -> Option<Cow<str>> {
        Some(self.format.get(5)).filter(|value| !value.is_empty())
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl ToId for &Id {
    /// Creates an identifier from a reference.
    #[inline]
    fn to_id(&self) -> Result<Cow<Id>> {
        Ok(Cow::Borrowed(self))
    }
}

impl ToId for &str {
    /// Creates an identifier from a string.
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
    /// use zrx_id::{Id, ToId};
    ///
    /// // Create identifier from string
    /// let id = "zri:file::docs:index.md:".to_id()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_id(&self) -> Result<Cow<Id>> {
        self.parse().map(Cow::Owned)
    }
}

// ----------------------------------------------------------------------------

impl FromStr for Id {
    type Err = Error;

    /// Attempts to create an identifier from a string.
    ///
    /// The string must adhere to the following format and include exactly five
    /// `:` separators, even if some components are omitted. Only the `binding`
    /// and `fragment` components are optional and can be left empty, all other
    /// components must be present:
    ///
    /// ``` text
    /// zri:<scheme>:<binding>:<context>:<path>:<fragment>
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
    /// use zrx_id::Id;
    ///
    /// // Create identifier from string
    /// let id: Id = "zri:file::docs:index.md:".parse()?;
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        let format = Format::from_str(validate(value)?)?;

        // Ensure prefix is valid
        if format.get(0) != "zri" {
            Err(Error::Prefix)?;
        }

        // Ensure scheme is set
        if format.get(1).is_empty() {
            Err(Error::Component("scheme"))?;
        }

        // Ensure context is set
        if format.get(3).is_empty() {
            Err(Error::Component("context"))?;
        }

        // Ensure path is set
        if format.get(4).is_empty() {
            Err(Error::Component("path"))?;
        }

        // No errors occurred
        Ok(Self { format })
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for Id {
    /// Formats the identifier for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format.fmt(f)
    }
}

impl fmt::Debug for Id {
    /// Formats the identifier for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Id")
            .field("scheme", &self.scheme())
            .field("binding", &self.binding())
            .field("context", &self.context())
            .field("path", &self.path())
            .field("fragment", &self.fragment())
            .finish()
    }
}
