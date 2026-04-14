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

//! Scope.

use ahash::AHasher;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::sync::Arc;

use super::Id;

mod error;

pub use error::{Error, Result};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scope.
///
/// Scopes represent hierarchies of identifiers, which are a fundamental tool
/// for representing the hierarchical structure of computations and relations.
/// A scope can consist of a single identifier, or arbitrary long chains of
/// multiple identifiers.
///
/// # Examples
///
/// ```
/// use zrx_scheduler::Scope;
///
/// // Create and transform scope
/// let scope = Scope::from_iter([1, 2, 3]);
/// assert_eq!(
///     scope.rotate_left(1),
///     Scope::from_iter([2, 3, 1])
/// );
/// ```
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Scope<I> {
    /// Path.
    path: Arc<[I]>,
    /// Precomputed hash.
    hash: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Scope<I>
where
    I: Id,
{
    /// Returns the identifier, if scope has length 1.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`] if the scope is empty, and [`Error::Depth`] if
    /// the scope is deeper than one level, containing multiple identifiers.
    ///
    /// # Examples
    ///
    /// ```
    /// // @todo
    /// ```
    #[inline]
    pub fn try_as_id(&self) -> Result<&I> {
        match &*self.path {
            [id] => Ok(id),
            [] => Err(Error::Empty),
            _ => Err(Error::Depth),
        }
    }

    /// Concatenates the scope with another scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Scope;
    ///
    /// // Create and concat scopes
    /// let scope = Scope::from(1);
    /// let other = Scope::from_iter([2, 3]);
    /// assert_eq!(
    ///     scope.concat(&other),
    ///     Scope::from_iter([1, 2, 3])
    /// );
    /// ```
    #[must_use]
    pub fn concat(&self, other: &Self) -> Self {
        let iter = self.path.iter().chain(other.path.iter());
        let path = iter.cloned().collect();
        Self { hash: hash(&path), path }
    }

    /// Reverses the scope.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Scope;
    ///
    /// // Create and transform scope
    /// let scope = Scope::from_iter([1, 2, 3]);
    /// assert_eq!(
    ///     scope.reverse(),
    ///     Scope::from_iter([3, 2, 1])
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn reverse(&self) -> Self {
        let mut path = self.path.to_vec();
        path.reverse();
        Self {
            hash: hash(&path),
            path: Arc::from(path),
        }
    }

    /// Rotates the scope left by `n` positions.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Scope;
    ///
    /// // Create and transform scope
    /// let scope = Scope::from_iter([1, 2, 3]);
    /// assert_eq!(
    ///     scope.rotate_left(1),
    ///     Scope::from_iter([2, 3, 1])
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn rotate_left(&self, n: usize) -> Self {
        let mut path = self.path.to_vec();
        path.rotate_left(n);
        Self {
            hash: hash(&path),
            path: Arc::from(path),
        }
    }

    /// Rotates the scope right by `n` positions.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Scope;
    ///
    /// // Create and transform scope
    /// let scope = Scope::from_iter([1, 2, 3]);
    /// assert_eq!(
    ///     scope.rotate_right(1),
    ///     Scope::from_iter([3, 1, 2])
    /// );
    /// ```
    #[inline]
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

impl<I> From<I> for Scope<I>
where
    I: Id,
{
    /// Creates a scope from an identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Scope;
    ///
    /// // Create scope
    /// let scope = Scope::from(42);
    /// ```
    #[inline]
    fn from(id: I) -> Self {
        let path = Arc::from([id]);
        Self { hash: hash(&path), path }
    }
}

// ----------------------------------------------------------------------------

impl<I> FromIterator<I> for Scope<I>
where
    I: Id,
{
    /// Creates a scope from an iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::Scope;
    ///
    /// // Create scope from iterator
    /// let scope = Scope::from_iter([1, 2, 3]);
    /// ```
    #[inline]
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        let path = iter.into_iter().collect();
        Self { hash: hash(&path), path }
    }
}

// ----------------------------------------------------------------------------

impl<I> Index<usize> for Scope<I> {
    type Output = I;

    /// Indexes the scope by position.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

// ----------------------------------------------------------------------------

impl<I> Hash for Scope<I> {
    /// Hashes the scope.
    ///
    /// Since scope paths are immutable, we can use a precomputed hash for fast
    /// hashing. This is especially useful when scope paths are used as keys in
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

impl<I> Display for Scope<I>
where
    I: Display,
{
    /// Formats the scope for display.
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

impl<I> Debug for Scope<I>
where
    I: Debug,
{
    // Formats the scope for debugging.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entry(&self.path).finish()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Precomputes the hash for the given path - this is used for fast hashing of
/// scopes, since scope paths are immutable, meaning hashes can be precomputed.
#[inline]
fn hash<P>(path: &P) -> u64
where
    P: Hash,
{
    let mut hasher = AHasher::default();
    path.hash(&mut hasher);
    hasher.finish()
}
