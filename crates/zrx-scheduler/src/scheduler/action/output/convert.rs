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

//! Output conversions.

use zrx_diagnostic::report::Report;

use crate::scheduler::action::Result;

use super::Outputs;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Conversion into [`Outputs`].
pub trait IntoOutputs<I> {
    /// Converts into an output collection.
    ///
    /// Note that this returns a [`Result`] containing a [`Report`] with the
    /// output collection, which can annotate data with additional diagnostics.
    ///
    /// # Errors
    ///
    /// Note that this method never returns an error by itself, as this trait is
    /// primarily provided for wrapping values returned from infallible actions.
    /// Consequentially, this trait isn't named `TryIntoOutputs`, as it's not
    /// falling under Rust's try-semantics.
    fn into_outputs(self) -> Result<Outputs<I>>;
}

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<I, O> IntoOutputs<I> for O
where
    O: Into<Outputs<I>>,
{
    /// Creates an output collection from an output and wraps it in a result.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::action::output::IntoOutputs;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from output
    /// let report = item.into_outputs()?;
    /// for output in report.data {
    ///     println!("{output:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_outputs(self) -> Result<Outputs<I>> {
        Ok(Report::new(self.into()))
    }
}

impl<I, O> IntoOutputs<I> for Report<O>
where
    O: Into<Outputs<I>>,
{
    /// Creates an output collection from an output in a report.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::action::output::IntoOutputs;
    /// use zrx_scheduler::action::Report;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from report
    /// let report = Report::new(item).into_outputs()?;
    /// for output in report.data {
    ///     println!("{output:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_outputs(self) -> Result<Outputs<I>> {
        Ok(self.map(Into::into))
    }
}

impl<I, O> IntoOutputs<I> for Result<O>
where
    O: Into<Outputs<I>>,
{
    /// Creates an output collection from an output in a result.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// use zrx_scheduler::action::output::IntoOutputs;
    /// use zrx_scheduler::action::Report;
    /// use zrx_scheduler::effect::Item;
    ///
    /// // Create item
    /// let item = Item::new("id", Some(42));
    ///
    /// // Create output collection from result
    /// let report = Ok(Report::new(item)).into_outputs()?;
    /// for output in report.data {
    ///     println!("{output:?}");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    fn into_outputs(self) -> Result<Outputs<I>> {
        self.map(|report| report.map(Into::into))
    }
}
