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

//! Selector.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use crate::format::Format;
use crate::path::validate;

use super::error::{Error, Result};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Convert to [`Selector`].
///
/// This trait allows to convert an arbitrary value into a selector, using a
/// [`Cow`] smart pointer to avoid unnecessary cloning, e.g. for references.
pub trait ToSelector {
    /// Creates a selector.
    #[allow(clippy::missing_errors_doc)]
    fn to_selector(&self) -> Result<Cow<Selector>>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Selector.
///
/// Selectors are similar to identifiers, but are used to match identifiers in
/// the system. They do not require any component to contain a value, and allow
/// to define a glob for any of its components. Empty components are always
/// considered wildcards, which means they match any value.
///
/// Slashes can be used inside each component to model hierarchical concepts
/// like paths. Backslashes must first be normalized to slashes by the caller
/// to unify behavior among different operating system, ensuring that caches
/// are portable. This is also why every method that takes a value will return
/// [`Error::Backslash`][] if a backslash is found. Thus, the caller should
/// use a library like [`path-slash`][] for consistent normalization.
///
/// Selectors are no means to an end, but rather a building block to associate
/// data or functions to identifiers via the construction of a [`Matcher`][],
/// which uses an efficient algorithm to match an arbitrary set of selectors in
/// linear time. While it's advisable to use [`Selector::new`] together with the
/// associated builder methods to create a new selector, selectors can also be
/// created from a structured string representation with [`Selector::from_str`],
/// which is used internally for serializing them to persistent storage:
///
/// ``` text
/// zrs:<scheme>:<binding>:<context>:<path>:<fragment>
/// ```
///
/// The decision to use a structured string representation as a data model was
/// made to allow for blazing fast cloning and derivation of new selectors.
///
/// [`Error::Backslash`]: crate::path::Error::Backslash
/// [`path-slash`]: https://crates.io/crates/path-slash
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
/// // Create selector and set path
/// let mut selector = Selector::new()?;
/// selector.set_path("**/*.md")?;
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
/// let selector: Selector = "zrs::::**/*.md:".parse()?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Selector {
    /// Formatted string.
    format: Format<6>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Selector {
    /// Creates a selector.
    ///
    /// # Errors
    ///
    /// This method is infallible, but we're synchronizing the signature with
    /// the fallible method [`Id::new`][] for reasons of concistency.
    ///
    /// [`Id::new`]: crate::Id::new
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::Selector;
    ///
    /// // Create selector and set path
    /// let mut selector = Selector::new()?;
    /// selector.set_path("**/*.md")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn new() -> Result<Self> {
        Ok(Self { format: "zrs:::::".parse()? })
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
    /// use zrx_id::Selector;
    ///
    /// // Create selector and set scheme
    /// let mut selector = Selector::new()?;
    /// selector.set_scheme("git")?;
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
    /// use zrx_id::Selector;
    ///
    /// // Create selector and set binding
    /// let mut selector = Selector::new()?;
    /// selector.set_binding("master")?;
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
    /// use zrx_id::Selector;
    ///
    /// // Create selector and set context
    /// let mut selector = Selector::new()?;
    /// selector.set_context("examples")?;
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
    /// use zrx_id::Selector;
    ///
    /// // Create selector and set path
    /// let mut selector = Selector::new()?;
    /// selector.set_path("**/*.md")?;
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
    /// use zrx_id::Selector;
    ///
    /// // Create selector and set fragment
    /// let mut selector = Selector::new()?;
    /// selector.set_fragment("anchor")?;
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
impl Selector {
    /// Returns the `scheme` component, if any.
    #[inline]
    pub fn scheme(&self) -> Option<Cow<str>> {
        Some(self.format.get(1)).filter(|value| !value.is_empty())
    }

    /// Returns the `binding` component, if any.
    #[inline]
    pub fn binding(&self) -> Option<Cow<str>> {
        Some(self.format.get(2)).filter(|value| !value.is_empty())
    }

    /// Returns the `context` component, if any.
    #[inline]
    pub fn context(&self) -> Option<Cow<str>> {
        Some(self.format.get(3)).filter(|value| !value.is_empty())
    }

    /// Returns the `path` component, if any.
    #[inline]
    pub fn path(&self) -> Option<Cow<str>> {
        Some(self.format.get(4)).filter(|value| !value.is_empty())
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

impl ToSelector for &Selector {
    /// Creates a selector from a reference.
    #[inline]
    fn to_selector(&self) -> Result<Cow<Selector>> {
        Ok(Cow::Borrowed(self))
    }
}

impl ToSelector for &str {
    /// Creates a selector from a string.
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
    /// use zrx_id::{Selector, ToSelector};
    ///
    /// // Create selector from string
    /// let selector = "zrs::::**/*.md:".to_selector()?;
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn to_selector(&self) -> Result<Cow<Selector>> {
        self.parse().map(Cow::Owned)
    }
}

// ----------------------------------------------------------------------------

impl FromStr for Selector {
    type Err = Error;

    /// Creates a selector from a string.
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
    /// use zrx_id::Selector;
    ///
    /// // Create selector from string
    /// let selector: Selector = "zrs::::**/*.md:".parse()?;
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        let format = Format::from_str(validate(value)?)?;

        // Ensure prefix is valid
        if format.get(0) != "zrs" {
            Err(Error::Prefix)?;
        }

        // No errors occurred
        Ok(Self { format })
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for Selector {
    /// Formats the selector for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.format.fmt(f)
    }
}

impl fmt::Debug for Selector {
    /// Formats the selector for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Selector")
            .field("scheme", &self.scheme())
            .field("binding", &self.binding())
            .field("context", &self.context())
            .field("path", &self.path())
            .field("fragment", &self.fragment())
            .finish()
    }
}
