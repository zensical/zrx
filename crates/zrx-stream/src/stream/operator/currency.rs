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

//! Bounded mutation currency for overlapping keyed transitions.

use std::collections::hash_map::Entry;
use std::hash::Hash;

use ahash::HashMap;

// ----------------------------------------------------------------------------
// Structs
// ----------------------------------------------------------------------------

struct Currency {
    /// Latest mutation that can invalidate an unresolved transition.
    version: u64,
    /// Number of transitions still capable of publication.
    pending: usize,
}

// ----------------------------------------------------------------------------

/// Latest mutation versions retained while keyed transitions remain live.
pub(super) struct MutationCurrency<K> {
    current: HashMap<K, Currency>,
    next: u64,
}

// ----------------------------------------------------------------------------
// Implementations
// ----------------------------------------------------------------------------

impl<K> MutationCurrency<K>
where
    K: Clone + Eq + Hash,
{
    /// Creates empty mutation currency.
    pub(super) fn new() -> Self {
        Self {
            current: HashMap::default(),
            next: 0,
        }
    }

    /// Advances a key and optionally registers one new live transition.
    pub(super) fn mark(&mut self, key: &K, new_transition: bool) -> u64 {
        self.next = self
            .next
            .checked_add(1)
            .expect("mutation version exhausted");
        match self.current.entry(key.clone()) {
            Entry::Occupied(mut entry) => {
                let currency = entry.get_mut();
                currency.version = self.next;
                if new_transition {
                    currency.pending = currency
                        .pending
                        .checked_add(1)
                        .expect("pending transition count exhausted");
                }
            }
            Entry::Vacant(entry) => {
                assert!(new_transition, "mutation has no pending transition");
                entry.insert(Currency { version: self.next, pending: 1 });
            }
        }
        self.next
    }

    /// Returns whether `version` is still the latest mutation for `key`.
    pub(super) fn is_current(&self, key: &K, version: u64) -> bool {
        self.current
            .get(key)
            .is_some_and(|currency| currency.version == version)
    }

    /// Releases one transition and reclaims unused key currency.
    pub(super) fn release(&mut self, key: &K) {
        let remove = {
            let currency = self
                .current
                .get_mut(key)
                .expect("released transition has no mutation currency");
            currency.pending = currency
                .pending
                .checked_sub(1)
                .expect("pending transition count underflowed");
            currency.pending == 0
        };
        if remove {
            self.current.remove(key);
        }
    }

    /// Returns whether no transition retains mutation currency.
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.current.is_empty()
    }
}
