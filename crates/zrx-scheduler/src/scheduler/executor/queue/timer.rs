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

//! Timer queue.

use crossbeam::channel::{at, never, Receiver};
use std::borrow::Cow;
use std::time::Instant;

use zrx_store::queue::Queue;
use zrx_store::{Store, StoreMut, StoreMutRef};

use crate::scheduler::action::Outputs;
use crate::scheduler::effect::Timer;

use super::{ToReceiver, Token};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Timer queue.
///
/// This data type manages a queue of [`Timer`] instances, which can be set,
/// reset, repeated, and cleared. Each timer is identified by a token, which
/// uniquely associates it to a node within a frontier. When a timer has been
/// set without [`Outputs`], it can't be set again, only reset or repeated.
///
/// Timers are the basis for all timed operators, such as `audit`, `debounce`,
/// `throttle`, and `sample`, which can be used to control the flow of actions.
#[derive(Debug)]
pub struct Timers<I> {
    /// Queue of timers.
    queue: Queue<Token, Timer<I>>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> Timers<I> {
    /// Creates a timer queue.
    #[must_use]
    pub fn new() -> Self {
        Self { queue: Queue::default() }
    }

    /// Submits a timer.
    #[allow(clippy::match_same_arms)]
    pub fn submit(&mut self, token: Token, timer: Timer<I>) {
        match timer {
            // Timer should be set, but not reset - we only overwrite the data
            // in case a previous timer exists, but don't change the deadline
            Timer::Set { deadline, data } => {
                if let Some(prior) = self.queue.get_mut(&token) {
                    *prior = Timer::Set {
                        deadline: match prior {
                            Timer::Set { deadline, .. } => *deadline,
                            Timer::Reset { deadline, .. } => *deadline,
                            Timer::Repeat { .. } => deadline,
                            Timer::Clear => unreachable!(),
                        },
                        data: prior.data().and(data),
                    };
                } else {
                    self.queue.insert(token, Timer::Set { deadline, data });
                    self.queue.set_deadline(&token, deadline);
                }
            }

            // Timer should always be reset - we overwrite the timer and reset
            // the deadline, effectively cancelling the previous timer
            timer @ Timer::Reset { deadline, .. } => {
                self.queue.insert(token, timer);
                self.queue.set_deadline(&token, deadline);
            }

            // Timer should be repeated - we overwrite the timer but don't reset
            // the deadline, as we need to let the active repetition complete
            timer @ Timer::Repeat { interval, .. } => {
                if self.queue.insert(token, timer).is_none() {
                    self.queue.set_deadline(&token, Instant::now() + interval);
                }
            }

            // Timer should be cleared - by removing the timer from the queue,
            // it's automatically cancelled, so nothing else needs to be done
            Timer::Clear => {
                self.queue.remove(&token);
            }
        }
    }

    /// Returns the next timer that is due.
    pub fn take(&mut self) -> Option<Job<I>> {
        let deadline = self.queue.deadline()?;
        self.queue.take().and_then(|(token, timer)| match timer {
            // In case of a one-shot timer, we just return the outputs together
            // with the associating token, so the scheduler can resolve it
            Timer::Set { data, .. } | Timer::Reset { data, .. } => {
                data.map(|outputs| (token, outputs))
            }

            // In case of a repeating timer, we create a new timer starting
            // from the current deadline, which ensures that the timer is not
            // skewed due to delays in processing. We always reset the data, so
            // it must be set explicitly again.
            Timer::Repeat { interval, data } => {
                let timer = Timer::Repeat { interval, data: None };

                // Create next timer and return outputs
                self.queue.insert(token, timer);
                self.queue.set_deadline(&token, deadline + interval);
                data.map(|outputs| (token, outputs))
            }

            // This variant is never stored, so it can't happen
            Timer::Clear => unreachable!(),
        })
    }
}

#[allow(clippy::must_use_candidate)]
impl<I> Timers<I> {
    // /// Returns the number of timers.
    // #[inline]
    // pub fn len(&self) -> usize {
    //     self.queue.len()
    // }

    /// Returns whether there are any timers.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I> ToReceiver<I> for Timers<I> {
    type Item = Instant;

    /// Creates a receiver for the timer queue.
    #[inline]
    fn to_receiver(&self) -> Cow<'_, Receiver<Self::Item>> {
        Cow::Owned(self.queue.deadline().map_or_else(never, at))
    }
}

// ----------------------------------------------------------------------------

impl<I> Default for Timers<I> {
    /// Creates a timer queue.
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// ----------------------------------------------------------------------------
// Type aliases
// ----------------------------------------------------------------------------

/// Timer queue job.
pub type Job<I> = (Token, Outputs<I>);
