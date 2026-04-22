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

//! Source.

use crossbeam::channel::{Receiver, TryRecvError};
use std::iter;

use crate::scheduler::action::context::Binding;
use crate::scheduler::action::{Action, Context};
use crate::scheduler::session::Error;
use crate::scheduler::signal::{Diff, Id, Value};
use crate::scheduler::step::{IntoSteps, Scope};

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> Action<I> for Receiver<Diff<I, T>>
where
    I: Id,
    T: Value,
{
    type Inputs = ();
    type Output<'a> = T;

    /// Receives from the channel and emits a step for each received diff.
    fn execute(&mut self, ctx: Context<I, Self>) -> impl IntoSteps<I, Self> {
        let Binding { mut output, .. } = ctx.bind();
        iter::from_fn(move || match self.try_recv() {
            Ok(diff) => Some(
                Scope::from(match diff {
                    Diff::Insert(key, value) => {
                        output.insert(key.clone(), value);
                        key
                    }
                    Diff::Remove(key) => {
                        output.remove(&key);
                        key
                    }
                })
                .done(),
            ),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                Some(Err(Into::into(Error::Disconnected)))
            }
        })
    }
}
