// Copyright (c) 2025-2026 Zensical and contributors

// SPDX-License-Identifier: MIT
// All contributions are certified under the DCO

use std::collections::{BTreeMap, HashMap};

use slab::Slab;
use zrx_store::entry::{OccupiedEntry as _, VacantEntry as _};
use zrx_store::{Stash, StoreEntry};

fn exercise_entry<S>(mut store: S)
where
    S: StoreEntry<String, u64>,
{
    {
        let entry = StoreEntry::entry(&mut store, "key".to_owned());
        assert_eq!(entry.key(), "key");
        let zrx_store::entry::Entry::Vacant(entry) = entry else {
            panic!("entry must be vacant")
        };
        let value = entry.insert(21);
        assert_eq!(value, &21);
    }

    let value = StoreEntry::entry(&mut store, "key".to_owned())
        .and_modify(|value| *value *= 2)
        .or_insert_with(|| panic!("occupied entry invoked initializer"));
    assert_eq!(value, &42);

    {
        let entry = StoreEntry::entry(&mut store, "key".to_owned());
        assert_eq!(entry.key(), "key");
        let zrx_store::entry::Entry::Occupied(mut entry) = entry else {
            panic!("entry must be occupied")
        };
        assert_eq!(entry.key(), "key");
        assert_eq!(entry.get(), &42);
        assert_eq!(entry.insert(84), Some(42));
        assert_eq!(entry.insert(84), None);
        assert_eq!(entry.remove_entry(), ("key".to_owned(), 84));
    }
    assert!(store.is_empty());

    let value =
        StoreEntry::entry(&mut store, "default".to_owned()).or_default();
    assert_eq!(value, &0);

    {
        let zrx_store::entry::Entry::Occupied(entry) =
            StoreEntry::entry(&mut store, "default".to_owned())
        else {
            panic!("entry must be occupied")
        };
        assert_eq!(entry.remove(), 0);
    }
    assert!(store.is_empty());

    {
        let entry = StoreEntry::entry(&mut store, "unused".to_owned())
            .and_modify(|_| panic!("vacant entry invoked modifier"));
        let zrx_store::entry::Entry::Vacant(entry) = entry else {
            panic!("entry must be vacant")
        };
        let key = entry.into_key();
        assert_eq!(key, "unused");
    }

    let value = StoreEntry::entry(&mut store, "length".to_owned())
        .or_insert_with_key(|key| key.len() as u64);
    assert_eq!(value, &6);
}

#[test]
fn hash_map_retains_one_native_entry() {
    exercise_entry(HashMap::new());
}

#[test]
fn btree_map_retains_one_native_entry() {
    exercise_entry(BTreeMap::new());
}

#[test]
fn slab_retains_one_located_entry() {
    exercise_entry(Slab::new());
}

#[test]
fn stash_retains_one_backing_entry() {
    exercise_entry(Stash::default());
}
