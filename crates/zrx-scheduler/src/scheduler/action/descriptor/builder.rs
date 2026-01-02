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

//! Descriptor builder.

use super::interest::Interest;
use super::property::Property;
use super::Descriptor;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Descriptor builder.
#[derive(Clone, Debug, Default)]
pub struct Builder {
    /// Action properties.
    properties: Vec<Property>,
    /// Action interest.
    interests: Vec<Interest>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Builder {
    /// Creates a descriptor builder.
    ///
    /// Note that the canonical way to create a [`Descriptor`] is to invoke the
    /// [`Descriptor::builder`] method, which creates an instance of [`Builder`].
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::descriptor::Builder;
    ///
    /// // Create descriptor builder
    /// let mut builder = Builder::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a property to the descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::descriptor::Property;
    /// use zrx_scheduler::action::Descriptor;
    ///
    /// // Create descriptor builder and add property
    /// let mut builder = Descriptor::builder().property(Property::Pure);
    /// ```
    #[inline]
    #[must_use]
    pub fn property(mut self, property: Property) -> Self {
        self.properties.push(property);
        self
    }

    /// Adds a property to the descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::descriptor::Interest;
    /// use zrx_scheduler::action::Descriptor;
    ///
    /// // Create descriptor builder and add interest
    /// let mut builder = Descriptor::builder().interest(Interest::Submit);
    /// ```
    #[inline]
    #[must_use]
    pub fn interest(mut self, interest: Interest) -> Self {
        self.interests.push(interest);
        self
    }

    /// Builds the descriptor.
    ///
    /// # Examples
    ///
    /// ```
    /// use zrx_scheduler::action::descriptor::Property;
    /// use zrx_scheduler::action::Descriptor;
    ///
    /// // Create descriptor builder
    /// let mut builder = Descriptor::builder();
    ///
    /// // Create descriptor from builder
    /// let descriptor = builder
    ///     .property(Property::Pure)
    ///     .property(Property::Stable)
    ///     .build();
    /// ```
    #[inline]
    pub fn build(self) -> Descriptor {
        Descriptor {
            properties: self.properties,
            interests: self.interests,
        }
    }
}
