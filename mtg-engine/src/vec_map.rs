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
    /// Sorted by key, each key once; each value in an allocation of its
    /// own. CBMC models a heap allocation as bytes, and a reference read
    /// back from bytes (a list of `&GameObject`, say) is one whose offset
    /// it no longer knows: into a buffer holding every value, each read
    /// through it chose among every byte of every value, and resolving a
    /// card that lists a graveyard ran out of memory. Into a value of its
    /// own, it chooses among those values only, at known offsets; the
    /// same resolution proved in under seven minutes.
    entries: Vec<(K, Box<V>)>,
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

    // `get` and `get_mut` return the entry from inside the search rather
    // than its index from `find`: a reference taken at an index the solver
    // leaves open is one into the whole buffer at an offset it leaves open,
    // which it reads byte by byte across every entry, where one returned
    // from each step of the search is one of a few known places.

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        for (k, v) in &self.entries {
            match k.borrow().cmp(key) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => return Some(&**v),
                std::cmp::Ordering::Greater => return None,
            }
        }
        None
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        for (k, v) in &mut self.entries {
            match (*k).borrow().cmp(key) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => return Some(&mut **v),
                std::cmp::Ordering::Greater => return None,
            }
        }
        None
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
            Ok(i) => Some(std::mem::replace(&mut *self.entries[i].1, value)),
            Err(i) if i == self.entries.len() => {
                self.entries.push((key, Box::new(value)));
                None
            }
            Err(i) => {
                self.entries.insert(i, (key, Box::new(value)));
                None
            }
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| *self.entries.remove(i).1)
    }

    /// Room for `additional` more entries without reallocating, as
    /// `Vec::reserve`.
    pub fn reserve(&mut self, additional: usize) {
        self.entries.reserve(additional);
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, &**v))
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(k, _)| k)
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.entries.iter().map(|(_, v)| &**v)
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut V> {
        self.entries.iter_mut().map(|(_, v)| &mut **v)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// As `BTreeMap::entry`, for `or_insert`, the one use the engine has.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        Entry { map: self, key }
    }
}

/// A place in a [`VecMap`] for a key, there or not, as `BTreeMap`'s entry.
pub struct Entry<'a, K, V> {
    map: &'a mut VecMap<K, V>,
    key: K,
}

impl<'a, K: Ord, V> Entry<'a, K, V> {
    /// The key's value, `default` inserted first if there was none.
    pub fn or_insert(self, default: V) -> &'a mut V {
        let i = match self.map.find(&self.key) {
            Ok(i) => i,
            Err(i) => {
                self.map.entries.insert(i, (self.key, Box::new(default)));
                i
            }
        };
        &mut *self.map.entries[i].1
    }
}

impl<'a, K, V> IntoIterator for &'a VecMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = std::iter::Map<std::slice::Iter<'a, (K, Box<V>)>, fn(&'a (K, Box<V>)) -> (&'a K, &'a V)>;

    fn into_iter(self) -> Self::IntoIter {
        fn pair<K, V>(e: &(K, Box<V>)) -> (&K, &V) {
            (&e.0, &*e.1)
        }
        self.entries.iter().map(pair as fn(&'a (K, Box<V>)) -> (&'a K, &'a V))
    }
}
