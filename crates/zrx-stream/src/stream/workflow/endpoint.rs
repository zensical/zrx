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

//! Typed workflow boundary descriptors.

use std::fmt;
use std::marker::PhantomData;
use std::slice;

use thiserror::Error;
use zrx_scheduler::Value;
use zrx_scheduler::action::Port;
use zrx_scheduler::plan::{InputId, OutputId};

use crate::stream::Id;
use crate::stream::Key;

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Direction of one external workflow endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Values enter the workflow.
    Input,
    /// Values leave the workflow.
    Output,
}

// ----------------------------------------------------------------------------

/// Invalid unique typed endpoint lookup.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum LookupError {
    /// No endpoint carries the requested type.
    #[error("workflow has no {direction} carrying {value}")]
    Missing {
        direction: Direction,
        value: &'static str,
    },
    /// More than one endpoint carries the requested type.
    #[error("workflow has multiple {direction}s carrying {value}")]
    Ambiguous {
        direction: Direction,
        value: &'static str,
    },
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// One erased workflow-local external input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Input {
    id: InputId,
    port: Port,
}

// ----------------------------------------------------------------------------

/// One erased workflow-local external output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Output {
    id: OutputId,
    port: Port,
}

// ----------------------------------------------------------------------------

/// Borrowing iterator over a workflow's erased input catalogue.
pub struct Inputs<'a, I>
where
    I: Id,
{
    inner: slice::Iter<'a, Input>,
    marker: PhantomData<fn() -> I>,
}

// ----------------------------------------------------------------------------

/// Borrowing iterator over a workflow's erased output catalogue.
pub struct Outputs<'a, I>
where
    I: Id,
{
    inner: slice::Iter<'a, Output>,
    marker: PhantomData<fn() -> I>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Input {
    pub(in crate::stream::workflow) const fn new(
        id: InputId, port: Port,
    ) -> Self {
        Self { id, port }
    }

    /// Returns the workflow-local input identity.
    #[must_use]
    pub const fn id(self) -> InputId {
        self.id
    }

    /// Returns the exact in-process keyed value descriptor.
    #[must_use]
    pub const fn port(self) -> Port {
        self.port
    }
}

// ----------------------------------------------------------------------------

impl Output {
    pub(in crate::stream::workflow) const fn new(
        id: OutputId, port: Port,
    ) -> Self {
        Self { id, port }
    }

    /// Returns the workflow-local output identity.
    #[must_use]
    pub const fn id(self) -> OutputId {
        self.id
    }

    /// Returns the exact in-process keyed value descriptor.
    #[must_use]
    pub const fn port(self) -> Port {
        self.port
    }
}

// ----------------------------------------------------------------------------

impl<'a, I> Inputs<'a, I>
where
    I: Id,
{
    pub(in crate::stream::workflow) fn new(inputs: &'a [Input]) -> Self {
        Self {
            inner: inputs.iter(),
            marker: PhantomData,
        }
    }

    /// Retains only inputs carrying `T`.
    pub fn of_type<T>(self) -> impl Iterator<Item = &'a Input> + 'a
    where
        T: Value,
    {
        let port = Port::of::<Key<I>, T>();
        self.inner.filter(move |input| input.port == port)
    }
}

// ----------------------------------------------------------------------------

impl<'a, I> Outputs<'a, I>
where
    I: Id,
{
    pub(in crate::stream::workflow) fn new(outputs: &'a [Output]) -> Self {
        Self {
            inner: outputs.iter(),
            marker: PhantomData,
        }
    }

    /// Retains only outputs carrying `T`.
    pub fn of_type<T>(self) -> impl Iterator<Item = &'a Output> + 'a
    where
        T: Value,
    {
        let port = Port::of::<Key<I>, T>();
        self.inner.filter(move |output| output.port == port)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl fmt::Display for Direction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Input => formatter.write_str("input"),
            Self::Output => formatter.write_str("output"),
        }
    }
}

// ----------------------------------------------------------------------------

impl<'a, I> Iterator for Inputs<'a, I>
where
    I: Id,
{
    type Item = &'a Input;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I> ExactSizeIterator for Inputs<'_, I> where I: Id {}

// ----------------------------------------------------------------------------

impl<'a, I> Iterator for Outputs<'a, I>
where
    I: Id,
{
    type Item = &'a Output;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I> ExactSizeIterator for Outputs<'_, I> where I: Id {}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

pub(in crate::stream::workflow) fn unique<T>(
    values: impl Iterator<Item = T>, direction: Direction, value: &'static str,
) -> Result<T, LookupError> {
    let mut values = values;
    let Some(found) = values.next() else {
        return Err(LookupError::Missing { direction, value });
    };
    if values.next().is_some() {
        return Err(LookupError::Ambiguous { direction, value });
    }
    Ok(found)
}
