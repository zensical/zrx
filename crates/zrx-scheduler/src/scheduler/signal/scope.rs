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

use super::Id;

mod error;
mod path;

pub use error::{Error, Result};
use path::Path;

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
    path: Path<I>,
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
        let mut path = match &self.path {
            Path::Value(head) => Vec::from([head.clone()]),
            Path::Chain(head) => head.to_vec(),
        };

        // Extend current path with other path
        match &other.path {
            Path::Value(tail) => path.push(tail.clone()),
            Path::Chain(tail) => path.extend_from_slice(tail),
        }

        // Create scope with concatenated path
        let path = Path::Chain(path.into());
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
        self.with_path(|path| path.reverse())
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
        self.with_path(|path| path.rotate_left(n))
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
        self.with_path(|path| path.rotate_right(n))
    }

    /// Applies a transformation to the scope, if it's a chain of values.
    fn with_path<F>(&self, f: F) -> Self
    where
        F: FnOnce(&mut Vec<I>),
    {
        match &self.path {
            Path::Value(_) => self.clone(),
            Path::Chain(path) => {
                let mut path = path.to_vec();
                f(&mut path);

                //
                let path = Path::Chain(path.into());
                Self { hash: hash(&path), path }
            }
        }
    }

    /// Return identifier if scope is a single value.
    #[deprecated(note = "temporary")]
    pub fn as_id(&self) -> Option<&I> {
        match &self.path {
            Path::Value(id) => Some(id),
            Path::Chain(path) => path.first(),
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
        let path = id.into();
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
        match &self.path {
            Path::Value(path) => path,
            Path::Chain(path) => &path[index],
        }
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
        match &self.path {
            Path::Value(path) => Display::fmt(path, f),
            Path::Chain(path) => {
                for (i, item) in path.iter().enumerate() {
                    Display::fmt(item, f)?;

                    // Write slash if not last
                    if i < path.len() - 1 {
                        f.write_str(" / ")?;
                    }
                }

                // No errors occurred
                Ok(())
            }
        }
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
fn hash<I>(path: &Path<I>) -> u64
where
    I: Hash,
{
    let mut hasher = AHasher::default();
    path.hash(&mut hasher);
    hasher.finish()
}
