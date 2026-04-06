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

//! Generic sender.

use crossbeam::channel::Sender;
use std::any::Any;
use std::fmt::Debug;
use zrx_storage::Storage;

use zrx_storage::convert::TryAsStorage;

use crate::scheduler::router::transport::{Error, Result};
use crate::scheduler::signal::{Diff, Id, Scope, Value};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Generic sender.
pub trait AnySender<I>: Any + Debug {
    /// Creates a boxed clone of the sender.
    fn clone_box(&self) -> Box<dyn AnySender<I>>;
    /// Forward a value to the sender.
    fn forward(&self, storage: &dyn Any, key: &Scope<I>);
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<I> dyn AnySender<I>
where
    I: Id,
{
    /// Attempts to downcast to a reference to a sender of `T`.
    #[inline]
    pub fn downcast_ref<T>(&self) -> Result<&Sender<Diff<I, T>>>
    where
        T: Value,
    {
        (self as &dyn Any).downcast_ref().ok_or(Error::Downcast)
    }
}

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl<I, T> AnySender<I> for Sender<Diff<I, T>>
where
    I: Id,
    T: Value,
{
    /// Creates a boxed clone of the sender.
    #[inline]
    fn clone_box(&self) -> Box<dyn AnySender<I>> {
        Box::new(self.clone())
    }

    /// Forward a value to the sender.
    #[inline]
    fn forward(&self, storage: &dyn Any, key: &Scope<I>) {
        let storage: &Storage<Scope<I>, T> =
            T::try_as_storage(storage).expect("invariant");

        // Forward insert or remove diff to sender
        let _ = match storage.get(key) {
            Some(data) => self.send(Diff::Insert(key.clone(), data.clone())),
            None => self.send(Diff::Remove(key.clone())),
        };
    }
}
