//! The counters on an object, as a Kani build keeps them: one field per
//! kind of counter, in the place of a `BTreeMap<CounterType, u32>`.
//!
//! `vec_map` says why a Kani build keeps no `BTreeMap`. A `VecMap` serves
//! for the objects, whose keys arrive in order, but counters of two kinds
//! the solver leaves open sit in an order it leaves open, and putting one
//! in writes at an offset it leaves open; CBMC then follows the buffer as a
//! whole rather than field by field, and adding a counter ran it out of
//! memory. The kinds are six and known, so each has a field of its own and
//! every access is to a field CBMC knows. `proofs::counter_map` holds it to
//! an ordered map keyed by counter kind, which `BTreeMap` is documented to
//! be.

use serde::{Deserialize, Serialize};

use crate::types::CounterType;

/// Every kind of counter, in the order `CounterType` sorts them.
static KINDS: [CounterType; 6] = [
    CounterType::PlusOnePlusOne,
    CounterType::MinusOneMinusOne,
    CounterType::Loyalty,
    CounterType::Slime,
    CounterType::Study,
    CounterType::Hatchling,
];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CounterMap {
    plus_one_plus_one: Option<u32>,
    minus_one_minus_one: Option<u32>,
    loyalty: Option<u32>,
    slime: Option<u32>,
    study: Option<u32>,
    hatchling: Option<u32>,
}

impl CounterMap {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn slot(&self, kind: CounterType) -> &Option<u32> {
        match kind {
            CounterType::PlusOnePlusOne => &self.plus_one_plus_one,
            CounterType::MinusOneMinusOne => &self.minus_one_minus_one,
            CounterType::Loyalty => &self.loyalty,
            CounterType::Slime => &self.slime,
            CounterType::Study => &self.study,
            CounterType::Hatchling => &self.hatchling,
        }
    }

    fn slot_mut(&mut self, kind: CounterType) -> &mut Option<u32> {
        match kind {
            CounterType::PlusOnePlusOne => &mut self.plus_one_plus_one,
            CounterType::MinusOneMinusOne => &mut self.minus_one_minus_one,
            CounterType::Loyalty => &mut self.loyalty,
            CounterType::Slime => &mut self.slime,
            CounterType::Study => &mut self.study,
            CounterType::Hatchling => &mut self.hatchling,
        }
    }

    pub fn get(&self, kind: &CounterType) -> Option<&u32> {
        self.slot(*kind).as_ref()
    }

    pub fn get_mut(&mut self, kind: &CounterType) -> Option<&mut u32> {
        self.slot_mut(*kind).as_mut()
    }

    pub fn contains_key(&self, kind: &CounterType) -> bool {
        self.slot(*kind).is_some()
    }

    /// As `BTreeMap::insert`: the value it replaced, if the kind was there.
    pub fn insert(&mut self, kind: CounterType, n: u32) -> Option<u32> {
        self.slot_mut(kind).replace(n)
    }

    pub fn remove(&mut self, kind: &CounterType) -> Option<u32> {
        self.slot_mut(*kind).take()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn len(&self) -> usize {
        KINDS.iter().filter(|k| self.slot(**k).is_some()).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The kinds held, with their counts, in kind order.
    pub fn iter(&self) -> Iter<'_> {
        Iter { map: self, next: 0 }
    }

    /// As `BTreeMap::entry`, for `or_insert`, the one use the engine has.
    pub fn entry(&mut self, kind: CounterType) -> Entry<'_> {
        Entry(self.slot_mut(kind))
    }
}

/// A kind's place in a [`CounterMap`], held or not.
pub struct Entry<'a>(&'a mut Option<u32>);

impl<'a> Entry<'a> {
    /// The kind's count, `default` put there first if it held none.
    pub fn or_insert(self, default: u32) -> &'a mut u32 {
        self.0.get_or_insert(default)
    }
}

/// The kinds a [`CounterMap`] holds, with their counts, in kind order.
pub struct Iter<'a> {
    map: &'a CounterMap,
    next: usize,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a CounterType, &'a u32);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(kind) = KINDS.get(self.next) {
            self.next += 1;
            if let Some(n) = self.map.slot(*kind) {
                return Some((kind, n));
            }
        }
        None
    }
}

impl<'a> IntoIterator for &'a CounterMap {
    type Item = (&'a CounterType, &'a u32);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}
