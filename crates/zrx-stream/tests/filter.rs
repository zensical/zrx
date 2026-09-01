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

//! Filter tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Workflow, sequential};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[test]
fn filter_preserves_matching_values_and_retracts_non_matches() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<i64>();
        let filtered =
            source.filter(sequential(|value: &i64| value.rem_euclid(2) == 0));
        workflow.output(&filtered);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<i64>().unwrap();
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 2).unwrap();
    revision.insert(Key::from(2_u64), 3).unwrap();
    let _input = revision.seal().unwrap();

    let mut run = runner.settle().unwrap();
    let changes: Vec<_> = run.output::<i64>().unwrap().collect();
    assert!(matches!(
        changes.as_slice(),
        [Change::Insert(first, 2), Change::Remove(second)]
            if first == &Key::from(1_u64) && second == &Key::from(2_u64)
    ));
}
