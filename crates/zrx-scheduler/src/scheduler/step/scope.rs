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

use std::mem;

use zrx_diagnostic::Diagnostic;

use crate::scheduler::signal::{Id, Key};
use crate::scheduler::step::{Result, Step, Steps};

use super::effect::Effect;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scope.
#[derive(Debug)]
pub struct Scope<I> {
    /// Key.
    key: Key<I>,
    /// Frontier identifier.
    id: Option<usize>,
    /// Diagnostics.
    diagnostics: Vec<Diagnostic>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Scope<I>
where
    I: Id,
{
    /// Creates a scope from the given key.
    #[must_use]
    pub fn new(key: Key<I>, id: usize) -> Self {
        Self {
            key,
            id: Some(id),
            diagnostics: Vec::new(),
        }
    }

    /// Creates a new scope, moving diagnostics into it.
    pub(crate) fn take(&mut self) -> Self {
        Self {
            key: self.key.clone(),
            id: self.id,
            diagnostics: mem::take(&mut self.diagnostics),
        }
    }

    /// Adds a diagnostic to the scope.
    pub fn add_diagnostic<D>(&mut self, diagnostic: D)
    where
        D: Into<Diagnostic>,
    {
        self.diagnostics.push(diagnostic.into());
    }

    /// Marks the step as done.
    #[allow(clippy::missing_errors_doc)]
    #[inline]
    pub fn done<C>(&mut self) -> Result<Steps<I, C>> {
        Ok(Steps::from(Step::new(self.take(), Effect::Done)))
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Scope<I> {
    /// Returns the key of the scope.
    #[inline]
    pub fn key(&self) -> &Key<I> {
        &self.key
    }

    /// Returns the frontier identifier of the scope.
    #[inline]
    pub fn id(&self) -> Option<usize> {
        self.id
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> From<Key<I>> for Scope<I> {
    /// Creates a scope from the given key.
    #[inline]
    fn from(scope: Key<I>) -> Self {
        Self {
            key: scope,
            id: None,
            diagnostics: Vec::new(),
        }
    }
}
