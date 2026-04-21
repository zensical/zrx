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

//! Function arguments.

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Function arguments.
pub trait Arguments: Send + Sync + 'static {}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

/// Marker for key arguments.
pub struct ForKey;

/// Marker for identifier arguments.
pub struct ForId;

/// Marker for value arguments.
pub struct ForValue;

/// Marker for splat arguments.
pub struct ForSplat;

/// Marker for key and value arguments.
pub struct ForKeyValue;

/// Marker for key and splat arguments.
pub struct ForKeySplat;

/// Marker for identifier and value arguments.
pub struct ForIdValue;

/// Marker for identifier and splat arguments.
pub struct ForIdSplat;

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> Arguments for T where T: Send + Sync + 'static {}
