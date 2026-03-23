// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the `Software`), to
// deal in the Software without restriction, including without limitation the
// rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
// sell copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:

// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED `AS IS`, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NON-INFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
// IN THE SOFTWARE.

// ----------------------------------------------------------------------------

//! Specificity.

use std::cmp::{self, Ordering};

pub mod convert;
pub mod segment;
mod tokens;

use convert::ToSpecificity;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Specificity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Specificity(u16, u16, u16, u16);

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Specificity {
    /// Computes the sum of both specificities.
    #[inline]
    fn sum(self, other: Self) -> Self {
        let Specificity(a1, b1, c1, l1) = self;
        let Specificity(a2, b2, c2, l2) = other;
        Self(a1 + a2, b1 + b2, c1 + c2, l1 + l2)
    }

    /// Computes the minimum of both specificities.
    #[inline]
    fn min(self, other: Self) -> Self {
        cmp::min(self, other)
    }

    /// Computes the minimum of both specificities while summing their lengths.
    #[inline]
    fn min_sum_len(self, other: Self) -> Self {
        let mut spec = cmp::min(self, other);
        spec.3 = self.3.saturating_add(other.3);
        spec
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> From<T> for Specificity
where
    T: ToSpecificity,
{
    /// Creates a specificity from a value.
    #[inline]
    fn from(value: T) -> Self {
        value.to_specificity()
    }
}

// ----------------------------------------------------------------------------

impl PartialOrd for Specificity {
    /// Orders two specificities.
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Specificity {
    /// Orders two specificities.
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        let Specificity(a1, b1, c1, l1) = self;
        let Specificity(a2, b2, c2, l2) = other;
        a1.cmp(a2)
            .then(b1.cmp(b2))
            .then(c2.cmp(c1)) // reversed: fewer ** = more specific
            .then(l1.cmp(l2))
    }
}
