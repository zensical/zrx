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

//! Output collection.

use std::fmt;
use std::vec::IntoIter;

use super::Output;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Output collection.
///
/// This data type represents a collection of outputs, which defines conversion
/// traits for various types, such as [`Option`], [`Vec`], and slices, allowing
/// to return multiple outputs from an action, which can be of different types,
/// as long as they implement the [`Into`] conversion trait for [`Output`].
///
/// # Examples
///
/// ```
/// use zrx_scheduler::action::Outputs;
/// use zrx_scheduler::effect::Item;
///
/// // Create output collection
/// let outputs = Outputs::from([
///     Item::new("a", Some(1)),
///     Item::new("b", Some(2)),
///     Item::new("c", Some(3)),
/// ]);
/// ```
pub struct Outputs<I> {
    /// Vector of outputs.
    inner: Vec<Output<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Outputs<I> {
    /// Extends the output collection with the given outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create output collection
    /// let outputs = Outputs::default().with([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    /// ```
    #[inline]
    #[must_use]
    pub fn with<O>(mut self, outputs: O) -> Self
    where
        O: IntoIterator,
        O::Item: Into<Output<I>>,
    {
        for output in outputs {
            self.inner.push(output.into());
        }
        self
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Outputs<I> {
    /// Returns the number of outputs.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are any outputs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> From<()> for Outputs<I> {
    /// Creates an output collection from the unit value.
    ///
    /// This implementation makes the API more flexible, as it allows to just
    /// return nothing from an action, which is necessary from time to time.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    ///
    /// // Create output collection from unit value
    /// let outputs = Outputs::from(());
    /// # let _: Outputs<()> = outputs;
    /// ```
    #[inline]
    fn from((): ()) -> Self {
        Self::default()
    }
}

impl<I, O> From<O> for Outputs<I>
where
    O: Into<Output<I>>,
{
    /// Creates an output collection from an output.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from output
    /// let outputs = Outputs::from(item);
    /// ```
    #[inline]
    fn from(value: O) -> Self {
        Self::from_iter(Some(value.into()))
    }
}

impl<I, O> From<Option<O>> for Outputs<I>
where
    O: Into<Output<I>>,
{
    /// Creates an output collection from an optional output.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from output option
    /// let outputs = Outputs::from(Some(item));
    /// ```
    #[inline]
    fn from(value: Option<O>) -> Self {
        Self::from_iter(value)
    }
}

impl<I, O, const N: usize> From<[O; N]> for Outputs<I>
where
    O: Into<Output<I>>,
{
    /// Creates an output collection from a slice of outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from output slice
    /// let outputs = Outputs::from([item]);
    /// ```
    #[inline]
    fn from(value: [O; N]) -> Self {
        Self::from_iter(value)
    }
}

impl<I, O> From<Vec<O>> for Outputs<I>
where
    O: Into<Output<I>>,
{
    /// Creates an output collection from a vector of outputs.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from output vector
    /// let outputs = Outputs::from(vec![item]);
    /// ```
    #[inline]
    fn from(value: Vec<O>) -> Self {
        Self::from_iter(value)
    }
}

// ----------------------------------------------------------------------------

impl<I, O> FromIterator<O> for Outputs<I>
where
    O: Into<Output<I>>,
{
    /// Creates an output collection from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create output collection from iterator
    /// let outputs = Outputs::from_iter([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = O>,
    {
        Self {
            inner: iter.into_iter().map(Into::into).collect(),
        }
    }
}

impl<I> IntoIterator for Outputs<I> {
    type Item = Output<I>;
    type IntoIter = IntoIter<Self::Item>;

    /// Creates an iterator over the output collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create output collection from iterator
    /// let outputs = Outputs::from_iter([
    ///     Item::new("a", Some(1)),
    ///     Item::new("b", Some(2)),
    ///     Item::new("c", Some(3)),
    /// ]);
    ///
    /// // Create iterator over outputs
    /// for output in outputs {
    ///     println!("{output:?}");
    /// }
    /// ```
    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Outputs<I> {
    /// Creates an output collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::Outputs;
    ///
    /// // Create output collection
    /// let outputs = Outputs::default();
    /// # let _: Outputs<()> = outputs;
    /// ```
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}

// ----------------------------------------------------------------------------

impl<I> fmt::Debug for Outputs<I>
where
    I: fmt::Debug,
{
    /// Formats the output collection for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Outputs")
            .field("inner", &self.inner)
            .finish()
    }
}
