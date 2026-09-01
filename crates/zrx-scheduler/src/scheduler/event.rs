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

//! Revision-framed source changes owned by scheduler admission.

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// One state mutation addressed by a complete opaque identity.
#[derive(Debug)]
pub enum Change<I, T> {
    /// Sets the current value at an identity.
    Insert(I, T),
    /// Clears the current value at an identity.
    Remove(I),
}

/// Semantic event kind within a source revision.
#[derive(Debug)]
pub(crate) enum Kind<C> {
    /// Begins the revision.
    Begin,
    /// Carries one owned collection or batch of changes.
    Changes(C),
    /// Seals the revision after its preceding changes.
    End,
    /// Aborts the revision.
    Abort,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One revision-framed source event carrying an owned changes payload.
#[derive(Debug)]
pub(crate) struct Event<C> {
    revision: Revision,
    kind: Kind<C>,
}

// ----------------------------------------------------------------------------

/// Opaque revision identity scoped to one external source session.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Revision(u64);

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I, T> Change<I, T> {
    /// Returns the identity addressed by the change.
    #[inline]
    #[must_use]
    pub const fn identity(&self) -> &I {
        match self {
            Self::Insert(identity, _) | Self::Remove(identity) => identity,
        }
    }
}

// ----------------------------------------------------------------------------

impl<C> Event<C> {
    /// Creates a revision-framed event.
    #[inline]
    #[must_use]
    pub(crate) const fn new(revision: Revision, kind: Kind<C>) -> Self {
        Self { revision, kind }
    }

    /// Splits the event into its revision and semantic kind.
    #[inline]
    #[must_use]
    pub(crate) fn into_parts(self) -> (Revision, Kind<C>) {
        (self.revision, self.kind)
    }
}

// ----------------------------------------------------------------------------

impl Revision {
    /// Creates an external source revision identity.
    #[inline]
    #[must_use]
    pub(crate) const fn new(value: u64) -> Self {
        Self(value)
    }
}
