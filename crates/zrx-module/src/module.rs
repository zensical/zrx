// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// Third-party contributions licensed under DCO

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

//! Module.

pub mod context;
pub mod error;

use context::Context;
use error::Result;

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

/// Module.
///
/// It's modules all the way down – the fundamental idea of this library is that
/// all specific functionality can and should be provided as modules, resulting
/// in an inherently flexible and composable system, where users can pick and
/// choose each module according to their requirements.
///
/// # Architecture
///
/// This crate is carefully integrated with other crates of this library, most
/// notably the [`zrx-id`] and [`zrx-stream`] crates.
///
/// - Each module is built from a dedicated [`Workflow`][], which represents a
///   set of [`Stream`][] transformations. In workflows, streams can be combined
///   and transformed using operators like [`Stream::join`][], [`Stream::map`][]
///   and many others, a concept borrowed from reactive programming.
///
/// - Modules can be attached and detached at runtime, allowing users to easily
///   add and remove functionality as required. The scheduler, which takes care
///   of orchestration, automatically recomputes the execution plan whenever
///   modules are added or removed.
///
/// - Modules can co-operate through typed subscriptions. Within a module, typed
///   subscriptions can be created through [`Context::add`][], adding a source
///   [`Stream`][] to the module's workflow that automatically subscribes to
///   matching streams from other modules.
///
/// # Considerations
///
/// Creating a module system that stands the test of time is a non-trivial task.
/// Deliberate care must be taken to ensure that the system isn't only flexible
/// and extensible enough, but also that it can evolve without major breaking
/// changes. The following design decisions ensure just that:
///
/// - The API surface of the module system is intentionally minimal, made of a
///   single entrypoint, [`Module::setup`]. As cooperation and synchronization
///   of modules is implemented with graphs, a single entrypoint is sufficient
///   for all use cases, and allows to keep the API simple and extensible.
///
/// - Upon initialization, modules are passed a [`Context`] for all interactions
///   with the system, which allows us to add new functionality without breaking
///   changes, as long as the signature of existing functions stays the same. It
///   also means experimental features can be gated behind feature flags.
///
/// - Modules are not generic over identifier types, like large parts of this
///   library, but tied to [`Id`][]. The reason is that the [`zrx-id`] crate is
///   the canonical way of working with identifiers, and the module system is
///   designed to be tightly integrated with it.
///
/// Our design is fundamentally different from some plugin and module systems
/// that expose a limited number of extension points (often called hooks), each
/// of which requires a static signature. This makes it much harder to evolve
/// without breaking the entire ecosystem. By contrast, our design doesn't come
/// with a fixed number of extension points, but allows users to create their
/// own extension points through subscriptions.
///
/// [`Error`]: crate::module::error::Error
/// [`Id`]: zrx_id::Id
/// [`Stream`]: zrx_stream::Stream
/// [`Stream::join`]: zrx_stream::Stream::join
/// [`Stream::map`]: zrx_stream::Stream::map
/// [`Workflow`]: zrx_stream::Workflow
/// [`zrx-id`]: zrx_id
/// [`zrx-stream`]: zrx_stream
pub trait Module {
    /// Initializes the module.
    ///
    /// # Errors
    ///
    /// This method should return an error when a problem is encountered trying
    /// to initialize the module.
    fn setup(&self, ctx: &mut Context) -> Result;
}
