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

//! Stream function tests.

use std::fmt;

use zrx_scheduler::Value;
use zrx_stream::function::arguments::WithValue;
use zrx_stream::function::{MapFn, Scope};
use zrx_stream::{Change, Key, run};

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct TestId(u64);

// ----------------------------------------------------------------------------

struct PrintableId;

// ----------------------------------------------------------------------------
// Trait implementations
// ----------------------------------------------------------------------------

impl Value for TestId {}

impl fmt::Display for TestId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

// ----------------------------------------------------------------------------

impl fmt::Display for PrintableId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("id")
    }
}

// ----------------------------------------------------------------------------
// Functions
// ----------------------------------------------------------------------------

#[test]
fn callback_contract_requires_only_a_printable_identifier() {
    fn accepts<F>(_: F)
    where
        F: MapFn<WithValue, PrintableId, u64, u64>,
    {
    }

    accepts(|value: &u64| *value);
}

#[test]
fn invokes_all_plain_callback_shapes_through_the_public_runtime() {
    let changes: Result<Vec<_>, _> = run::<TestId, _>(|scope| {
        scope
            .iter([(TestId(7), 3_u64)])
            .map(|scope: &mut Scope<'_, TestId>| {
                scope.key().try_as_id().unwrap().0
            })
            .map(|scope: &mut Scope<'_, TestId>, value: &u64| {
                scope.key().try_as_id().unwrap().0 + *value
            })
            .map(|key: &Key<TestId>| key.try_as_id().unwrap().0)
            .map(|key: &Key<TestId>, value: &u64| {
                key.try_as_id().unwrap().0 + *value
            })
            .map(|id: &TestId| id.0)
            .map(|id: &TestId, value: &u64| id.0 + *value)
            .map(|value: &u64| *value * 2)
    })
    .unwrap()
    .collect();

    let changes = changes.unwrap();
    assert!(matches!(
        changes.as_slice(),
        [Change::Insert(key, 28)] if key == &Key::from(TestId(7))
    ));
}

#[test]
fn invokes_all_splat_callback_shapes_through_the_public_runtime() {
    let changes: Result<Vec<_>, _> = run::<TestId, _>(|scope| {
        scope.iter([(TestId(7), (2_u64, 3_u64))]).map(
            |scope: &mut Scope<'_, TestId>, a: &u64, b: &u64| {
                scope.key().try_as_id().unwrap().0 + *a + *b
            },
        )
    })
    .unwrap()
    .collect();
    assert!(matches!(
        changes.unwrap().as_slice(),
        [Change::Insert(_, 12)]
    ));

    let changes: Result<Vec<_>, _> = run::<TestId, _>(|scope| {
        scope.iter([(TestId(7), (2_u64, 3_u64))]).map(
            |key: &Key<TestId>, a: &u64, b: &u64| {
                key.try_as_id().unwrap().0 + *a + *b
            },
        )
    })
    .unwrap()
    .collect();
    assert!(matches!(
        changes.unwrap().as_slice(),
        [Change::Insert(_, 12)]
    ));

    let changes: Result<Vec<_>, _> = run::<TestId, _>(|scope| {
        scope
            .iter([(TestId(7), (2_u64, 3_u64))])
            .map(|id: &TestId, a: &u64, b: &u64| id.0 + *a + *b)
    })
    .unwrap()
    .collect();
    assert!(matches!(
        changes.unwrap().as_slice(),
        [Change::Insert(_, 12)]
    ));

    let changes: Result<Vec<_>, _> = run::<TestId, _>(|scope| {
        scope
            .iter([(
                TestId(7),
                (1_u64, 2_u64, 3_u64, 4_u64, 5_u64, 6_u64, 7_u64, 8_u64),
            )])
            .map(
                |a: &u64,
                 b: &u64,
                 c: &u64,
                 d: &u64,
                 e: &u64,
                 f: &u64,
                 g: &u64,
                 h: &u64| {
                    *a + *b + *c + *d + *e + *f + *g + *h
                },
            )
    })
    .unwrap()
    .collect();
    assert!(matches!(
        changes.unwrap().as_slice(),
        [Change::Insert(_, 36)]
    ));
}

#[test]
fn reports_declared_errors_and_caught_user_panics() {
    let mut execution = run::<TestId, _>(|scope| {
        scope.iter([(TestId(1), ()), (TestId(2), ())]).map(
            |id: &TestId,
             (): &()|
             -> std::result::Result<u64, std::io::Error> {
                if id.0 == 1 {
                    Err(std::io::Error::other("declared"))
                } else {
                    panic!("callback failed")
                }
            },
        )
    })
    .unwrap();

    let changes: Result<Vec<_>, _> = execution.by_ref().collect();
    assert!(changes.unwrap().is_empty());
    let report = execution.finish().unwrap();
    let failures: Vec<_> = report
        .invocations()
        .iter()
        .flat_map(|invocation| invocation.outcomes.failures())
        .map(ToString::to_string)
        .collect();
    assert_eq!(failures, ["declared", "caught panic: callback failed"]);
}

#[test]
fn reports_identifier_projection_failure_for_a_composite_key() {
    let mut execution = run::<TestId, _>(|scope| {
        scope
            .iter([(Key::from_iter([TestId(1), TestId(2)]), ())])
            .map(|id: &TestId, (): &()| id.0)
    })
    .unwrap();

    let changes: Result<Vec<_>, _> = execution.by_ref().collect();
    assert!(changes.unwrap().is_empty());
    let report = execution.finish().unwrap();
    let failures: Vec<_> = report
        .invocations()
        .iter()
        .flat_map(|invocation| invocation.outcomes.failures())
        .map(ToString::to_string)
        .collect();
    assert_eq!(failures, ["key depth exceeds one level"]);
}
