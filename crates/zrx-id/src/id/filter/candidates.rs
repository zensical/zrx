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

//! Iterator over candidates.

use slab::Slab;
use std::iter::Peekable;

use crate::id::matcher::matches::IntoIter;
use crate::id::matcher::Matches;
use crate::id::TryToId;

use super::condition::Condition;
use super::error::Result;
use super::Filter;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Iterator over candidates.
pub struct Candidates<'a> {
    /// Iterator over matches.
    matches: Peekable<IntoIter>,
    /// Condition set, built from expressions.
    conditions: &'a Slab<Condition>,
    /// Condition indices of negations.
    negations: &'a [u32],
    /// Condition term mappings.
    mapping: &'a [u32],
    /// Match set used during iteration.
    workset: Matches,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Filter {
    /// Returns the indices of expressions that match the identifier.
    ///
    /// This method compares expressions part of the filter against the given
    /// identifier, and returns an iterator over the indices of the expressions
    /// that match. Note that the order of the returned indices corresponds to
    /// the order in which the expressions were added to the filter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Matcher`][] if the identifier is invalid.
    ///
    /// [`Error::Matcher`]: crate::id::filter::Error::Matcher
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_id::{selector, Expression, Filter, Id};
    ///
    /// // Create filter builder and insert expression
    /// let mut builder = Filter::builder();
    /// builder.insert(Expression::all(|expr| {
    ///     expr.with(selector!(location = "**/*.md")?)?
    ///         .with(selector!(provider = "file")?)
    /// })?);
    ///
    /// // Create filter from builder
    /// let filter = builder.build()?;
    ///
    /// // Create identifier and obtain candidate expressions
    /// let id: Id = "zri:file:::docs:index.md:".parse()?;
    /// for index in filter.candidates(&id)? {
    ///     println!("{index:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn candidates<T>(&self, id: &T) -> Result<Candidates<'_>>
    where
        T: TryToId,
    {
        let matches = self.matcher.matches(id)?;
        Ok(Candidates {
            matches: matches.into_iter().peekable(),
            conditions: &self.conditions,
            negations: &self.negations,
            mapping: &self.mapping,
            workset: Matches::new(),
        })
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Iterator for Candidates<'_> {
    type Item = usize;

    /// Returns the next candidate.
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.workset.clear();

            // Retrieve the next match without consuming it, as we must first
            // check if there're any conditions with negations that we need to
            // process first, or whether the current match lies exactly within
            // one of those negations
            let opt = self.matches.peek().copied();

            // Retrieve the index of the current condition for processing - if
            // there's a match within the match set, use that to check if we
            // should process the condition the match is a part of, or the
            // next condition with a negation first
            let check = if let Some(start) = opt {
                let index = self.mapping[start];

                // Either chose the current condition, or the condition that
                // needs to be checked despite of any matches being present
                let opt = self.negations.first().copied();
                opt.filter(|&first| first <= index).map_or(index, |first| {
                    self.negations = &self.negations[1..];
                    first
                })

            // No more matches - in this case we need to process all remaining
            // conditions that contain negations
            } else if let Some(&first) = self.negations.first() {
                self.negations = &self.negations[1..];
                first

            // No more conditions to check
            } else {
                return None;
            };

            // If there're matches, consume all matches that belong to the
            // condition, and insert them into the working set of matches
            if let Some(mut start) = opt {
                // Do a backwards scan on the terms to find the index of the
                // first term for the condition, to correctly assign matches
                while start > 0 && self.mapping[start - 1] == check {
                    start -= 1;
                }

                // Next, consume all matches for the current condition, and
                // add them to the working set of matches
                while let Some(index) =
                    self.matches.next_if(|&index| self.mapping[index] == check)
                {
                    self.workset.add(index - start);
                }
            }

            // After consuming all matches for this condition, check whether
            // it is satisfied - if not, continue with the next condition
            let index = check as usize;
            if self.conditions[index].satisfies(&self.workset) {
                return Some(index);
            }
        }
    }
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod matches {
        use crate::id::expression::Expression;
        use crate::id::filter::{Filter, Result};
        use crate::selector;

        #[test]
        fn handles_any() -> Result {
            let mut builder = Filter::builder();
            let _ = builder.insert(Expression::any(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?);
            let filter = builder.build()?;
            for (id, check) in [
                ("zri:file:::docs:image.jpg:", vec![0]),
                ("zri:file:::docs:image.png:", vec![0]),
                ("zri:file:::docs:image.gif:", vec![]),
            ] {
                assert_eq!(
                    filter.candidates(&id)?.collect::<Vec<_>>(), // fmt
                    check
                );
            }
            Ok(())
        }

        #[test]
        fn handles_all() -> Result {
            let mut builder = Filter::builder();
            let _ = builder.insert(Expression::all(|expr| {
                expr.with(selector!(location = "**/*.md")?)?
                    .with(selector!(provider = "file")?)
            })?);
            let filter = builder.build()?;
            for (id, check) in [
                ("zri:file:::docs:index.md:", vec![0]),
                ("zri:file:::docs:image.png:", vec![]),
                ("zri:git:::docs:image.md:", vec![]),
            ] {
                assert_eq!(
                    filter.candidates(&id)?.collect::<Vec<_>>(), // fmt
                    check
                );
            }
            Ok(())
        }

        #[test]
        fn handles_not() -> Result {
            let mut builder = Filter::builder();
            let _ = builder.insert(Expression::not(|expr| {
                expr.with(selector!(location = "**/*.jpg")?)?
                    .with(selector!(location = "**/*.png")?)
            })?);
            let filter = builder.build()?;
            for (id, check) in [
                ("zri:file:::docs:index.md:", vec![0]),
                ("zri:file:::docs:image.jpg:", vec![]),
                ("zri:file:::docs:image.png:", vec![]),
            ] {
                assert_eq!(
                    filter.candidates(&id)?.collect::<Vec<_>>(), // fmt
                    check
                );
            }
            Ok(())
        }

        #[test]
        fn handles_all_any() -> Result {
            let mut builder = Filter::builder();
            let _ = builder.insert(Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(location = "**/*.jpg")?)?
                            .with(selector!(location = "**/*.png")?)
                    }))
            })?);
            let filter = builder.build()?;
            for (id, check) in [
                ("zri:file:::docs:index.md:", vec![]),
                ("zri:file:::docs:image.jpg:", vec![0]),
                ("zri:file:::docs:image.png:", vec![0]),
                ("zri:file:::docs:image.gif:", vec![]),
                ("zri:git:::docs:image.jpg:", vec![]),
                ("zri:git:::docs:image.png:", vec![]),
            ] {
                assert_eq!(
                    filter.candidates(&id)?.collect::<Vec<_>>(), // fmt
                    check
                );
            }
            Ok(())
        }

        #[test]
        fn handles_all_any_not() -> Result {
            let mut builder = Filter::builder();
            let _ = builder.insert(Expression::all(|expr| {
                expr.with(selector!(provider = "file")?)?
                    .with(Expression::any(|expr| {
                        expr.with(selector!(context = "docs")?)? // fmt
                            .with(Expression::not(|expr| {
                                expr.with(selector!(location = "**/*.jpg")?)?
                                    .with(selector!(location = "**/*.png")?)
                            }),
                        )
                    }))
            })?);
            let filter = builder.build()?;
            for (id, check) in [
                ("zri:file:::docs:index.md:", vec![0]),
                ("zri:file:::docs:image.jpg:", vec![0]),
                ("zri:file:::docs:image.png:", vec![0]),
                ("zri:file:::docs:image.gif:", vec![0]),
                ("zri:git:::docs:image.jpg:", vec![]),
                ("zri:git:::docs:image.png:", vec![]),
            ] {
                assert_eq!(
                    filter.candidates(&id)?.collect::<Vec<_>>(), // fmt
                    check
                );
            }
            Ok(())
        }
    }
}
