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

//! Formatted string.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::str::{from_utf8_unchecked, FromStr};
use std::{array, fmt};

mod builder;
mod encoding;
mod error;
mod path;

pub use builder::Builder;
use encoding::decode;
pub use error::{Error, Result};
use path::validate;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Formatted string.
///
/// This is a low-level construct which allows to efficiently work with strings
/// that contain a predefined number of components separated by `:` characters.
/// If a component contains a `:` itself, it is percent-encoded, indicated by
/// a flag. This is slower, but not expected to be common.
///
/// Formatted strings are optimized for very fast conversion with [`FromStr`]
/// or gradual construction through [`Format::builder`], which both produce an
/// immutable instance. Note that encapsulating types like [`Selector`][] and
/// [`Id`][] wrap the formatted string in an [`Arc`][] to provide fast cloning.
///
/// This implementation is currently limited to 64 spans, which should probably
/// be sufficient for all use cases that can ever happen. For our means, an `u8`
/// would be more than enough, but since Rust will align the field to 64 bits
/// anyway, there's no point in being cheap.
///
/// [`Arc`]: std::sync::Arc
/// [`Id`]: crate::id::Id
/// [`Selector`]: crate::id::matcher::selector::Selector
///
/// # Examples
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
#[derive(Clone)]
pub struct Format<const N: usize> {
    /// String representation.
    value: Box<[u8]>,
    /// Set of spans.
    spans: [Range<u16>; N],
    /// Encoding flags.
    flags: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<const N: usize> Format<N> {
    /// Returns the value at the given index.
    ///
    /// If the value is not percent-encoded, which means it does not contain a
    /// `:` character, a borrowed reference is returned which is essentially a
    /// zero-cost operation and expected to be the common case. Otherwise, the
    /// value is percent-decoded and an owned value is returned.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds. Since [`Format`] is a low-level
    /// construct, we don't expect this to happen, as indices should be known.
    ///
    /// # Examples
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
    /// // Obtain value at index
    /// assert_eq!(format.get(0), "a");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn get(&self, index: usize) -> Cow<'_, str> {
        let p = self.spans[index].start as usize;
        let q = self.spans[index].end as usize;
        if self.flags & (1 << index) == 0 {
            // SAFETY: The value is guaranteed to be valid UTF-8, as it was
            // created from a valid UTF-8 string. Additionally, the value is
            // not percent-encoded, so we can just return a borrowed reference
            // to the formatted string value, which is the common fast path.
            unsafe { Cow::Borrowed(from_utf8_unchecked(&self.value[p..q])) }
        } else {
            decode(&self.value[p..q])
        }
    }

    /// Creates an iterator over all components.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string from string
    /// let format: Format::<3> = "a:b:c".parse()?;
    ///
    /// // Create iterator over components
    /// for component in format.iter() {
    ///     println!("{component:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = Cow<'_, str>> {
        (0..N).map(|index| self.get(index))
    }

    /// Returns the string representation.
    ///
    /// # Examples
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
    #[inline]
    #[must_use]
    pub fn as_str(&self) -> &str {
        // SAFETY: The value is guaranteed to be valid UTF-8, as it was created
        // from valid UTF-8 strings, so we can just return a borrowed reference
        unsafe { from_utf8_unchecked(&self.value) }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<const N: usize> FromStr for Format<N> {
    type Err = Error;

    /// Attempts to create a formatted string from a string.
    ///
    /// # Errors
    ///
    /// If the span number is off, [`Error::Mismatch`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string from string
    /// let format: Format::<3> = "a:b:c".parse()?;
    /// assert_eq!(format.as_str(), "a:b:c");
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        let mut spans = array::from_fn(|_| 0u16..0u16);
        let mut flags = 0;

        // Initialize start and span counter
        let mut start = 0u16;
        let mut index = 0;
        let mut shift = 1;

        // Compute spans from characters
        for (i, char) in value.char_indices() {
            match char {
                // If the current character is a separator, finalize the span.
                // It's very unlikely that any conversion here results in an
                // error, but since identifiers might potentially contain user
                // data, we handle it and return an error.
                ':' => {
                    let end = u16::try_from(i).map_err(|_| Error::Overflow)?;
                    validate(&value[start.into()..end.into()])?;

                    // Finalize current span
                    spans[index] = start..end;
                    index += 1;

                    // Continue after separator
                    start = end + 1;
                    shift = 1 << index;
                }

                // If the current span contains a percent sign, and we haven't
                // already marked the span as percent-encoded, check if the next
                // two characters are valid hexadecimal digits. If so, mark it
                // as percent-encoded. Otherwise, proceed without modification.
                '%' if flags & shift == 0 => {
                    let bytes = value.as_bytes();
                    if let Some(&[b1, b2]) = bytes.get(i + 1..i + 3) {
                        if b1.is_ascii_hexdigit() && b2.is_ascii_hexdigit() {
                            flags |= shift;
                        }
                    }
                }

                // Consume all other characters
                _ => {}
            }
        }

        // Finalize last span
        let end = u16::try_from(value.len()).map_err(|_| Error::Overflow)?;
        spans[index] = start..end;

        // Return format or error on number mismatch
        if index == N - 1 {
            Ok(Format {
                value: value.as_bytes().into(),
                spans,
                flags,
            })
        } else {
            Err(Error::Mismatch)
        }
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize> Hash for Format<N> {
    /// Hashes the formatted string.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize> PartialEq for Format<N> {
    /// Compares two formatted strings for equality.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create and compare formatted strings
    /// let a: Format::<3> = "a:b:c".parse()?;
    /// let b: Format::<3> = "a:b:c".parse()?;
    /// assert_eq!(a, b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<const N: usize> Eq for Format<N> {}

// ----------------------------------------------------------------------------

impl<const N: usize> PartialOrd for Format<N> {
    /// Orders two formatted strings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create and compare formatted strings
    /// let a: Format::<3> = "b:c:d".parse()?;
    /// let b: Format::<3> = "a:b:c".parse()?;
    /// assert!(a > b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<const N: usize> Ord for Format<N> {
    /// Orders two formatted strings.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create and compare formatted strings
    /// let a: Format::<3> = "b:c:d".parse()?;
    /// let b: Format::<3> = "a:b:c".parse()?;
    /// assert!(a > b);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize> fmt::Display for Format<N> {
    /// Formats the formatted string for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize> fmt::Debug for Format<N> {
    /// Formats the formatted string for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Format")
            .field("value", &self.as_str())
            .field("spans", &self.spans)
            .field("flags", &self.flags)
            .finish()
    }
}
