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

//! Sender set.

use crossbeam::channel::Sender;

use crate::scheduler::engine::Token;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Sender set.
#[derive(Debug)]
pub struct Senders<T> {
    inner: Vec<(Token, Sender<T>)>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<T> Senders<T> {
    /// Adds a sender to the sender set.
    #[inline]
    pub fn add(&mut self, token: Token, sender: Sender<T>) {
        self.inner.push((token, sender));
    }

    /// Sends a value to all senders in the sender set.
    pub fn send(&mut self, value: T) -> Vec<Token>
    where
        T: Clone,
    {
        let mut tokens = Vec::new();
        let mut i = 0;

        // Send to all but last sender, cloning value for each send - in case a
        // sender has been dropped, we just swap it with the last sender.
        while i < self.inner.len().saturating_sub(1) {
            if self.inner[i].1.send(value.clone()).is_err() {
                self.inner.swap_remove(i);
            } else {
                tokens.push(self.inner[i].0);
                i += 1;
            }
        }

        // Send to last sender without cloning
        if !self.inner.is_empty() && self.inner[i].1.send(value).is_err() {
            self.inner.swap_remove(i);
        } else {
            tokens.push(self.inner[i].0);
        }

        // Return tokens
        tokens
    }
}

// #[allow(clippy::must_use_candidate)]
// impl<T> Senders<T> {
//     /// Returns the number of senders.
//     #[inline]
//     pub fn len(&self) -> usize {
//         self.inner.len()
//     }

//     /// Returns whether there are any senders.
//     #[inline]
//     pub fn is_empty(&self) -> bool {
//         self.inner.is_empty()
//     }
// }

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<T> Default for Senders<T> {
    /// Creates a sender set.
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::default() }
    }
}
