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
