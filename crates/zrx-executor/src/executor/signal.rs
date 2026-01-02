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

//! Signal for synchronization.

use std::sync::{Condvar, Mutex};

use super::error::{Error, Result};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Signal for synchronization.
pub struct Signal {
    /// Whether to continue or terminate.
    mutex: Mutex<bool>,
    /// Condition to block thread without busy-waiting.
    value: Condvar,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Signal {
    /// Creates a signal.
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(false),
            value: Condvar::new(),
        }
    }

    /// Returns whether the worker should terminate.
    ///
    /// This method is used inside a worker when there're no more tasks to be
    /// executed to hand back control to the executor, allowing the worker to
    /// terminate gracefully. The worker waits without consuming any CPU-time
    /// for the continuation or termination signal of the executor.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Signal`] if the signal is poisoned.
    pub fn should_terminate(&self) -> Result<bool> {
        self.mutex
            .lock()
            .and_then(|guard| {
                if *guard {
                    Ok(guard)
                } else {
                    self.value.wait(guard)
                }
            })
            .map_err(|_| Error::Signal)
            .map(|value| *value)
    }

    /// Signals the termination of the executor.
    ///
    /// This method is called when the executor is being terminated, ensuring
    /// that all workers can finish their work, and then terminate gracefully.
    ///
    /// # Errors
    ///
    /// This method returns [`Error::Signal`] if the signal is poisoned.
    pub fn terminate(&self) -> Result {
        self.mutex
            .lock()
            .map_err(|_| Error::Signal)
            .map(|mut value| *value = true)?;

        // Notify all workers.
        self.notify();
        Ok(())
    }

    /// Notifies all workers.
    pub fn notify(&self) {
        self.value.notify_all();
    }
}
