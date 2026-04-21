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

//! Key.

use ahash::AHasher;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::sync::Arc;

use super::value::Value;

mod error;
mod id;

pub use error::{Error, Result};
pub use id::Id;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Key.
///
/// Keys represent hierarchies of identifiers, which are a fundamental tool for
/// representing the hierarchical structure of computations and relations. A key
/// can consist of a single identifier, or arbitrary long chains of multiple
/// identifiers to model derivations and dependencies.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::Key;
///
/// // Create and transform key
/// let key = Key::from_iter([1, 2, 3]);
/// assert_eq!(
///     key.rotate_left(1),
///     Key::from_iter([2, 3, 1])
/// );
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Key<I> {
    /// Path.
    path: Arc<[I]>,
    /// Precomputed hash.
    hash: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Key<I> {
    /// Returns the identifier, if key has length 1.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`] if the key is empty, and [`Error::Depth`] if
    /// the key is deeper than one level, containing multiple identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::Key;
    ///
    /// // Create key
    /// let key = Key::from(42);
    ///
    /// // Obtain identifier
    /// assert_eq!(key.try_as_id()?, &42);
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn try_as_id(&self) -> Result<&I> {
        match &*self.path {
            [id] => Ok(id),
            [] => Err(Error::Empty),
            _ => Err(Error::Depth),
        }
    }
}

impl<I> Key<I>
where
    I: Id,
{
    /// Concatenates the key with another key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Key;
    ///
    /// // Create and concat keys
    /// let key = Key::from(1);
    /// let other = Key::from_iter([2, 3]);
    /// assert_eq!(
    ///     key.concat(&other),
    ///     Key::from_iter([1, 2, 3])
    /// );
    /// ```
    #[must_use]
    pub fn concat(&self, other: &Self) -> Self {
        let iter = self.path.iter().chain(other.path.iter());
        let path = iter.cloned().collect();
        Self { hash: hash(&path), path }
    }

    /// Reverses the key.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Key;
    ///
    /// // Create and transform key
    /// let key = Key::from_iter([1, 2, 3]);
    /// assert_eq!(
    ///     key.reverse(),
    ///     Key::from_iter([3, 2, 1])
    /// );
    /// ```
    #[must_use]
    pub fn reverse(&self) -> Self {
        let mut path = self.path.to_vec();
        path.reverse();
        Self {
            hash: hash(&path),
            path: Arc::from(path),
        }
    }

    /// Rotates the key left by `n` positions.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Key;
    ///
    /// // Create and transform key
    /// let key = Key::from_iter([1, 2, 3]);
    /// assert_eq!(
    ///     key.rotate_left(1),
    ///     Key::from_iter([2, 3, 1])
    /// );
    /// ```
    #[must_use]
    pub fn rotate_left(&self, n: usize) -> Self {
        let mut path = self.path.to_vec();
        path.rotate_left(n);
        Self {
            hash: hash(&path),
            path: Arc::from(path),
        }
    }

    /// Rotates the key right by `n` positions.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Key;
    ///
    /// // Create and transform key
    /// let key = Key::from_iter([1, 2, 3]);
    /// assert_eq!(
    ///     key.rotate_right(1),
    ///     Key::from_iter([3, 1, 2])
    /// );
    /// ```
    #[must_use]
    pub fn rotate_right(&self, n: usize) -> Self {
        let mut path = self.path.to_vec();
        path.rotate_right(n);
        Self {
            hash: hash(&path),
            path: Arc::from(path),
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Value for Key<I> where I: Value {}

// ----------------------------------------------------------------------------

impl<I> From<I> for Key<I>
where
    I: Id,
{
    /// Creates a key from an identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Key;
    ///
    /// // Create key
    /// let key = Key::from(42);
    /// ```
    #[inline]
    fn from(id: I) -> Self {
        let path = Arc::from([id]);
        Self { hash: hash(&path), path }
    }
}

// ----------------------------------------------------------------------------

impl<I> FromIterator<I> for Key<I>
where
    I: Id,
{
    /// Creates a key from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Key;
    ///
    /// // Create key from iterator
    /// let key = Key::from_iter([1, 2, 3]);
    /// ```
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        let path = iter.into_iter().collect();
        Self { hash: hash(&path), path }
    }
}

// ----------------------------------------------------------------------------

impl<I> Index<usize> for Key<I> {
    type Output = I;

    /// Returns a reference to the identifier at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

// ----------------------------------------------------------------------------

impl<I> Hash for Key<I> {
    /// Hashes the key.
    ///
    /// Since key paths are immutable, we can use a precomputed hash for fast
    /// hashing. This is especially useful when key paths are used as keys in
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

impl<I> Display for Key<I>
where
    I: Display,
{
    /// Formats the key for display.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, item) in self.path.iter().enumerate() {
            Display::fmt(item, f)?;

            // Write slash if not last
            if i < self.path.len() - 1 {
                f.write_str(" / ")?;
            }
        }

        // No errors occurred
        Ok(())
    }
}

impl<I> Debug for Key<I>
where
    I: Debug,
{
    // Formats the key for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entry(&self.path).finish()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Precomputes the hash for the given path - this is used for fast hashing of
/// keys, since key paths are immutable, meaning hashes can be precomputed.
#[inline]
fn hash<P>(path: &P) -> u64
where
    P: Hash,
{
    let mut hasher = AHasher::default();
    path.hash(&mut hasher);
    hasher.finish()
}
