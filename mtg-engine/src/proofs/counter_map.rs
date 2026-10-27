//! `CounterMap`, which a Kani build keeps an object's counters in, is an
//! ordered map keyed by counter kind: what `BTreeMap<CounterType, u32>` is
//! documented to be.
//!
//! The same induction as `proofs::vec_map`'s, over the six kinds there are.
//! The reference is an array indexed by kind, in the order `CounterType`
//! sorts them; each proof starts from any map holding any reference's
//! entries (every subset of the kinds, any counts) and applies one
//! operation on any kind: an insert, a remove, a lookup, a write through
//! `get_mut` or `entry(..).or_insert(..)`, a `clear`. The operation returns
//! what the reference says, and the map holds the changed reference's
//! entries afterwards, as `iter`, iteration of `&map`, `len` and
//! `is_empty` read them.

use crate::counter_map::CounterMap;
use crate::types::CounterType;

/// Every kind, in the order `CounterType` sorts them.
const KINDS: [CounterType; 6] = [
    CounterType::PlusOnePlusOne,
    CounterType::MinusOneMinusOne,
    CounterType::Loyalty,
    CounterType::Slime,
    CounterType::Study,
    CounterType::Hatchling,
];

fn agrees(map: &CounterMap, reference: &[Option<u32>; 6]) -> bool {
    let held = reference.iter().filter(|n| n.is_some()).count();
    let mut expected = KINDS.iter().zip(reference).filter_map(|(k, n)| n.map(|n| (*k, n)));
    let mut by_ref = expected.clone();
    map.len() == held
        && map.is_empty() == (held == 0)
        && map.iter().all(|(k, n)| expected.next() == Some((*k, *n)))
        && expected.next().is_none()
        && map.into_iter().all(|(k, n)| by_ref.next() == Some((*k, *n)))
        && by_ref.next().is_none()
}

/// Any map holding any reference's entries.
fn any_map() -> (CounterMap, [Option<u32>; 6]) {
    let mut reference: [Option<u32>; 6] = [None; 6];
    let mut map = CounterMap::new();
    for (kind, slot) in KINDS.iter().zip(reference.iter_mut()) {
        if kani::any() {
            let n: u32 = kani::any();
            *slot = Some(n);
            map.insert(*kind, n);
        }
    }
    assert!(agrees(&map, &reference));
    (map, reference)
}

/// Any kind: its place in the reference, and itself.
fn any_kind() -> (usize, CounterType) {
    let i: usize = kani::any();
    kani::assume(i < KINDS.len());
    (i, KINDS[i])
}

#[kani::proof]
#[kani::unwind(8)]
/// `insert` returns the count the kind held, and the kind then holds the
/// new one.
fn inserting_counters_is_inserting_into_an_ordered_map() {
    let (mut map, mut reference) = any_map();
    let (i, kind) = any_kind();
    let n: u32 = kani::any();
    assert!(map.insert(kind, n) == reference[i].replace(n));
    assert!(agrees(&map, &reference));
}

#[kani::proof]
#[kani::unwind(8)]
/// `remove` returns the count the kind held, and the kind then holds none.
fn removing_counters_is_removing_from_an_ordered_map() {
    let (mut map, mut reference) = any_map();
    let (i, kind) = any_kind();
    assert!(map.remove(&kind) == reference[i].take());
    assert!(agrees(&map, &reference));
}

#[kani::proof]
#[kani::unwind(8)]
/// `get` and `contains_key` read what the kind holds, and change nothing.
fn reading_counters_is_reading_an_ordered_map() {
    let (map, reference) = any_map();
    let (i, kind) = any_kind();
    assert!(map.get(&kind).copied() == reference[i] && map.contains_key(&kind) == reference[i].is_some());
    assert!(agrees(&map, &reference));
}

#[kani::proof]
#[kani::unwind(8)]
/// `get_mut` reaches the kind's count exactly when it holds one, and a
/// write through it is the kind's new count.
fn writing_counters_through_get_mut_writes_that_kind() {
    let (mut map, mut reference) = any_map();
    let (i, kind) = any_kind();
    let n: u32 = kani::any();
    match map.get_mut(&kind) {
        Some(c) => {
            assert!(reference[i].is_some());
            *c = n;
            reference[i] = Some(n);
        }
        None => assert!(reference[i].is_none()),
    }
    assert!(agrees(&map, &reference));
}

#[kani::proof]
#[kani::unwind(8)]
/// `entry(kind).or_insert(default)` reaches the kind's count, `default`
/// put there first when it held none, and a write through it is the kind's
/// new count.
fn or_insert_reaches_the_kinds_count_inserting_the_default_first() {
    let (mut map, mut reference) = any_map();
    let (i, kind) = any_kind();
    let (default, n): (u32, u32) = (kani::any(), kani::any());
    let c = map.entry(kind).or_insert(default);
    assert!(*c == reference[i].unwrap_or(default));
    *c = n;
    reference[i] = Some(n);
    assert!(agrees(&map, &reference));
}

#[kani::proof]
#[kani::unwind(8)]
/// `clear` leaves no kind holding a count.
fn clearing_counters_leaves_none() {
    let (mut map, _) = any_map();
    map.clear();
    assert!(agrees(&map, &[None; 6]));
}
