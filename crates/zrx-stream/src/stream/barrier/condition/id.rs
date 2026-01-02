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

//! Condition.

use zrx_id::{Id, Matcher};

use super::ConditionFn;

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl ConditionFn<Id> for Matcher {
    /// Returns whether the given identifier satisfies the condition.
    ///
    /// This allows to create a [`Condition`][] from a [`Matcher`], making it
    /// convenient to use a set of [`Selector`][] instances for matching.
    ///
    /// [`Condition`]: crate::stream::barrier::Condition
    /// [`Selector`]: zrx_id::Selector
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{Id, Matcher};
    /// use zrx_stream::barrier::Condition;
    ///
    /// // Create matcher builder and add selector
    /// let mut builder = Matcher::builder();
    /// builder.add("zrs:::::**/*.md:")?;
    ///
    /// // Create condition from matcher
    /// let condition = Condition::new(builder.build()?);
    ///
    /// // Create identifier and test condition
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// assert!(condition.satisfies(&id));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn satisfies(&self, id: &Id) -> bool {
        // We can safely use expect here, since we're certain we pass a valid
        // identifier to the matcher, which means it can never fail
        self.is_match(id).expect("invariant")
    }
}
