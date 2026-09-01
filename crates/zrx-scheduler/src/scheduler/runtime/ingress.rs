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

//! Installed source bindings and their single-open-revision lifecycle.

use ahash::HashMap;
use thiserror::Error as ThisError;

use crate::scheduler::RevisionId;
use crate::scheduler::action::Port;
use crate::scheduler::plan::{InputBinding, InputId, InputIndex, Route};

// ----------------------------------------------------------------------------
// Enums
// ----------------------------------------------------------------------------

/// Invalid source ingress operation.
#[derive(Clone, Debug, PartialEq, Eq, ThisError)]
pub enum Error {
    /// The external input is not installed.
    #[error("external input {0:?} is not installed")]
    Input(InputId),
    /// The external input already has an open revision.
    #[error("external input {0:?} already has an open revision")]
    Open(InputId),
    /// The segment type differs from the installed input position.
    #[error("segment type differs from node {node} lane {lane}")]
    Port { node: usize, lane: usize },
}

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct Source {
    pub route: Route,
    pub port: Port,
}

// ----------------------------------------------------------------------------

struct State {
    source: Source,
    open: Option<RevisionId>,
}

// ----------------------------------------------------------------------------

pub struct Sources {
    states: Vec<State>,
    by_id: HashMap<InputId, InputIndex>,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl Sources {
    pub fn new(
        bindings: Vec<InputBinding>, by_id: HashMap<InputId, InputIndex>,
    ) -> Self {
        let states = bindings
            .into_iter()
            .map(|binding| State {
                source: Source {
                    route: binding.route,
                    port: binding.port,
                },
                open: None,
            })
            .collect();
        Self { states, by_id }
    }

    fn index(&self, input: InputId) -> Option<InputIndex> {
        self.by_id.get(&input).copied()
    }

    fn state(&self, input: InputIndex) -> &State {
        &self.states[input.get()]
    }

    fn state_mut(&mut self, input: InputIndex) -> &mut State {
        &mut self.states[input.get()]
    }

    pub fn resolve(&self, input: InputId) -> Result<InputIndex, Error> {
        self.index(input).ok_or(Error::Input(input))
    }

    pub fn available(
        &self, input: InputId,
    ) -> Result<(InputIndex, Source), Error> {
        let index = self.resolve(input)?;
        let state = self.state(index);
        if state.open.is_some() {
            return Err(Error::Open(input));
        }
        Ok((index, state.source))
    }

    pub fn open(&mut self, input: InputIndex, revision: RevisionId) {
        let open = &mut self.state_mut(input).open;
        assert!(open.replace(revision).is_none(), "source already open");
    }

    pub fn active(
        &self, input: InputIndex, revision: RevisionId,
    ) -> Option<Source> {
        let state = self.state(input);
        (state.open == Some(revision)).then_some(state.source)
    }

    pub fn source(&self, input: InputId) -> Option<Source> {
        self.index(input).map(|index| self.state(index).source)
    }

    pub fn source_at(&self, input: InputIndex) -> Source {
        self.state(input).source
    }

    pub fn close(&mut self, input: InputIndex, revision: RevisionId) -> bool {
        let state = self.state_mut(input);
        if state.open != Some(revision) {
            return false;
        }
        state.open = None;
        true
    }
}
