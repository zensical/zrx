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

//! Session collection.

use ahash::HashMap;
use std::any::Any;

use crate::scheduler::graph::{Descriptor, Source};

use super::error::{Error, Result};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Session collection.
#[derive(Debug)]
pub struct Sessions {
    /// Source nodes.
    sources: Vec<Source>,
    /// Active sessions.
    items: HashMap<usize, Vec<usize>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Sessions {
    /// Creates a session collection.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::session::Sessions;
    ///
    /// // Create session collection
    /// let sessions = Sessions::new([]);
    /// ```
    #[must_use]
    pub fn new<S>(sources: S) -> Self
    where
        S: IntoIterator<Item = Source>,
    {
        Self {
            sources: sources.into_iter().collect(),
            items: HashMap::default(),
        }
    }

    /// Inserts the session identifier with the given type.
    ///
    /// Before creating a session, we must check whether there are source nodes
    /// that match the given type. If there aren't, emissions would be swallowed
    /// without any effect, so we need to ensure that at least one source is
    /// registered for the session to be valid.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Type`] if no source matches the given type.
    pub fn insert<T>(&mut self, id: usize) -> Result
    where
        T: Any,
    {
        let descriptor = Descriptor::new::<T>();

        // We might have multiple sources with the same descriptors, since the
        // scheduler allows for an arbitrary number of source nodes, so we need
        // to traverse all sources and collect the actions from the descriptors
        // that match the given type
        let iter = self.sources.iter();
        let actions = iter
            .filter(|source| source.descriptor == descriptor)
            .flat_map(|source| source.actions.iter().copied())
            .collect::<Vec<_>>();

        // Only insert session if there's at least one source that matches the
        // given type, or otherwise return a error to signal creation failed
        if actions.is_empty() {
            Err(Error::Type)
        } else {
            self.items.insert(id, actions);
            Ok(())
        }
    }

    /// Returns the actions for the given session identifier.
    #[inline]
    pub fn get(&self, id: usize) -> impl Iterator<Item = usize> {
        self.items.get(&id).into_iter().flatten().copied()
    }

    /// Removes the session with the given identifier.
    ///
    /// This method does not actively terminate the session, as only the owner
    /// is able to terminate the session by dropping it.
    #[inline]
    pub fn remove(&mut self, id: usize) {
        self.items.remove(&id);
    }
}
