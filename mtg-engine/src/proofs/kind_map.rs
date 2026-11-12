//! `KindMap`, which a Kani build keeps an object's counters and a player's
//! mana pool in, is an ordered map keyed by kind: what
//! `BTreeMap<CounterType, u32>` and `BTreeMap<ManaType, u32>` are
//! documented to be.
//!
//! The same induction as `proofs::vec_map`'s, over the six kinds there are,
//! for each of the two keys. The reference is an array indexed by kind, in
//! the order the key type sorts them; each proof starts from any map
//! holding any reference's entries (every subset of the kinds, any values)
//! and applies one operation on any kind: an insert, a remove, a lookup, a
//! write through `get_mut` or `entry(..).or_insert(..)`, a `clear`. The
//! operation returns what the reference says, and the map holds the changed
//! reference's entries afterwards, as `iter`, iteration of `&map`, `keys`,
//! `values`, `len` and `is_empty` read them.

use crate::kind_map::{Kind, KindMap};
use crate::types::{CounterType, ManaType};

/// The kinds in the order their type sorts them, read off the type's own
/// `Ord`: `Kind::ALL` is what the map iterates in, so it is checked here
/// against the order a `BTreeMap` would use.
fn sorted<K: Kind + Ord>() -> bool {
    K::ALL.windows(2).all(|w| w[0] < w[1]) && K::ALL.iter().enumerate().all(|(i, k)| k.slot() == i)
}

fn agrees<K: Kind>(map: &KindMap<K>, reference: &[Option<u32>; 6]) -> bool {
    // What the reference holds, in kind order, each entry at a known place.
    let mut expected: [Option<(K, u32)>; 6] = [None; 6];
    let mut held = 0;
    for (i, kind) in K::ALL.iter().enumerate() {
        if let Some(n) = reference[i] {
            expected[held] = Some((*kind, n));
            held += 1;
        }
    }
    let same = |(got, n): ([Option<(K, u32)>; 6], usize)| n == held && got == expected;
    map.len() == held
        && map.is_empty() == (held == 0)
        && same(taken(map.iter().map(|(k, n)| (*k, *n))))
        && same(taken(map.into_iter().map(|(k, n)| (*k, *n))))
        && same(taken(map.keys().zip(map.values()).map(|(k, n)| (*k, *n))))
}

/// The entries an iteration hands out, each at the place it lands, and how
/// many (at most seven taken, so one too many shows). Compared so, rather
/// than walked beside another iterator, each entry is read at a known
/// place: two iterators that each skip to their next entry stand, after the
/// first, at places of every value, and walking one beside the other ran
/// these proofs out of memory.
fn taken<K: Kind>(it: impl Iterator<Item = (K, u32)>) -> ([Option<(K, u32)>; 6], usize) {
    let mut got = [None; 6];
    let mut n = 0;
    for entry in it.take(7) {
        if n < 6 {
            got[n] = Some(entry);
        }
        n += 1;
    }
    (got, n)
}

/// Any map holding any reference's entries.
fn any_map<K: Kind>() -> (KindMap<K>, [Option<u32>; 6]) {
    let mut reference: [Option<u32>; 6] = [None; 6];
    let mut map = KindMap::new();
    for (kind, slot) in K::ALL.iter().zip(reference.iter_mut()) {
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
fn any_kind<K: Kind>() -> (usize, K) {
    let i: usize = kani::any();
    kani::assume(i < K::ALL.len());
    (i, K::ALL[i])
}

/// `insert` returns the value the kind held, and the kind then holds the
/// new one.
fn inserting<K: Kind>() {
    let (mut map, mut reference) = any_map::<K>();
    let (i, kind) = any_kind::<K>();
    let n: u32 = kani::any();
    assert!(map.insert(kind, n) == reference[i].replace(n));
    assert!(agrees(&map, &reference));
}

/// `remove` returns the value the kind held, and the kind then holds none.
fn removing<K: Kind>() {
    let (mut map, mut reference) = any_map::<K>();
    let (i, kind) = any_kind::<K>();
    assert!(map.remove(&kind) == reference[i].take());
    assert!(agrees(&map, &reference));
}

/// `get` and `contains_key` read what the kind holds, and change nothing.
fn reading<K: Kind>() {
    let (map, reference) = any_map::<K>();
    let (i, kind) = any_kind::<K>();
    assert!(map.get(&kind).copied() == reference[i] && map.contains_key(&kind) == reference[i].is_some());
    assert!(agrees(&map, &reference));
}

/// `get_mut` reaches the kind's value exactly when it holds one, and a
/// write through it is the kind's new value.
fn writing_through_get_mut<K: Kind>() {
    let (mut map, mut reference) = any_map::<K>();
    let (i, kind) = any_kind::<K>();
    let n: u32 = kani::any();
    match map.get_mut(&kind) {
        Some(c) => {
            assert!(reference[i].is_some());
            *c = n;
            reference[i] = Some(n);
        }
        None => {
            assert!(reference[i].is_none());
        }
    }
    assert!(agrees(&map, &reference));
}

/// `entry(kind).or_insert(default)` reaches the kind's value, `default`
/// put there first when it held none, and a write through it is the kind's
/// new value.
fn or_insert<K: Kind>() {
    let (mut map, mut reference) = any_map::<K>();
    let (i, kind) = any_kind::<K>();
    let (default, n): (u32, u32) = (kani::any(), kani::any());
    let c = map.entry(kind).or_insert(default);
    assert!(*c == reference[i].unwrap_or(default));
    *c = n;
    reference[i] = Some(n);
    assert!(agrees(&map, &reference));
}

/// `clear` leaves no kind holding a value.
fn clearing<K: Kind>() {
    let (mut map, _) = any_map::<K>();
    map.clear();
    assert!(agrees(&map, &[None; 6]));
}

/// Each operation's proof, for each key a `KindMap` is kept for.
macro_rules! for_each_key {
    ($($name:ident: $op:ident::<$key:ty>),* $(,)?) => {$(
        #[kani::proof]
        #[kani::unwind(8)]
        fn $name() {
            $op::<$key>();
        }
    )*};
}

for_each_key! {
    inserting_counters_is_inserting_into_an_ordered_map: inserting::<CounterType>,
    removing_counters_is_removing_from_an_ordered_map: removing::<CounterType>,
    reading_counters_is_reading_an_ordered_map: reading::<CounterType>,
    writing_counters_through_get_mut_writes_that_kind: writing_through_get_mut::<CounterType>,
    or_insert_reaches_the_kinds_count_inserting_the_default_first: or_insert::<CounterType>,
    clearing_counters_leaves_none: clearing::<CounterType>,
    inserting_mana_is_inserting_into_an_ordered_map: inserting::<ManaType>,
    removing_mana_is_removing_from_an_ordered_map: removing::<ManaType>,
    reading_mana_is_reading_an_ordered_map: reading::<ManaType>,
    writing_mana_through_get_mut_writes_that_type: writing_through_get_mut::<ManaType>,
    or_insert_reaches_the_types_mana_inserting_the_default_first: or_insert::<ManaType>,
    clearing_mana_leaves_none: clearing::<ManaType>,
}

#[kani::proof]
/// Each key's kinds are listed in the order the key sorts them, each at its
/// own place: the order a map keyed by it iterates in.
fn the_kinds_are_in_their_types_order() {
    assert!(sorted::<CounterType>() && sorted::<ManaType>());
}
