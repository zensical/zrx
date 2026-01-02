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

//! Path transformations.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

/// Normalizes the given absolute or relative path.
///
/// This method combines all path components into a unified normalized path,
/// which consolidates all redundant or applicable components like consecutive
/// slashes, or `.` and `..` components, while making sure that relative paths
/// stay valid. Absolute paths are of course limited to their root directory.
///
/// Note that any returned relative path will never start with a component for
/// the current directory, i.e., `.`, and that trailing slashes are preserved,
/// which is essential for relative path computation, since trailing slashes
/// add a level of directory traversal.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use zrx_path::transform::normalize;
///
/// // Normalize path with `..` components
/// let path = normalize("a/../b");
/// assert_eq!(path, PathBuf::from("b"));
/// ```
pub fn normalize<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    let path = path.as_ref();

    // Analyze all components of the given path, and normalize all `.` and `..`
    // components, so that we get a comparable path, e.g., for relative URLs
    let mut stack = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match stack.last() {
                // If the current component is `..`, and we have a component on
                // the stack that resembles a normal path, remove the parent
                Some(Component::Normal(_)) => {
                    stack.pop();
                }
                // If the current component is `..`, and the last component is
                // another `..` component, or the stack is empty, add `..`
                Some(Component::ParentDir) | None => {
                    stack.push(Component::ParentDir);
                }
                // Otherwise just ignore `..`, which is the case when the prior
                // component is either a root or a prefix component
                Some(_) => {}
            },
            _ => stack.push(component),
        }
    }

    // Trailing slashes must be preserved, which Rust just doesn't when paths
    // are constructed, since relative path computation would be incorrect
    if path.to_string_lossy().ends_with(['/', '\\']) {
        stack.push(Component::Normal(OsStr::new("")));
    }

    // Collect components into path
    stack.into_iter().collect()
}

/// Creates a relative path from the given base path.
///
/// If the base path ends with a slash, its last component must be treated as
/// a folder. Otherwise, it's treated as a file. This is relevant for relative
/// path computation, as a folder at the end of the base path might require an
/// additional `..` component to be added to the relative path.
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use zrx_path::transform::relative_to;
///
/// // Create relative path from base
/// let path = relative_to("a/b/c", "a/d/e");
/// assert_eq!(path, PathBuf::from("../b/c"));
/// ```
pub fn relative_to<P, Q>(path: P, base: Q) -> PathBuf
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    let path = normalize(path);
    let base = normalize(base);

    // Collect all components from both paths
    let target = path.components().collect::<Vec<_>>();
    let mut source = base.components().collect::<Vec<_>>();

    // Start by searching for the common prefix of both paths, since those are
    // the parts that don't need to be traversed, and can thus be excluded
    let mut prefix = 0;
    while prefix < target.len() && prefix < source.len() {
        if target[prefix] == source[prefix] {
            prefix += 1;
        } else {
            break;
        }
    }

    // If the base path does not end in a trailing slash, it means we need to
    // compute the relative path from the folder the last component is in, so
    // we remove the last path segment from the base path. This also means all
    // paths that refer to folders must end in a slash.
    if !base.to_string_lossy().ends_with(['/', '\\']) {
        source.pop();
    }

    // Next, for each remaining component in the base path, add the same number
    // of `..` components to the relative path, as this is the number of parent
    // directories we need to traverse until we reach the common prefix, from
    // which we then descend into the target path.
    let mut stack = Vec::new();
    for _ in prefix..source.len() {
        stack.push(Component::ParentDir);
    }

    // Now, we can append the remaining components from the target path, and
    // descend the tree to the file or folder that we're interested in
    for &component in &target[prefix..] {
        stack.push(component);
    }

    // In case the base and target path are the same, we do not need to move at
    // all, which we denote by adding a `.` component
    if stack.is_empty() {
        stack.push(Component::CurDir);
    }

    // Trailing slashes must be preserved, or relative path computation will be
    // faulty, so preserve trailing slashes, which Rust doesn't do by default
    if path.to_string_lossy().ends_with(['/', '\\']) {
        stack.push(Component::Normal(OsStr::new("")));
    }

    // Collect components into path
    stack.into_iter().collect()
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {

    mod normalize {
        use std::path::Path;

        use crate::path::transform::normalize;

        #[test]
        fn handles_dot() {
            assert_eq!(normalize("a/./b"), Path::new("a/b"));
        }

        #[test]
        fn handles_dot_leading() {
            assert_eq!(normalize("./a/b"), Path::new("a/b"));
        }

        #[test]
        fn handles_dot_trailing() {
            assert_eq!(normalize("a/b/."), Path::new("a/b"));
        }

        #[test]
        fn handles_dot_sequence() {
            assert_eq!(normalize("a/././b"), Path::new("a/b"));
        }

        #[test]
        fn handles_dotdot() {
            assert_eq!(normalize("a/../b"), Path::new("b"));
        }

        #[test]
        fn handles_dotdot_leading() {
            assert_eq!(normalize("../a/b"), Path::new("../a/b"));
        }

        #[test]
        fn handles_dotdot_trailing() {
            assert_eq!(normalize("a/b/.."), Path::new("a"));
        }

        #[test]
        fn handles_dotdot_current() {
            assert_eq!(normalize("a/.."), Path::new(""));
        }

        #[test]
        fn handles_dotdot_parent() {
            assert_eq!(normalize("a/../.."), Path::new(".."));
        }

        #[test]
        fn handles_dotdot_nested() {
            assert_eq!(normalize("a/../../b"), Path::new("../b"));
        }

        #[test]
        fn handles_slashes() {
            assert_eq!(normalize("a//b//c"), Path::new("a/b/c"));
        }

        #[test]
        fn handles_folder() {
            assert_eq!(normalize("a/b/"), Path::new("a/b/"));
        }

        #[test]
        fn handles_folder_dotdot() {
            assert_eq!(normalize("a/../b/"), Path::new("b/"));
        }

        #[test]
        fn handles_folder_dotdot_leading() {
            assert_eq!(normalize("../a/b/"), Path::new("../a/b/"));
        }

        #[test]
        fn handles_folder_dotdot_trailing() {
            assert_eq!(normalize("a/b/../"), Path::new("a/"));
        }

        #[test]
        fn handles_folder_dotdot_current() {
            assert_eq!(normalize("a/../"), Path::new(""));
        }

        #[test]
        fn handles_folder_dotdot_parent() {
            assert_eq!(normalize("a/../../"), Path::new("../"));
        }

        #[test]
        fn handles_folder_dotdot_nested() {
            assert_eq!(normalize("a/../../b/"), Path::new("../b/"));
        }

        #[test]
        fn handles_folder_slashes() {
            assert_eq!(normalize("a//b//c/"), Path::new("a/b/c/"));
        }

        #[test]
        fn handles_empty() {
            assert_eq!(normalize(""), Path::new(""));
        }

        #[test]
        fn handles_empty_dot() {
            assert_eq!(normalize("."), Path::new(""));
        }

        #[test]
        fn handles_empty_dot_folder() {
            assert_eq!(normalize("./"), Path::new(""));
        }

        #[test]
        fn handles_empty_dot_sequence() {
            assert_eq!(normalize("./."), Path::new(""));
        }

        #[test]
        fn handles_absolute_dot() {
            assert_eq!(normalize("/a/./b"), Path::new("/a/b"));
        }

        #[test]
        fn handles_absolute_dot_leading() {
            assert_eq!(normalize("/./a"), Path::new("/a"));
        }

        #[test]
        fn handles_absolute_dot_trailing() {
            assert_eq!(normalize("/a/."), Path::new("/a"));
        }

        #[test]
        fn handles_absolute_dotdot() {
            assert_eq!(normalize("/a/../b"), Path::new("/b"));
        }

        #[test]
        fn handles_absolute_dotdot_leading() {
            assert_eq!(normalize("/../a"), Path::new("/a"));
        }

        #[test]
        fn handles_absolute_dotdot_trailing() {
            assert_eq!(normalize("/a/.."), Path::new("/"));
        }
    }

    mod relative_to {
        use std::path::Path;

        use crate::path::transform::relative_to;

        #[test]
        fn handles_current() {
            assert_eq!(relative_to("a/b", "a/b"), Path::new("."));
        }

        #[test]
        fn handles_nested() {
            assert_eq!(relative_to("a/b", "a"), Path::new("b"));
        }

        #[test]
        fn handles_nested_extension() {
            assert_eq!(relative_to("a/b.ext", "a"), Path::new("b.ext"));
        }

        #[test]
        fn handles_nested_folder() {
            assert_eq!(relative_to("a/b/", "a"), Path::new("b/"));
        }

        #[test]
        fn handles_parent() {
            assert_eq!(relative_to("a", "a/b"), Path::new("."));
        }

        #[test]
        fn handles_parent_extension() {
            assert_eq!(relative_to("a", "a/b.ext"), Path::new("."));
        }

        #[test]
        fn handles_parent_folder() {
            assert_eq!(relative_to("a", "a/b/"), Path::new(".."));
        }

        #[test]
        fn handles_sibling() {
            assert_eq!(relative_to("a/b", "a/c"), Path::new("b"));
        }

        #[test]
        fn handles_sibling_folder() {
            assert_eq!(relative_to("a/b", "a/c/"), Path::new("../b"));
        }

        #[test]
        fn handles_dotdot() {
            assert_eq!(relative_to(".", ".."), Path::new("."));
        }

        #[test]
        fn handles_dotdot_nested() {
            assert_eq!(relative_to("a", ".."), Path::new("a"));
        }

        #[test]
        fn handles_dotdot_parent() {
            assert_eq!(relative_to("a", "../.."), Path::new("../a"));
        }

        #[test]
        fn handles_empty() {
            assert_eq!(relative_to("", ""), Path::new("."));
        }

        #[test]
        fn handles_empty_path() {
            assert_eq!(relative_to("", "a"), Path::new("."));
        }

        #[test]
        fn handles_empty_base() {
            assert_eq!(relative_to("a", ""), Path::new("a"));
        }
    }
}
