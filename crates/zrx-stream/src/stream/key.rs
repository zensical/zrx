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

//! Hierarchical stream keys.

use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::slice::Iter;
use std::sync::Arc;

use ahash::AHasher;
use zrx_scheduler::Value;

mod error;
mod id;

pub use error::{Error, Result};
pub use id::Id;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Immutable hierarchical identity of one stream item.
#[derive(Clone, PartialOrd, Ord)]
pub struct Key<I> {
    path: Arc<[I]>,
    hash: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Key<I> {
    /// Returns the identifier if this key contains exactly one component.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`] for an empty key and [`Error::Depth`] for a
    /// key containing more than one component.
    #[inline]
    pub fn try_as_id(&self) -> Result<&I> {
        match &*self.path {
            [id] => Ok(id),
            [] => Err(Error::Empty),
            _ => Err(Error::Depth),
        }
    }

    /// Creates an iterator over the key components.
    #[inline]
    pub fn iter(&self) -> Iter<'_, I> {
        self.path.iter()
    }
}

impl<I> Key<I>
where
    I: Id,
{
    /// Concatenates this key with another key.
    #[must_use]
    pub fn concat<K>(&self, tail: K) -> Self
    where
        K: AsRef<Self>,
    {
        let tail = tail.as_ref();
        self.path.iter().chain(tail.path.iter()).cloned().collect()
    }

    /// Reverses the key components.
    #[must_use]
    pub fn reverse(&self) -> Self {
        self.path.iter().rev().cloned().collect()
    }

    /// Rotates the key components left by `count` positions.
    ///
    /// # Panics
    ///
    /// Panics if `count` exceeds the key length.
    #[must_use]
    pub fn rotate_left(&self, count: usize) -> Self {
        self.path[count..]
            .iter()
            .chain(self.path[..count].iter())
            .cloned()
            .collect()
    }

    /// Rotates the key components right by `count` positions.
    ///
    /// # Panics
    ///
    /// Panics if `count` exceeds the key length.
    #[inline]
    #[must_use]
    pub fn rotate_right(&self, count: usize) -> Self {
        self.rotate_left(self.path.len() - count)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> PartialEq for Key<I>
where
    I: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.path, &other.path) || self.path == other.path
    }
}

impl<I> Eq for Key<I> where I: Eq {}

impl<I> Value for Key<I> where I: Id {}

impl<I> AsRef<Self> for Key<I> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<I> From<I> for Key<I>
where
    I: Id,
{
    #[inline]
    fn from(id: I) -> Self {
        [id].into_iter().collect()
    }
}

impl<I> FromIterator<I> for Key<I>
where
    I: Id,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = I>,
    {
        let path = iter.into_iter().collect();
        Self { hash: hash(&path), path }
    }
}

impl<'a, I> IntoIterator for &'a Key<I> {
    type Item = &'a I;
    type IntoIter = Iter<'a, I>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<I> Index<usize> for Key<I> {
    type Output = I;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

impl<I> Hash for Key<I> {
    #[inline]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write_u64(self.hash);
    }
}

impl<I> Display for Key<I>
where
    I: Display,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, item) in self.path.iter().enumerate() {
            Display::fmt(item, formatter)?;
            if index + 1 < self.path.len() {
                formatter.write_str(" / ")?;
            }
        }
        Ok(())
    }
}

impl<I> Debug for Key<I>
where
    I: Debug,
{
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.path.iter()).finish()
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[inline]
fn hash<P>(path: &P) -> u64
where
    P: Hash,
{
    let mut hasher = AHasher::default();
    path.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::{Error, Key};

    #[test]
    fn transforms_hierarchical_keys() {
        let key = Key::from_iter([1, 2, 3]);

        assert_eq!(key.reverse(), Key::from_iter([3, 2, 1]));
        assert_eq!(key.rotate_left(1), Key::from_iter([2, 3, 1]));
        assert_eq!(key.rotate_right(1), Key::from_iter([3, 1, 2]));
        assert_eq!(key.concat(Key::from(4)), Key::from_iter([1, 2, 3, 4]));
    }

    #[test]
    fn projects_only_scalar_keys() {
        assert!(matches!(Key::from(1).try_as_id(), Ok(&1)));
        assert!(matches!(
            Key::<u64>::from_iter([]).try_as_id(),
            Err(Error::Empty)
        ));
        assert!(matches!(
            Key::from_iter([1, 2]).try_as_id(),
            Err(Error::Depth)
        ));
    }
}
