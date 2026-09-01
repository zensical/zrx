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

//! Map tests.

use std::sync::Mutex;

use zrx_diagnostic::sink::Sink;
use zrx_executor::strategy::Immediate;
use zrx_scheduler::action::Record;
use zrx_stream::function::Scope;
use zrx_stream::{Change, Key, Workflow, sequential};

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[test]
fn map_callbacks_execute_through_the_runner_boundary() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u64>();
        let multiplier = Mutex::new(2_u64);
        let mapped = source
            .map(sequential(
                move |scope: &mut Scope<'_, u64>, value: &u64| {
                    scope.mark("mapping");
                    scope.measure("multiply", |scope| {
                        scope
                            .emit(zrx_diagnostic::warning!("mapped {}", value));
                        *value * *multiplier.lock().unwrap()
                    })
                },
            ))
            .map(|id: &u64, value: &u64| *id + *value);
        workflow.output(&mapped);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(7_u64), 3).unwrap();
    let _input = revision.seal().unwrap();

    let mut run = runner.settle().unwrap();
    let values: Vec<_> = run.output::<u64>().unwrap().collect();
    let diagnostics: Vec<_> =
        run.report()
            .invocations()
            .iter()
            .flat_map(|invocation| {
                invocation.instrumentation.records().iter().filter_map(
                    |entry| {
                        let Record::Diagnostic(diagnostic) = entry else {
                            return None;
                        };
                        Some(diagnostic.message.clone())
                    },
                )
            })
            .collect();
    let annotations: Vec<_> =
        run.report()
            .invocations()
            .iter()
            .flat_map(|invocation| {
                invocation.instrumentation.records().iter().filter_map(
                    |entry| {
                        let Record::Annotation(annotation) = entry else {
                            return None;
                        };
                        Some(annotation.name())
                    },
                )
            })
            .collect();
    let measurements: Vec<_> =
        run.report()
            .invocations()
            .iter()
            .flat_map(|invocation| {
                invocation.instrumentation.records().iter().filter_map(
                    |entry| {
                        let Record::Measurement(measurement) = entry else {
                            return None;
                        };
                        Some(measurement.name())
                    },
                )
            })
            .collect();

    assert!(matches!(
        values.as_slice(),
        [Change::Insert(key, 13)] if key == &Key::from(7_u64)
    ));
    assert_eq!(diagnostics, ["mapped 3"]);
    assert_eq!(annotations, ["mapping"]);
    assert_eq!(measurements, ["multiply"]);
}

// ----------------------------------------------------------------------------

#[test]
fn filter_map_emits_insertions_and_retractions() {
    let workflow = Workflow::<u64>::build(|workflow| {
        let source = workflow.input::<u64>();
        let filtered =
            source.filter_map(sequential(|id: &u64, value: &u64| {
                value.is_multiple_of(2).then_some(*id + *value)
            }));
        workflow.output(&filtered);
    });
    let mut runner = workflow.runner_with(Immediate::new()).unwrap();
    let input = runner.input::<u64>().unwrap();
    let mut revision = input.begin().unwrap();
    revision.insert(Key::from(1_u64), 2).unwrap();
    revision.insert(Key::from(2_u64), 3).unwrap();
    let _input = revision.seal().unwrap();

    let mut run = runner.settle().unwrap();
    let changes: Vec<_> = run.output::<u64>().unwrap().collect();
    assert!(matches!(
        changes.as_slice(),
        [Change::Insert(first, 3), Change::Remove(second)]
            if first == &Key::from(1_u64) && second == &Key::from(2_u64)
    ));
}
