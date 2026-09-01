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

/// Marker for scope argument.
pub struct WithScope;

/// Marker for scope and value arguments.
pub struct WithScopeValue;

/// Marker for scope and splat arguments.
pub struct WithScopeSplat;

// ----------------------------------------------------------------------------

/// Marker for key argument.
pub struct WithKey;

/// Marker for key and value arguments.
pub struct WithKeyValue;

/// Marker for key and splat arguments.
pub struct WithKeySplat;

// ----------------------------------------------------------------------------

/// Marker for identifier argument.
pub struct WithId;

/// Marker for identifier and value arguments.
pub struct WithIdValue;

/// Marker for identifier and splat arguments.
pub struct WithIdSplat;

// ----------------------------------------------------------------------------

/// Marker for value argument.
pub struct WithValue;

/// Marker for splat argument.
pub struct WithSplat;

// ----------------------------------------------------------------------------
// Blanket implementations
// ----------------------------------------------------------------------------

impl<T> Arguments for T where T: Send + Sync + 'static {}
