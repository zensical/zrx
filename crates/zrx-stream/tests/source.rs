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

//! Source tests.

use zrx_executor::strategy::Immediate;
use zrx_stream::{Change, Key, Workflow};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[test]
fn source_forwards_one_segment_to_every_output() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<String>();
        workflow.output(&source);
        workflow.output(&source);
    });
    let input = workflow.input::<String>().unwrap();
    let outputs: Vec<_> =
        workflow.outputs().of_type::<String>().copied().collect();
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input_at::<String>(input).unwrap();

    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), "one".to_owned()).unwrap();
    revision.remove(Key::from(2_u64)).unwrap();
    let _input = revision.seal().unwrap();

    let mut run = runner.settle().unwrap();
    let first: Vec<_> = run.output_at::<String>(outputs[0]).unwrap().collect();
    let second: Vec<_> = run.output_at::<String>(outputs[1]).unwrap().collect();
    assert!(matches!(
        first.as_slice(),
        [Change::Insert(first, value), Change::Remove(second)]
            if first == &Key::from(1_u64)
                && value == "one"
                && second == &Key::from(2_u64)
    ));
    assert!(matches!(
        second.as_slice(),
        [Change::Insert(first, value), Change::Remove(second)]
            if first == &Key::from(1_u64)
                && value == "one"
                && second == &Key::from(2_u64)
    ));
}
