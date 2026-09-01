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

//! Action-visible control-plane events and revision progress.

use std::marker::PhantomData;
use std::time::Instant;

use crate::scheduler::Id;

use super::{Emitter, Output, Result, WakeKey};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// One control-plane event serialized through an action's ordinary job slot.
pub enum Event {
    /// A keyed temporal wake became due.
    Wake {
        /// Action-local semantic identity that became due.
        key: WakeKey,
        /// Deadline whose lower bound has elapsed.
        deadline: Instant,
    },
    /// Shared key-free revision progress reached this action.
    Progress(ProgressEvent),
}

/// Key-free revision progress delivered at a subscribed graph position.
///
/// Delivery is ordered but not balanced. If abort prunes an undispatched
/// `Begin`, a subscriber can receive `Abort` as the revision's only event.
/// Subscribers must therefore treat `Abort` as independently terminal.
/// Progress ordering does not impose a global order against data arriving on
/// independently scheduled converging branches: branch data can reach a
/// subscriber before its shared `Begin`. Subscribers should create revision
/// state lazily, while `End` still proves all preceding relevant work drained.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProgressEvent {
    /// A new source revision has opened.
    Begin,
    /// The revision branch ended and all preceding relevant work drained.
    End,
    /// The revision was aborted, possibly before `Begin` reached this action.
    Abort,
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Scoped one-pass driver over the event supplied to this invocation.
pub struct Events<I>
where
    I: Id,
{
    event: Option<Event>,
    marker: PhantomData<fn(I)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl ProgressEvent {
    pub(crate) const fn is_end(&self) -> bool {
        matches!(self, Self::End)
    }

    pub(crate) const fn is_abort(&self) -> bool {
        matches!(self, Self::Abort)
    }
}

// ----------------------------------------------------------------------------

impl<I> Events<I>
where
    I: Id,
{
    pub(super) const fn empty() -> Self {
        Self {
            event: None,
            marker: PhantomData,
        }
    }

    pub(super) const fn one(event: Event) -> Self {
        Self {
            event: Some(event),
            marker: PhantomData,
        }
    }

    /// Drains the supplied event.
    pub fn for_each<V>(
        mut self, output: &mut Output<I, V>,
        mut callback: impl FnMut(Event, &mut Emitter<'_, I, V>) -> Result,
    ) {
        let Some(event) = self.event.take() else {
            return;
        };
        if let Err(error) = callback(event, &mut output.emitter()) {
            output.outcomes.report(error);
        }
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> Drop for Events<I>
where
    I: Id,
{
    fn drop(&mut self) {
        assert!(
            self.event.is_none(),
            "action returned with an unread control-plane event"
        );
    }
}
