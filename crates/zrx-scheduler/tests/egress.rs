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

//! Egress integration tests.

use std::sync::{Arc, Mutex};

use zrx_executor::strategy::WorkSharing;
use zrx_scheduler::Change;
use zrx_scheduler::Settlement;
use zrx_scheduler::action::{Action, Context, Job};
use zrx_scheduler::plan::{
    InputBinding, InputId, OutputBinding, OutputError, OutputId, Plan,
    PlanError, Route,
};

#[path = "support/runtime.rs"]
mod support;
use support::{Batch, Runtime};

const INPUT: InputId = InputId::new(1);
const OUTPUT: OutputId = OutputId::new(1);
const OUTPUT_B: OutputId = OutputId::new(2);

fn assert_complete(settlements: &[Settlement]) {
    assert!(matches!(settlements, [Settlement::Complete(_)]));
}

fn assert_aborted(settlements: &[Settlement]) {
    assert!(matches!(settlements, [Settlement::Aborted(_)]));
}

fn item(value: u64) -> Change<u64, u64> {
    Change::Insert(value, value * 10)
}

struct Pass;

impl Action<u64> for Pass {
    type Inputs = (u64,);
    type Output = u64;

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, emit| {
            match change {
                Change::Insert(key, value) => {
                    emit.insert(key, value.into_owned());
                }
                Change::Remove(key) => emit.remove(key),
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

struct Collect(Arc<Mutex<Vec<(u64, u64)>>>);

impl Action<u64> for Collect {
    type Inputs = (u64,);
    type Output = ();

    fn execute(&mut self, context: Context<'_, u64, Self>) {
        let Context {
            inputs: input, output, events, ..
        } = context;
        input.for_each(output, |change, _| {
            if let Change::Insert(key, value) = change {
                self.0.lock().unwrap().push((key, *value.as_ref()));
            }
            Ok(())
        });
        events.for_each(output, |_, _| Ok(()));
    }
}

fn program(routes: Vec<Vec<Route>>, jobs: Vec<Job<u64>>) -> Plan<u64> {
    Plan::builder(jobs, routes)
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT, 0)])
        .build()
        .unwrap()
}

fn take(runtime: &mut Runtime<u64>) -> Vec<(u64, u64)> {
    let mut values = Vec::new();
    let egress = runtime.egress().expect("egress batch is visible");
    assert_eq!(egress.output(), OUTPUT);
    egress.for_each::<u64>(|change| {
        if let Change::Insert(key, value) = change {
            values.push((key, *value.as_ref()));
        }
    });
    values
}

#[test]
fn egress_acceptance_retires_the_boundary_obligation() {
    let mut runtime = Runtime::new(program(vec![vec![]], vec![Job::new(Pass)]));
    let revision = runtime.begin(INPUT).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1), item(2)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let report = runtime.run_until_idle();
    assert!(report.settlements().is_empty());
    assert_eq!(take(&mut runtime), [(1, 10), (2, 20)]);
    let tick = runtime.tick();
    assert!(!tick.progressed());
    assert_complete(tick.into_report().settlements());
    assert!(runtime.egress().is_none());
}

#[test]
fn full_egress_preserves_and_orders_later_dispatches() {
    let mut runtime = Runtime::new(program(vec![vec![]], vec![Job::new(Pass)]));
    let revision = runtime.begin(INPUT).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1)]))
        .unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(2)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let report = runtime.run_until_idle();
    assert!(report.settlements().is_empty());
    assert_eq!(take(&mut runtime), [(1, 10)]);
    let report = runtime.run_until_idle();
    assert!(report.settlements().is_empty());
    assert_eq!(take(&mut runtime), [(2, 20)]);
    assert_complete(runtime.tick().into_report().settlements());
}

#[test]
fn ready_egress_rotates_after_acceptance_despite_replenishment() {
    let program = Plan::builder(vec![Job::new(Pass)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .outputs(vec![
            OutputBinding::new::<u64, u64>(OUTPUT, 0),
            OutputBinding::new::<u64, u64>(OUTPUT_B, 0),
        ])
        .build()
        .unwrap();
    let mut runtime =
        Runtime::with_strategy(program, WorkSharing::with_capacity(1, 1));
    let revision = runtime.begin(INPUT).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1)]))
        .unwrap();
    let _ = runtime.run_until_idle();

    assert_eq!(runtime.egress().unwrap().output(), OUTPUT);

    runtime
        .ingress(revision, Batch::new(vec![item(2)]))
        .unwrap();
    let _ = runtime.run_until_idle();

    assert_eq!(runtime.egress().unwrap().output(), OUTPUT_B);
    assert_eq!(runtime.egress().unwrap().output(), OUTPUT);
}

#[test]
fn one_output_fans_out_to_internal_and_external_destinations() {
    let internal = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = Runtime::new(program(
        vec![vec![Route::new(1, 0)], vec![]],
        vec![Job::new(Pass), Job::new(Collect(Arc::clone(&internal)))],
    ));
    let revision = runtime.begin(INPUT).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1)]))
        .unwrap();
    runtime.seal(revision).unwrap();

    let report = runtime.run_until_idle();
    assert!(report.settlements().is_empty());
    assert_eq!(*internal.lock().unwrap(), [(1, 10)]);
    assert_eq!(take(&mut runtime), [(1, 10)]);
    assert_complete(runtime.tick().into_report().settlements());
}

#[test]
fn abort_discards_unaccepted_egress_and_releases_its_obligation() {
    let mut runtime = Runtime::new(program(vec![vec![]], vec![Job::new(Pass)]));
    let revision = runtime.begin(INPUT).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1)]))
        .unwrap();
    let report = runtime.run_until_idle();
    assert!(report.settlements().is_empty());

    runtime.abort(revision).unwrap();
    let report = runtime.run_until_idle();
    assert!(runtime.egress().is_none());
    assert_aborted(report.settlements());
}

#[test]
fn construction_rejects_invalid_and_duplicate_outputs() {
    let wrong = Plan::builder(vec![Job::new(Pass)], vec![vec![]])
        .outputs(vec![OutputBinding::new::<u64, usize>(OUTPUT, 0)])
        .build();
    assert!(matches!(
        wrong,
        Err(PlanError::Output(OutputError::Invalid(OUTPUT)))
    ));

    let duplicate = Plan::builder(vec![Job::new(Pass)], vec![vec![]])
        .outputs(vec![
            OutputBinding::new::<u64, u64>(OUTPUT, 0),
            OutputBinding::new::<u64, u64>(OUTPUT, 0),
        ])
        .build();
    assert!(matches!(
        duplicate,
        Err(PlanError::Output(OutputError::Duplicate(OUTPUT)))
    ));
}

#[test]
fn reinstalling_outputs_replaces_compiled_destinations() {
    let program = Plan::builder(vec![Job::new(Pass)], vec![vec![]])
        .inputs(vec![InputBinding::new::<u64, u64>(INPUT, Route::new(0, 0))])
        .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT, 0)])
        .outputs(vec![OutputBinding::new::<u64, u64>(OUTPUT_B, 0)])
        .build()
        .unwrap();
    let mut runtime = Runtime::new(program);
    let revision = runtime.begin(INPUT).unwrap();

    runtime
        .ingress(revision, Batch::new(vec![item(1)]))
        .unwrap();
    let _ = runtime.run_until_idle();

    assert_eq!(runtime.egress().unwrap().output(), OUTPUT_B);
    assert!(runtime.egress().is_none());
}
