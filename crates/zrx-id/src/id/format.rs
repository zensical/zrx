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

//! Formatted string.

use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::str::{from_utf8_unchecked, FromStr};

pub mod container;
pub mod encoding;
mod error;
pub mod span;

use container::{Container, Recommended};
use encoding::{decode, encode};
pub use error::{Error, Result};
use span::{init, Span};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Formatted string.
///
/// This is a low-level construct which allows to create and manage strings that
/// contain a predefined number of values, all of which are separated with `:`.
/// If a value contains a `:` itself, it is percent-encoded, which is indicated
/// by a flag. This is slower, but not expected to be common.
///
/// Formatted strings are optimized for very fast conversion with [`FromStr`],
/// as well as cloning, since we only expect a few lookups but much more clones
/// for similar identifiers. Often, when strings are cloned, only a subset of
/// spans need to be changed, which is why we optimize for those cases.
///
/// This implementation is currently limited to 64 spans, which should probably
/// be sufficient for all use cases that can ever happen. For our means, an `u8`
/// would be more than enough, but since Rust will align the field to 64 bits
/// anyway, there's no point in being cheap.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
/// use zrx_id::format::Format;
///
/// // Create formatted string
/// let mut format = Format::<3>::new();
/// format.set(0, "a")?;
/// format.set(1, "b")?;
/// format.set(2, "c")?;
///
/// // Obtain string representation
/// assert_eq!(format.as_str(), "a:b:c");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Format<const N: usize, C = Recommended>
where
    C: Container,
{
    /// String representation.
    value: C,
    /// Set of spans.
    spans: [Span; N],
    /// Encoding flags.
    flags: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<const N: usize, C> Format<N, C>
where
    C: Container,
{
    /// Creates a formatted string.
    ///
    /// # Panics
    ///
    /// Panics if the span count is greater than 64.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string
    /// let mut format = Format::<3>::new();
    /// format.set(0, "a")?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new() -> Self {
        debug_assert!(N <= 64, "span count must be <= 64");
        Self {
            value: C::from(&[b':'; N][1..]), // N - 1
            spans: init::<N>(),
            flags: 0,
        }
    }

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
    /// construct, we don't expect this to happen, as indexes should be known.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string
    /// let mut format = Format::<3>::new();
    /// format.set(0, "a")?;
    ///
    /// // Obtain value at index
    /// let value = format.get(0);
    /// assert_eq!(value, "a");
    /// # Ok(())
    /// # }
    /// ```
    pub fn get(&self, index: usize) -> Cow<str> {
        let range: Range<_> = self.spans[index].into();
        if self.flags & (1 << index) == 0 {
            // SAFETY: The value is guaranteed to be valid UTF-8, as it was
            // created from a valid UTF-8 string. Additionally, the value is
            // not percent-encoded, so we can just return a borrowed reference
            // to the formatted string value, which is the common fast path.
            unsafe { Cow::Borrowed(from_utf8_unchecked(&self.value[range])) }
        } else {
            decode(&self.value[range])
        }
    }

    /// Updates the value at the given index.
    ///
    /// # Errors
    ///
    /// If the span overflows, [`Error::Length`] is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string and insert value
    /// let mut format = Format::<3>::new();
    /// format.set(0, "a")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn set<S>(&mut self, index: usize, value: S) -> Result
    where
        S: AsRef<[u8]>,
    {
        let value = encode(value.as_ref());

        // Now, check if the value is borrowed or owned. If it is borrowed, it
        // means that no encoding was necessary, and we can just return a slice
        // of the formatted string when required. Otherwise, at least one byte
        // was encoded, so we set the flag to indicate the need for decoding.
        match value {
            Cow::Borrowed(_) => self.flags &= !(1 << index),
            Cow::Owned(_) => self.flags |= 1 << index,
        }

        // Replace value in affected span
        self.value.splice(self.spans[index], value.as_ref());

        // Compute the difference in lengths of the new and prior value, as we
        // need to shift the end of the affected span, as well as the start and
        // end of all subsequent spans in order to maintain a valid format
        let by = i16::try_from(value.len())
            .ok()
            .and_then(|len| len.checked_sub_unsigned(self.spans[index].len()))
            .ok_or(Error::Length)?;

        // Shift affected span and all subsequent spans
        self.spans[index].shift_end(by)?;
        for i in (index + 1)..N {
            self.spans[i].shift(by)?;
        }

        // No errors occurred
        Ok(())
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
    /// // Create formatted string
    /// let mut format = Format::<3>::new();
    /// format.set(0, "a")?;
    /// format.set(1, "b")?;
    /// format.set(2, "c")?;
    ///
    /// // Obtain string representation
    /// assert_eq!(format.as_str(), "a:b:c");
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn as_str(&self) -> &str {
        // SAFETY: The value is guaranteed to be valid UTF-8, as it was created
        // from a valid UTF-8 string, so we can just return a borrowed reference
        unsafe { from_utf8_unchecked(&self.value) }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<const N: usize, C> FromStr for Format<N, C>
where
    C: Container,
{
    type Err = Error;

    /// Attempts to create a formatted string from a string.
    ///
    /// # Errors
    ///
    /// If the span count is off, [`Error::Cardinality`] is returned.
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
    /// # Ok(())
    /// # }
    /// ```
    fn from_str(value: &str) -> Result<Self> {
        let mut format = Format::new();
        format.value = C::from(value.as_bytes());

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
                    let end = u16::try_from(i).map_err(|_| Error::Length)?;

                    // Finalize current span
                    format.spans[index] = Span::new(start, end);
                    index += 1;

                    // Continue after separator
                    start = end + 1;
                    shift = 1 << index;
                }

                // If the current span contains a percent sign, and we haven't
                // already marked the span as percent-encoded, check if the next
                // two characters are valid hexadecimal digits. If so, mark it
                // as percent-encoded. Otherwise, proceed without modification.
                '%' if format.flags & shift == 0 => {
                    let bytes = value.as_bytes();
                    if let Some(&[b1, b2]) = bytes.get(i + 1..i + 3) {
                        if b1.is_ascii_hexdigit() && b2.is_ascii_hexdigit() {
                            format.flags |= shift;
                        }
                    }
                }

                // Consume all other characters
                _ => {}
            }
        }

        // Finalize last span
        let end = u16::try_from(value.len()).map_err(|_| Error::Length)?;
        format.spans[index] = Span::new(start, end);

        // Return format or error on incorrect span count
        if index == N - 1 {
            Ok(format)
        } else {
            Err(Error::Cardinality)
        }
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize, C> Hash for Format<N, C>
where
    C: Container,
{
    /// Hashes the formatted string.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize, C> PartialEq for Format<N, C>
where
    C: Container + Eq,
{
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

impl<const N: usize, C> Eq for Format<N, C> where C: Container + Eq {}

// ----------------------------------------------------------------------------

impl<const N: usize, C> PartialOrd for Format<N, C>
where
    C: Container + Ord,
{
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

impl<const N: usize, C> Ord for Format<N, C>
where
    C: Container + Ord,
{
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

impl<const N: usize, C> Default for Format<N, C>
where
    C: Container,
{
    /// Creates a formatted string.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_id::format::Format;
    ///
    /// // Create formatted string
    /// let mut format = Format::<3>::default();
    /// format.set(0, "a");
    /// ```
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------

impl<const N: usize, C> fmt::Display for Format<N, C>
where
    C: Container,
{
    /// Formats the formatted string for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<const N: usize, C> fmt::Debug for Format<N, C>
where
    C: Container,
{
    /// Formats the formatted string for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Format")
            .field("value", &self.as_str())
            .field("spans", &self.spans)
            .field("flags", &self.flags)
            .finish()
    }
}
