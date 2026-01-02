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

//! Macros for graph creation.

// ----------------------------------------------------------------------------
// Macros
// ----------------------------------------------------------------------------

/// Creates a graph builder from the given pairs.
///
/// This macro creates a [`Builder`][] and adds nodes and edges based on the
/// provided source-target pairs. It's primarily intended for use in tests and
/// examples to quickly set up graphs, and is not optimized for performance.
/// Additionally, the node type must implement [`Copy`].
///
/// [`Builder`]: crate::graph::Builder
///
/// # Examples
///
/// ```
/// use zrx_graph::graph_builder;
///
/// // Create graph builder from pairs
/// let builder = graph_builder! {
///     "a" => "b", "a" => "c",
///     "b" => "c",
/// };
/// ```
#[macro_export]
macro_rules! graph_builder {
    ($($source:expr => $target:expr),+ $(,)?) => {{
        let mut builder = $crate::Graph::builder();
        let mut nodes = std::collections::BTreeMap::new();
        $(
            nodes.entry($source).or_insert_with(|| builder.add_node($source));
            nodes.entry($target).or_insert_with(|| builder.add_node($target));
        )*
        $(
            let _ = builder.add_edge(nodes[$source], nodes[$target], ());
        )*
        builder
    }};
}

/// Creates a graph from the given pairs.
///
/// This macro creates a [`Graph`][] and adds nodes and edges based on the
/// provided source-target pairs. It's primarily intended for use in tests and
/// examples to quickly set up graphs, and is not optimized for performance.
/// Additionally, the node type must implement [`Copy`].
///
/// In case you need a [`Builder`][], e.g. to inspect the graph before building
/// or to print it to DOT format, use the [`graph_builder!`][] macro.
///
/// [`Builder`]: crate::graph::Builder
/// [`Graph`]: crate::graph::Graph
///
/// # Examples
///
/// ```
/// use zrx_graph::graph;
///
/// // Create graph from pairs
/// let graph = graph! {
///     "a" => "b", "a" => "c",
///     "b" => "c",
/// };
/// ```
#[macro_export]
macro_rules! graph {
    ($($source:expr => $target:expr),+ $(,)?) => {{
        $crate::graph_builder!($($source => $target),+).build()
    }};
}
