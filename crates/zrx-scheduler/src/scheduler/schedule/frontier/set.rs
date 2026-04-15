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

//! Frontier set.

use ahash::HashMap;
use slab::Slab;
use std::ops::{Index, IndexMut};

use zrx_graph::traversal::{Error, Result};

use crate::scheduler::signal::{Id, Scope};

use super::Frontier;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Frontier set.
#[derive(Debug)]
pub struct Frontiers<I> {
    /// Underlying store.
    store: Slab<Frontier<I>>,
    /// Index for frontier lookup.
    index: HashMap<Scope<I>, Vec<usize>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Frontiers<I>
where
    I: Id,
{
    /// Inserts a frontier into the set.
    pub fn insert(&mut self, frontier: Frontier<I>) -> Result<usize> {
        let Frontier { scope, mut traversal, .. } = frontier;

        // Try to converge with each existing frontier
        if let Some(ids) = self.index.get(&scope) {
            for &id in ids {
                match self.store[id].traversal.converge(traversal) {
                    Ok(()) => return Ok(id),
                    Err(Error::Disjoint(existing)) => traversal = existing,
                    Err(err) => return Err(err),
                }
            }
        }

        // No convergence with any existing frontier, insert new one
        let id = self.store.insert(Frontier::new(scope.clone(), traversal));

        // Update index and return frontier identifier
        self.index.entry(scope).or_default().push(id);
        Ok(id)
    }

    /// Removes a frontier from the set.
    pub fn remove(&mut self, id: usize) -> Option<Frontier<I>> {
        let frontier = self.store.try_remove(id)?;
        let scope = frontier.scope();
        if let Some(ids) = self.index.get_mut(scope) {
            ids.retain(|&x| x != id);
            if ids.is_empty() {
                self.index.remove(scope);
            }
        }
        Some(frontier)
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Frontiers<I> {
    /// Returns the number of frontiers.
    #[inline]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Returns whether there are any frontiers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Index<usize> for Frontiers<I> {
    type Output = Frontier<I>;

    /// Returns a reference to the frontier at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.store[index]
    }
}

impl<I> IndexMut<usize> for Frontiers<I> {
    /// Returns a mutable reference to the frontier at the index.
    ///
    /// # Panics
    ///
    /// Panics if the index is out of bounds.
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.store[index]
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Frontiers<I> {
    /// Creates a frontier set.
    #[inline]
    fn default() -> Self {
        Self {
            store: Slab::default(),
            index: HashMap::default(),
        }
    }
}
