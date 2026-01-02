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

//! Formatted string builder.

use std::array;
use std::borrow::Cow;

use super::encoding::encode;
use super::error::{Error, Result};
use super::path::validate;
use super::Format;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Formatted string builder.
#[derive(Clone, Debug)]
pub struct Builder<'a, const N: usize> {
    /// Formatted string source, if any.
    source: Option<&'a Format<N>>,
    /// Component values.
    values: [Option<Cow<'a, str>>; N],
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<const N: usize> Format<N> {
    /// Creates a formatted string builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string builder
    /// let mut builder = Format::<3>::builder();
    /// ```
    #[inline]
    #[must_use]
    pub fn builder<'a>() -> Builder<'a, N> {
        Builder {
            source: None,
            values: [const { None }; N],
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a, const N: usize> Builder<'a, N> {
    /// Updates the value at the given index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds. Since [`Format`] is a low-level
    /// construct, we don't expect this to happen, as indices should be known.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string builder and set value
    /// let mut builder = Format::<3>::builder().with(0, "a");
    /// ```
    #[inline]
    #[must_use]
    pub fn with<S>(mut self, index: usize, value: S) -> Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.set(index, value);
        self
    }

    /// Updates the value at the given index.
    ///
    /// This method accepts all types that can be converted into a reference to
    /// a string slice, most prominently [`str`] and [`String`].
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds. Since [`Format`] is a low-level
    /// construct, we don't expect this to happen, as indices should be known.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string builder and set value
    /// let mut builder = Format::<3>::builder();
    /// builder.set(0, "a");
    /// ```
    #[inline]
    pub fn set<S>(&mut self, index: usize, value: S) -> &mut Self
    where
        S: Into<Cow<'a, str>>,
    {
        self.values[index] = Some(value.into());
        self
    }

    /// Builds the formatted string.
    ///
    /// This method consumes the builder and constructs a [`Format`] from the
    /// provided values. If no value is set for a component, it's represented
    /// as an empty component in the resulting formatted string. [`Format::get`]
    /// returns [`None`] for such components.
    ///
    /// # Errors
    ///
    /// If a span overflows, [`Error::Overflow`] is returned.
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string builder
    /// let mut builder = Format::<3>::builder();
    /// builder.set(0, "a");
    /// builder.set(1, "b");
    /// builder.set(2, "c");
    ///
    /// // Create formatted string from builder
    /// let format = builder.build()?;
    ///
    /// // Obtain string representation
    /// assert_eq!(format.as_str(), "a:b:c");
    /// # Ok(())
    /// # }
    /// ```
    pub fn build(self) -> Result<Format<N>> {
        let mut spans = array::from_fn(|_| 0u16..0u16);
        let mut flags = 0;

        // Compute the minimum capacity by using the length from the formatted
        // string source, if any, or a reasonable default. This turns out to be
        // significantly faster than summing up individual component lengths,
        // especially if only a few components are modified.
        let capacity = self.source.map_or(64, |format| format.value.len());
        let mut buffer = Vec::with_capacity(capacity);

        // Write all components to the buffer, interspersed with `:` separators,
        // and percent-encode each component if it contains `:` characters
        for (index, opt) in self.values.into_iter().enumerate() {
            if index > 0 {
                buffer.push(b':');
            }

            // Compute the starting position of the current component, and make
            // sure the index fits into 16 bits, or return an overflow error
            let start =
                u16::try_from(buffer.len()).map_err(|_| Error::Overflow)?;

            // If no value is set for this component, but we have a formatted
            // string source, we can just copy the component from there, since
            // we can be sure that the encoding is already correct, if any, and
            // can thus skip encoding and validation
            if let (None, Some(format)) = (opt.as_ref(), self.source) {
                let p = format.spans[index].start as usize;
                let q = format.spans[index].end as usize;

                // Write component from formatted string source and compute the
                // ending position of the current component
                buffer.extend_from_slice(&format.value[p..q]);
                let end =
                    u16::try_from(buffer.len()).map_err(|_| Error::Overflow)?;

                // Store span for current component, and copy encoding flags
                // from formatted string source, since they are identical
                spans[index] = start..end;
                flags |= format.flags & (1 << index);
                continue;
            }

            // If no value is set for this component, we append a colon and set
            // the span to an empty range at the current position
            let Some(value) = opt else {
                spans[index] = start..start;
                continue;
            };

            // Percent-encode the current component, and remember if encoding is
            // necessary by inspecting if the encoded value is borrowed or owned
            let value = encode(value.as_bytes());
            match value {
                Cow::Borrowed(_) => flags &= !(1 << index),
                Cow::Owned(_) => flags |= 1 << index,
            }

            // Ensure that the given value is a valid path
            validate(&value)?;

            // Compute the ending position of the current component, and make
            // sure the index fits into 16 bits, or return an overflow error
            buffer.extend_from_slice(value.as_bytes());
            let end =
                u16::try_from(buffer.len()).map_err(|_| Error::Overflow)?;

            // Store span for current component
            spans[index] = start..end;
        }

        // Return formatted string
        Ok(Format {
            value: buffer.into(),
            spans,
            flags,
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<'a, const N: usize> From<&'a Format<N>> for Builder<'a, N> {
    /// Creates a formatted string builder from a formatted string.
    ///
    /// This implementation is primarily provided for [`Format::to_builder`],
    /// which allows convert a [`Format`] back into a builder.
    #[inline]
    fn from(format: &'a Format<N>) -> Self {
        Self {
            source: Some(format),
            values: [const { None }; N],
        }
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize> Default for Builder<'_, N> {
    /// Creates a formatted string builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::Builder;
    ///
    /// // Create formatted string builder
    /// let mut builder = Builder::<3>::default();
    /// ```
    #[inline]
    fn default() -> Self {
        Format::builder()
    }
}
