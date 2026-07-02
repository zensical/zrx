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

//! Sequence.

use std::iter::once;
use std::slice::Iter;
use std::vec::IntoIter;

use zrx_scheduler::Value;

mod element;
pub mod filter;

pub use element::Element;
pub use filter::Filter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Sequence.
///
/// Sequences are ordered collections of expressions that can be used to match
/// identifiers. Sequences can contain gaps, which allow to implement flexible
/// matching of identifiers with varying numbers of elements, including but
/// not limited to prefix and suffix matching.
///
/// Each [`Element`] of a [`Sequence`] can either be an [`Expression`][] or a
/// [`Gap`][]. An [`Expression`][] is a logical combination of one or multiple
/// [`Id`][] and [`Selector`][] instances, while a [`Gap`][] represents a
/// wildcard that can match any number of elements, including zero.
///
/// The following convenience constructors are provided:
///
/// - [`Sequence::prefix`]: Creates a sequence for prefix matching.
/// - [`Sequence::suffix`]: Creates a sequence for suffix matching.
///
/// [`Expression`]: crate::id::expression::Expression
/// [`Gap`]: crate::id::sequence::Element::Gap
/// [`Id`]: crate::id::Id
/// [`Selector`]: crate::id::selector::Selector
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::{selector, Sequence};
///
/// // Create sequence
/// let sequence = Sequence::from([
///     selector!(location = "zensical.toml")?,
///     selector!(location = "**/*.md")?,
/// ]);
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    /// Sequence elements.
    elements: Box<[Element]>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Sequence {
    /// Creates a sequence where the given elements are a prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence for a prefix
    /// let sequence = Sequence::prefix([
    ///     selector!(location = "zensical.toml")?,
    /// ]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn prefix<T>(sequence: T) -> Self
    where
        T: Into<Self>,
    {
        let sequence = sequence.into();
        let elements = sequence.into_iter().chain(once(Element::Gap));
        Self {
            elements: elements.collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    /// Creates a sequence where the given elements are a suffix.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence for a suffix
    /// let sequence = Sequence::suffix([
    ///     selector!(location = "**/*.md")?,
    /// ]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn suffix<T>(sequence: T) -> Self
    where
        T: Into<Self>,
    {
        let sequence = sequence.into();
        let elements = once(Element::Gap).chain(sequence);
        Self {
            elements: elements.collect::<Vec<_>>().into_boxed_slice(),
        }
    }

    /// Creates an iterator over the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]);
    ///
    /// // Create iterator over sequence
    /// for element in sequence.iter() {
    ///     println!("{element:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    #[must_use]
    pub fn iter(&self) -> Iter<'_, Element> {
        self.elements.iter()
    }
}

#[allow(clippy::must_use_candidate)]
impl Sequence {
    /// Returns the number of elements.
    #[inline]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Returns whether there are any elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for Sequence {}

// ----------------------------------------------------------------------------

impl<E> From<E> for Sequence
where
    E: Into<Element>,
{
    /// Creates a sequence from an element.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from(
    ///     selector!(location = "zensical.toml")?,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from(value: E) -> Self {
        Self::from_iter([value])
    }
}

impl<E, const N: usize> From<[E; N]> for Sequence
where
    E: Into<Element>,
{
    /// Creates a sequence from a slice of elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from(value: [E; N]) -> Self {
        Self::from_iter(value)
    }
}

impl<E> From<&[E]> for Sequence
where
    E: Into<Element> + Clone,
{
    /// Creates a sequence from a slice of elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from(&[
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ][..]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from(value: &[E]) -> Self {
        Self::from_iter(value.into_iter().cloned())
    }
}

impl<E> From<Vec<E>> for Sequence
where
    E: Into<Element>,
{
    /// Creates a sequence from a vector of elements.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from(vec![
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from(value: Vec<E>) -> Self {
        Self::from_iter(value)
    }
}

// ----------------------------------------------------------------------------

impl<E> FromIterator<E> for Sequence
where
    E: Into<Element>,
{
    /// Creates a sequence from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence from iterator
    /// let sequence = Sequence::from_iter([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = E>,
    {
        let elements = iter.into_iter().map(Into::into);
        Self {
            elements: elements.collect::<Vec<_>>().into_boxed_slice(),
        }
    }
}

impl<'a> IntoIterator for &'a Sequence {
    type Item = &'a Element;
    type IntoIter = Iter<'a, Element>;

    /// Creates an iterator over the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]);
    ///
    /// // Create iterator over sequence
    /// for element in &sequence {
    ///     println!("{element:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for Sequence {
    type Item = Element;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates a consuming iterator over the sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Sequence};
    ///
    /// // Create sequence
    /// let sequence = Sequence::from([
    ///     selector!(location = "zensical.toml")?,
    ///     selector!(location = "**/*.md")?,
    /// ]);
    ///
    /// // Create iterator over sequence
    /// for element in sequence {
    ///     println!("{element:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

// ----------------------------------------------------------------------------

impl Default for Sequence {
    /// Creates a sequence that matches everything.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::Sequence;
    ///
    /// // Create empty sequence
    /// let sequence = Sequence::default();
    /// assert!(!sequence.is_empty());
    /// ```
    #[inline]
    fn default() -> Self {
        Self::from(Element::Gap)
    }
}
