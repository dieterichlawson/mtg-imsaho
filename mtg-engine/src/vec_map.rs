//! An ordered map kept as a vector in key order, which the proofs build the
//! engine with in place of `BTreeMap` and `HashMap`.
//!
//! CBMC models memory byte by byte. A `BTreeMap` leaf holds eleven values
//! and shifts them on insert, so inserting one `GameObject` (dozens of
//! fields, several of them heap-allocated) asked CBMC for more than 9 GB
//! and did not finish; a `HashMap` seeds its hasher from the operating
//! system, which Kani cannot model at all. A vector in key order answers
//! every question the engine asks of those maps the same way: the same
//! value for a key, and the same entries in the same (key) order. The
//! proofs in `proofs::vec_map` hold it to `BTreeMap` for every sequence of
//! operations within their bounds.
//!
//! Only `#[cfg(kani)]` builds use it; every other build keeps the standard
//! maps.

use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VecMap<K, V> {
    /// Sorted by key, each key once.
    entries: Vec<(K, V)>,
}

impl<K, V> Default for VecMap<K, V> {
    fn default() -> Self {
        Self { entries: Vec::new() }
    }
}

impl<K: Ord, V> VecMap<K, V> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Where `key` is, or where it would go to keep the keys in order.
    fn find<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        for (i, (k, _)) in self.entries.iter().enumerate() {
            match k.borrow().cmp(key) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => return Ok(i),
                std::cmp::Ordering::Greater => return Err(i),
            }
        }
        Err(self.entries.len())
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| &self.entries[i].1)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| &mut self.entries[i].1)
    }

    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).is_ok()
    }

    /// As `BTreeMap::insert`: the value it replaced, if the key was there.
    /// A key past every other, as the engine's ever-increasing object ids
    /// are, goes on the end without moving anything.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.find(&key) {
            Ok(i) => Some(std::mem::replace(&mut self.entries[i].1, value)),
            Err(i) if i == self.entries.len() => {
                self.entries.push((key, value));
                None
            }
            Err(i) => {
                self.entries.insert(i, (key, value));
                None
            }
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| self.entries.remove(i).1)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| v)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| v)
    }
}
