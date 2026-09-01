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

//! Workflow tests.

use zrx_executor::strategy::Immediate;
use zrx_scheduler::action::control::{Event, ProgressEvent};
use zrx_scheduler::action::{Action, Context};
use zrx_stream::operator::Operator;
use zrx_stream::{Change, Key, Stream, Workflow};

// ----------------------------------------------------------------------------
// Traits
// ----------------------------------------------------------------------------

trait AtEndExt {
    fn at_end(&self) -> Stream<u64, u64>;
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct AtEnd;

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Action<Key<u64>> for AtEnd {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, Key<u64>, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |_, _| Ok(()));
        events.for_each(output, |event, emit| match event {
            Event::Progress(ProgressEvent::End) => {
                emit.insert(Key::from(0), 1);
                Ok(())
            }
            Event::Progress(ProgressEvent::Begin | ProgressEvent::Abort) => {
                Ok(())
            }
            Event::Wake { .. } => {
                unreachable!("progress-only test action received a wake")
            }
        });
    }
}

// ----------------------------------------------------------------------------

impl AtEndExt for Stream<u64, u64> {
    fn at_end(&self) -> Stream<u64, u64> {
        self.subscribe_progress(AtEnd)
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[test]
fn streams_expose_stable_workflow_local_node_identities() {
    let mut nodes = None;
    let _workflow = Workflow::<u32>::build(|workflow| {
        let source = workflow.input::<u64>();
        let source_node = source.node();
        let clone = source.clone();
        let mapped = source.map(|value: &u64| *value);
        nodes = Some((source_node, clone.node(), mapped.node()));
    });

    let (source, clone, mapped) =
        nodes.expect("workflow construction recorded nodes");
    assert_eq!(source, clone);
    assert_ne!(source, mapped);
}

#[test]
fn custom_operator_progress_is_derived_during_lowering() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<String>();
        let mapped = source.map(|value: &String| value.len() as u64);
        let settled = mapped.at_end();
        workflow.output(&settled);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<String>().unwrap();

    let mut revision = input.begin().unwrap();
    revision
        .insert(Key::from(1), String::from("value"))
        .unwrap();
    let _input = revision.seal().unwrap();
    let changes: Vec<_> =
        runner.settle().unwrap().output::<u64>().unwrap().collect();

    assert!(matches!(
        changes.as_slice(),
        [Change::Insert(key, 1)] if key == &Key::from(0)
    ));
}
