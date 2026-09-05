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

//! Concurrent input readiness and sender teardown tests.

use std::time::{Duration, Instant};

use zrx_executor::Strategy;
use zrx_executor::strategy::{Immediate, WorkSharing};
use zrx_scheduler::Settlement;
use zrx_stream::{Advance, Change, Key, Workflow};

fn concurrent_bursts<S: Strategy>(strategy: S) {
    let workflow = Workflow::<u64>::build(|w| {
        for _ in 0..128 {
            let _ = w.input::<u64>();
        }
        for _ in 0..4 {
            let input = w.input::<u64>();
            w.output(&input);
        }
    });
    let ports: Vec<_> = workflow.inputs().copied().collect();
    let mut runner = workflow.runner_with(strategy).unwrap();
    let quiet: Vec<_> = ports[..128]
        .iter()
        .map(|p| runner.input_at::<u64>(*p).unwrap())
        .collect();
    let threads: Vec<_> = ports[128..]
        .iter()
        .enumerate()
        .map(|(producer, p)| {
            let mut input = runner.input_at::<u64>(*p).unwrap();
            std::thread::spawn(move || {
                for revision in 0..8 {
                    let mut writer = input.begin().unwrap();
                    for item in 0..96 {
                        let id = u64::try_from(
                            producer * 768 + revision * 96 + item,
                        )
                        .unwrap();
                        assert_eq!(
                            writer
                                .emit_from(&mut std::iter::once(
                                    Change::Insert(Key::from(id), id + 1)
                                ))
                                .unwrap(),
                            1
                        );
                        if item % 7 == 0 {
                            std::thread::yield_now();
                        }
                    }
                    input = writer.seal().unwrap();
                }
                input = input.begin().unwrap().abort().unwrap();
                drop(input.begin().unwrap());
            })
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(20);
    let (mut complete, mut aborted, mut count, mut sum) = (0, 0, 0, 0u64);
    loop {
        assert!(Instant::now() < deadline, "producer or readiness stalled");
        match runner.advance().unwrap() {
            Advance::Output(batch) => {
                for change in batch.into_changes::<u64>() {
                    if let Change::Insert(_, value) = change {
                        count += 1;
                        sum += value;
                    }
                }
            }
            Advance::Progress(report) => {
                for settlement in report.settlements() {
                    match settlement {
                        Settlement::Complete(_) => complete += 1,
                        Settlement::Aborted(_) => aborted += 1,
                    }
                }
            }
            Advance::Settled
                if threads.iter().all(std::thread::JoinHandle::is_finished) =>
            {
                break;
            }
            Advance::Settled | Advance::Idle => std::thread::yield_now(),
        }
    }
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(
        (complete, aborted, count, sum),
        (32, 8, 3072, 3072 * 3073 / 2)
    );
    assert!(runner.errors().is_empty());
    drop(quiet);
    // Dropping quiet sessions reports disconnection once, then remains settled.
    assert!(matches!(runner.advance().unwrap(), Advance::Progress(_)));
    assert!(matches!(runner.advance().unwrap(), Advance::Settled));
}
#[test]
fn concurrent_bursts_immediate() {
    concurrent_bursts(Immediate::new());
}
#[test]
fn concurrent_bursts_workers() {
    concurrent_bursts(WorkSharing::new(4));
}
