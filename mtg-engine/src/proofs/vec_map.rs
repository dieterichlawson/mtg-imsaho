//! `VecMap`, which a Kani build keeps the engine's objects and card names
//! in, is an ordered map: what `BTreeMap` is documented to be.
//!
//! The reference is the plainest ordered map there is, an array indexed by
//! key: a key holds the value last inserted for it until it is removed, and
//! the map's entries are the keys that hold one, in increasing key order.
//!
//! The proof is by induction over operations. Its invariant is that the map
//! holds the reference's entries, in the reference's order, which every
//! map built by `new` does. Each proof here starts from any map holding any
//! reference's entries (every subset of [`KEYS`] keys, any values, put in
//! in key order) and applies one operation (an insert, a remove, a lookup,
//! a write through `get_mut` or through `values_mut`) on any key and value. The operation returns what the reference says
//! (`insert` and `remove` the value that was there), and the map holds the
//! changed reference's entries afterwards, as `iter`, `keys`, `values` and
//! `len` read them. So every sequence of operations, of any length, answers
//! as the reference does.
//!
//! `BTreeMap` itself is not run here: CBMC does not get through a B-tree
//! with symbolic keys (three operations ran past 11 minutes and 7 GB). The
//! engine relies on `BTreeMap` only for what its documentation promises,
//! which is the reference's behaviour.
//!
//! The map is generic and never looks at a value except to store, return or
//! drop it, so what holds for `u8` values holds for `GameObject`s; keys are
//! compared only through `Ord`, so what holds for three keys holds for any
//! three keys of any ordered type, and the operations touch at most one
//! entry and its neighbours' order.

use crate::vec_map::VecMap;

/// Distinct keys.
const KEYS: usize = 3;

fn agrees(map: &VecMap<u8, u8>, reference: &[Option<u8>; KEYS]) -> bool {
    let mut expected: Vec<(u8, u8)> = Vec::with_capacity(KEYS);
    for (k, v) in reference.iter().enumerate() {
        if let Some(v) = v {
            expected.push((u8::try_from(k).expect("a few keys"), *v));
        }
    }
    map.len() == expected.len()
        && map.is_empty() == expected.is_empty()
        && map.iter().map(|(k, v)| (*k, *v)).eq(expected.iter().copied())
        && map.keys().copied().eq(expected.iter().map(|(k, _)| *k))
        && map.values().copied().eq(expected.iter().map(|(_, v)| *v))
}

/// Any map holding any reference's entries: each of [`KEYS`] keys held or
/// not, with any value, put in in key order. Room for every key is
/// reserved first, which changes nothing a map's operations return, so
/// that no insert reallocates (`proofs::cards::no_growing` says why).
fn any_map() -> (VecMap<u8, u8>, [Option<u8>; KEYS]) {
    let mut reference: [Option<u8>; KEYS] = [None; KEYS];
    let mut map: VecMap<u8, u8> = VecMap::new();
    map.reserve(KEYS);
    for (k, slot) in reference.iter_mut().enumerate() {
        if kani::any() {
            let v: u8 = kani::any();
            *slot = Some(v);
            map.insert(u8::try_from(k).expect("a few keys"), v);
        }
    }
    assert!(agrees(&map, &reference));
    (map, reference)
}

/// Any key.
fn any_key() -> (usize, u8) {
    let k: usize = kani::any();
    kani::assume(k < KEYS);
    (k, u8::try_from(k).expect("a few keys"))
}

/// `insert` returns the value the key held, and the key then holds the new
/// one, in its place in the order.
#[kani::proof]
#[kani::unwind(5)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
fn inserting_into_a_vec_map_is_inserting_into_an_ordered_map() {
    let (mut map, mut reference) = any_map();
    let (k, key) = any_key();
    let value: u8 = kani::any();
    assert!(map.insert(key, value) == reference[k].replace(value));
    assert!(agrees(&map, &reference));
    kani::cover!(map.len() == KEYS, "every key held");
}

/// `remove` returns the value the key held, and the key then holds none.
#[kani::proof]
#[kani::unwind(5)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
fn removing_from_a_vec_map_is_removing_from_an_ordered_map() {
    let (mut map, mut reference) = any_map();
    let (k, key) = any_key();
    assert!(map.remove(&key) == reference[k].take());
    assert!(agrees(&map, &reference));
    kani::cover!(map.is_empty(), "no key held");
}

/// `get` and `contains_key` read what the key holds, and change nothing.
#[kani::proof]
#[kani::unwind(5)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
fn reading_a_vec_map_is_reading_an_ordered_map() {
    let (map, reference) = any_map();
    let (k, key) = any_key();
    assert!(map.get(&key).copied() == reference[k] && map.contains_key(&key) == reference[k].is_some());
    assert!(agrees(&map, &reference));
}

/// `get_mut` reaches the key's value exactly when it holds one, and a write
/// through it is the key's new value.
#[kani::proof]
#[kani::unwind(5)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
fn writing_through_get_mut_writes_the_keys_value() {
    let (mut map, mut reference) = any_map();
    let (k, key) = any_key();
    let value: u8 = kani::any();
    match map.get_mut(&key) {
        Some(v) => {
            assert!(reference[k].is_some());
            *v = value;
            reference[k] = Some(value);
        }
        None => {
            assert!(reference[k].is_none());
        }
    }
    assert!(agrees(&map, &reference));
}

/// `values_mut` reaches every value, each once, in key order.
#[kani::proof]
#[kani::unwind(5)]
#[kani::stub(alloc::alloc::Global::grow_impl_runtime, crate::proofs::cards::no_growing)]
fn values_mut_reaches_every_value_in_key_order() {
    let (mut map, mut reference) = any_map();
    let add: u8 = kani::any();
    for v in map.values_mut() {
        *v = v.wrapping_add(add);
    }
    for v in reference.iter_mut().flatten() {
        *v = v.wrapping_add(add);
    }
    assert!(agrees(&map, &reference));
}
