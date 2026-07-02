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

//! Condition builder.

use crate::id::expression::Expression;
use crate::id::sequence::{Element, Sequence};

use super::segment::Segment;
use super::Condition;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Condition builder.
#[derive(Debug)]
pub struct Builder {
    /// Condition segments.
    segments: Vec<Segment>,
    /// Extracted expressions.
    expressions: Vec<Expression>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Condition {
    /// Creates a condition builder from a sequence.
    #[inline]
    #[must_use]
    pub fn builder<T>(sequence: T) -> Builder
    where
        T: Into<Sequence>,
    {
        let mut expressions = Vec::new();
        Builder {
            segments: extract(sequence.into(), &mut expressions),
            expressions,
        }
    }
}

// ----------------------------------------------------------------------------

impl Builder {
    /// Builds the condition.
    #[inline]
    #[must_use]
    pub fn build(self) -> Condition {
        Condition {
            segments: self.segments.into_boxed_slice(),
            expressions: self.expressions.into_boxed_slice(),
        }
    }

    /// Optimizes the condition builder.
    ///
    /// It's important to optimize the condition before building it, as this
    /// will collapse adjacent gaps and reduce the number of segments.
    #[inline]
    #[must_use]
    pub fn optimize(self) -> Self {
        Self {
            segments: optimize(self.segments),
            expressions: self.expressions,
        }
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Extracts segments and expressions from a sequence.
fn extract(
    sequence: Sequence, expressions: &mut Vec<Expression>,
) -> Vec<Segment> {
    let iter = sequence.into_iter();
    iter.map(|element| match element {
        Element::Gap => Segment::Gap,
        Element::Expression(expr) => {
            let index = expressions.len();
            expressions.push(expr);
            Segment::Expression(index)
        }
    })
    .collect()
}

// ----------------------------------------------------------------------------

/// Optimizes a set of segments by collapsing adjacent gaps.
fn optimize(mut segments: Vec<Segment>) -> Vec<Segment> {
    segments.dedup_by(|left, right| {
        matches!(left, Segment::Gap) && matches!(right, Segment::Gap)
    });
    segments
}
