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

//! Specified value.

use std::cmp::Ordering;
use std::ops::Deref;

use super::convert::ToSpecificity;
use super::Specificity;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Specified value.
///
/// This data type is a thin wrapper around a value of type `T`, augmenting the
/// value with its computed [`Specificity`], so it can be efficiently ordered.
#[derive(Clone, Debug)]
pub struct Specified<T> {
    /// Inner value.
    inner: T,
    /// Specificity.
    specificity: Specificity,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Specified<T> {
    /// Returns the inner value, consuming the specified value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::Specified;
    ///
    /// // Create selector
    /// let selector = selector!(location = "**/*.md")?;
    ///
    /// // Create specified value from selector
    /// let value = Specified::from(selector.clone());
    /// assert_eq!(value.into_inner(), selector);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn into_inner(self) -> T {
        self.inner
    }
}

#[allow(clippy::must_use_candidate)]
impl<T> Specified<T> {
    /// Returns the specificity.
    #[inline]
    pub fn specificity(self) -> Specificity {
        self.specificity
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<T> for Specified<T>
where
    T: ToSpecificity,
{
    /// Creates a specified value from a value.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::Specified;
    ///
    /// // Create selector
    /// let selector = selector!(location = "**/*.md")?;
    ///
    /// // Create specified value from selector
    /// let value = Specified::from(selector);
    /// assert_eq!(value.specificity(), (0, 1, 1, 3).into());
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from(inner: T) -> Self {
        let specificity = inner.to_specificity();
        Specified { inner, specificity }
    }
}

// ----------------------------------------------------------------------------

impl<T> Deref for Specified<T> {
    type Target = T;

    /// Dereferences to the inner value.
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// ----------------------------------------------------------------------------

impl<T> PartialEq for Specified<T>
where
    T: PartialEq,
{
    /// Compares two specified values for equality.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::Specified;
    ///
    /// // Create and compare selectors
    /// let a = Specified::from(selector!(location = "*.md")?);
    /// let b = Specified::from(selector!(location = "*.md")?);
    /// assert_eq!(a, b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> Eq for Specified<T> where T: Eq {}

// ----------------------------------------------------------------------------

impl<T> PartialOrd for Specified<T>
where
    T: Eq,
{
    /// Orders two values by specificity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::Specified;
    ///
    /// // Create and compare selectors by specificity
    /// let a = Specified::from(selector!(location = "**/*.md")?);
    /// let b = Specified::from(selector!(location = "*.md")?);
    /// assert!(a < b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for Specified<T>
where
    T: Eq,
{
    /// Orders two values by specificity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::selector;
    /// use zrx_id::specificity::Specified;
    ///
    /// // Create and compare selectors by specificity
    /// let a = Specified::from(selector!(location = "**/*.md")?);
    /// let b = Specified::from(selector!(location = "*.md")?);
    /// assert!(a < b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.specificity.cmp(&other.specificity)
    }
}
