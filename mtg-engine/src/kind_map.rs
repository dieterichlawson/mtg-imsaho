//! The counters on an object, and the mana in a pool, as a Kani build keeps
//! them: one slot per kind, in the place of a `BTreeMap` keyed by the kind.
//!
//! `vec_map` says why a Kani build keeps no `BTreeMap`. A `VecMap` serves
//! for the objects, whose keys arrive in order, but counters of two kinds
//! the solver leaves open sit in an order it leaves open, and putting one
//! in writes at an offset it leaves open; CBMC then follows the buffer as a
//! whole rather than field by field, and adding a counter ran it out of
//! memory. A mana pool's six inserts ran past five minutes. The kinds of
//! counter and of mana are six each and known, so each kind has a slot of
//! its own and every access is to a slot CBMC knows. `proofs::kind_map`
//! holds it to an ordered map keyed by kind, which `BTreeMap` is documented
//! to be.

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::types::{CounterType, ManaType};

/// A key a [`KindMap`] holds a slot for: one of six, each with its place.
pub trait Kind: Copy + Eq + 'static {
    /// Every kind, in the order the type sorts them.
    const ALL: &'static [Self; 6];

    /// Its place in [`Kind::ALL`].
    fn slot(self) -> usize;
}

impl Kind for CounterType {
    const ALL: &'static [Self; 6] = &[
        CounterType::PlusOnePlusOne,
        CounterType::MinusOneMinusOne,
        CounterType::Loyalty,
        CounterType::Slime,
        CounterType::Study,
        CounterType::Hatchling,
    ];

    fn slot(self) -> usize {
        match self {
            CounterType::PlusOnePlusOne => 0,
            CounterType::MinusOneMinusOne => 1,
            CounterType::Loyalty => 2,
            CounterType::Slime => 3,
            CounterType::Study => 4,
            CounterType::Hatchling => 5,
        }
    }
}

impl Kind for ManaType {
    const ALL: &'static [Self; 6] = &[
        ManaType::White,
        ManaType::Blue,
        ManaType::Black,
        ManaType::Red,
        ManaType::Green,
        ManaType::Colorless,
    ];

    fn slot(self) -> usize {
        match self {
            ManaType::White => 0,
            ManaType::Blue => 1,
            ManaType::Black => 2,
            ManaType::Red => 3,
            ManaType::Green => 4,
            ManaType::Colorless => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KindMap<K> {
    slots: [Option<u32>; 6],
    #[serde(skip)]
    kind: PhantomData<K>,
}

impl<K> Default for KindMap<K> {
    fn default() -> Self {
        Self { slots: [None; 6], kind: PhantomData }
    }
}

impl<K: Kind> KindMap<K> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, kind: &K) -> Option<&u32> {
        self.slots[kind.slot()].as_ref()
    }

    pub fn get_mut(&mut self, kind: &K) -> Option<&mut u32> {
        self.slots[kind.slot()].as_mut()
    }

    pub fn contains_key(&self, kind: &K) -> bool {
        self.slots[kind.slot()].is_some()
    }

    /// As `BTreeMap::insert`: the value it replaced, if the kind was there.
    pub fn insert(&mut self, kind: K, n: u32) -> Option<u32> {
        self.slots[kind.slot()].replace(n)
    }

    pub fn remove(&mut self, kind: &K) -> Option<u32> {
        self.slots[kind.slot()].take()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|n| n.is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The kinds held, with their values, in kind order.
    pub fn iter(&self) -> Iter<'_, K> {
        Iter { map: self, next: 0 }
    }

    /// The kinds held, in kind order.
    pub fn keys(&self) -> impl Iterator<Item = &K> + '_ {
        self.iter().map(|(k, _)| k)
    }

    /// The values held, in kind order.
    pub fn values(&self) -> impl Iterator<Item = &u32> + '_ {
        self.iter().map(|(_, n)| n)
    }

    /// As `BTreeMap::entry`, for `or_insert`, the one use the engine has.
    pub fn entry(&mut self, kind: K) -> Entry<'_> {
        Entry(&mut self.slots[kind.slot()])
    }
}

/// A kind's place in a [`KindMap`], held or not.
pub struct Entry<'a>(&'a mut Option<u32>);

impl<'a> Entry<'a> {
    /// The kind's value, `default` put there first if it held none.
    pub fn or_insert(self, default: u32) -> &'a mut u32 {
        self.0.get_or_insert(default)
    }
}

/// The kinds a [`KindMap`] holds, with their values, in kind order.
pub struct Iter<'a, K> {
    map: &'a KindMap<K>,
    next: usize,
}

impl<'a, K: Kind> Iterator for Iter<'a, K> {
    type Item = (&'a K, &'a u32);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(kind) = K::ALL.get(self.next) {
            self.next += 1;
            if let Some(n) = &self.map.slots[kind.slot()] {
                return Some((kind, n));
            }
        }
        None
    }
}

impl<'a, K: Kind> IntoIterator for &'a KindMap<K> {
    type Item = (&'a K, &'a u32);
    type IntoIter = Iter<'a, K>;

    fn into_iter(self) -> Iter<'a, K> {
        self.iter()
    }
}

/// `KindMap` against `BTreeMap`, under `cargo test`, as `vec_map`'s tests
/// hold `VecMap` to it: `proofs::kind_map` proves one operation from any
/// map, and these play long random sequences, so a change is caught by
/// every test run (and by the weekly mutation sweep, which runs only `cargo
/// test`).
#[cfg(test)]
mod tests {
    use super::{Kind, KindMap};
    use crate::types::{CounterType, ManaType};
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::collections::BTreeMap;

    /// A kind's slots are its kinds in the order the type sorts them, each
    /// in the place `ALL` gives it, so the map's order is `BTreeMap`'s.
    fn slots_are_in_sorted_order<K: Kind + Ord + std::fmt::Debug>() {
        assert!(K::ALL.windows(2).all(|w| w[0] < w[1]), "ALL is in the order the type sorts: {:?}", K::ALL);
        for (i, kind) in K::ALL.iter().enumerate() {
            assert_eq!(kind.slot(), i, "{kind:?} is in slot {}, not {i}", kind.slot());
        }
    }

    #[test]
    fn each_kind_has_the_slot_of_its_place_in_sorted_order() {
        slots_are_in_sorted_order::<CounterType>();
        slots_are_in_sorted_order::<ManaType>();
    }

    fn agrees<K: Kind + Ord + std::fmt::Debug>(map: &KindMap<K>, reference: &BTreeMap<K, u32>, what: &str) {
        let expected: Vec<(&K, &u32)> = reference.iter().collect();
        assert_eq!(map.iter().collect::<Vec<_>>(), expected, "{what}: entries");
        assert_eq!(map.into_iter().collect::<Vec<_>>(), expected, "{what}: entries by `for`");
        assert_eq!(map.keys().collect::<Vec<_>>(), reference.keys().collect::<Vec<_>>(), "{what}: keys");
        assert_eq!(map.values().collect::<Vec<_>>(), reference.values().collect::<Vec<_>>(), "{what}: values");
        assert_eq!((map.len(), map.is_empty()), (reference.len(), reference.is_empty()), "{what}: len");
        for kind in K::ALL {
            assert_eq!(map.get(kind), reference.get(kind), "{what}: get {kind:?}");
            assert_eq!(map.contains_key(kind), reference.contains_key(kind), "{what}: contains_key {kind:?}");
        }
    }

    /// Every operation, in random sequences, answers as `BTreeMap` does and
    /// leaves the map holding what `BTreeMap` holds.
    fn answers_as_a_btree_map<K: Kind + Ord + std::fmt::Debug>() {
        for seed in 0..64 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut map: KindMap<K> = KindMap::new();
            let mut reference: BTreeMap<K, u32> = BTreeMap::new();
            for step in 0..300 {
                let kind = K::ALL[rng.gen_range(0..K::ALL.len())];
                let n: u32 = rng.gen_range(0..5);
                let what = format!("seed {seed} step {step}");
                match rng.gen_range(0..7) {
                    0 | 1 => assert_eq!(map.insert(kind, n), reference.insert(kind, n), "{what}: insert {kind:?}"),
                    2 => assert_eq!(map.remove(&kind), reference.remove(&kind), "{what}: remove {kind:?}"),
                    3 => {
                        let (got, want) = (map.get_mut(&kind), reference.get_mut(&kind));
                        assert_eq!(got.is_some(), want.is_some(), "{what}: get_mut {kind:?}");
                        if let (Some(got), Some(want)) = (got, want) {
                            *got += n;
                            *want += n;
                        }
                    }
                    4 | 5 => {
                        *map.entry(kind).or_insert(n) += 1;
                        *reference.entry(kind).or_insert(n) += 1;
                    }
                    _ if rng.gen_range(0..6) == 0 => {
                        map.clear();
                        reference.clear();
                    }
                    _ => {}
                }
                agrees(&map, &reference, &what);
            }
        }
    }

    #[test]
    fn a_kind_map_of_counters_answers_every_operation_as_a_btree_map_does() {
        answers_as_a_btree_map::<CounterType>();
    }

    #[test]
    fn a_kind_map_of_mana_answers_every_operation_as_a_btree_map_does() {
        answers_as_a_btree_map::<ManaType>();
    }
}
