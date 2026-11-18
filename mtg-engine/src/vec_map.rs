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
            Err(i) => {
                self.insert_at(i, (key, Box::new(value)));
                None
            }
        }
    }

    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).ok().map(|i| *self.remove_at(i).1)
    }

    // `Vec::insert` and `Vec::remove` move the entries after `i` in one
    // copy, of a length that depends on where `i` is; CBMC encodes a copy of
    // a length it leaves open over the whole buffer, and adding one counter
    // to an object ran it out of memory. One entry moved one place at a
    // time is an exchange of two known places at each step.

    /// As `Vec::insert(i, entry)`.
    fn insert_at(&mut self, i: usize, entry: (K, Box<V>)) {
        self.entries.push(entry);
        let mut j = self.entries.len() - 1;
        while j > i {
            self.exchange(j - 1, j);
            j -= 1;
        }
    }

    /// As `Vec::remove(i)`.
    fn remove_at(&mut self, i: usize) -> (K, Box<V>) {
        let mut j = i;
        while j + 1 < self.entries.len() {
            self.exchange(j, j + 1);
            j += 1;
        }
        self.entries.pop().expect("an entry at i")
    }

    /// Exchange entries `a` and `b`, as `slice::swap`, each moved whole.
    /// `slice::swap` copies the two as bytes, eight at a time, and a proof
    /// that reads a key's string back from bytes copied so no longer knows
    /// which allocation it points into: every comparison of a key after an
    /// insertion out of order was then of any string, and every value read
    /// of any value (Divine Reckoning's kept creatures, read back from the
    /// card's state as a list of any length).
    fn exchange(&mut self, a: usize, b: usize) {
        assert!(a < b && b < self.entries.len());
        let base = self.entries.as_mut_ptr();
        // SAFETY: `a` and `b` are distinct indices in bounds of `entries`;
        // each value is read out once and written back once, into the
        // other's place, so none is dropped or duplicated.
        unsafe {
            let first = std::ptr::read(base.add(a));
            let second = std::ptr::read(base.add(b));
            std::ptr::write(base.add(a), second);
            std::ptr::write(base.add(b), first);
        }
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

impl<'a, K: Ord + Clone, V> Entry<'a, K, V> {
    /// The key's value, `default` inserted first if there was none. The
    /// value is reached through `get_mut`, for the reason `get_mut` gives,
    /// not at the index the search found.
    pub fn or_insert(self, default: V) -> &'a mut V {
        if let Err(i) = self.map.find(&self.key) {
            self.map.insert_at(i, (self.key.clone(), Box::new(default)));
        }
        self.map.get_mut(&self.key).expect("there, or just put there")
    }

    /// As `or_insert(V::default())`, as `BTreeMap`'s entry.
    pub fn or_default(self) -> &'a mut V
    where
        V: Default,
    {
        self.or_insert(V::default())
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

/// An ordered set kept as a vector in order, which the proofs build the
/// combat's sets with in place of `BTreeSet`, for the reason [`VecMap`]
/// stands in for `BTreeMap`.
///
/// The keys themselves, in order, and not a [`VecMap`] of nothing: a map's
/// entry boxes its value, and a box of nothing is a dangling pointer,
/// which Kani's checks of each pointer step (`same_allocation`) took
/// gigabytes over: marking one creature as having dealt first-strike
/// damage ran out of memory in 23,000 statements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VecSet<K> {
    keys: Vec<K>,
}

impl<K> Default for VecSet<K> {
    fn default() -> Self {
        Self { keys: Vec::new() }
    }
}

impl<K: Ord> VecSet<K> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Room for `additional` more keys without reallocating, as
    /// `Vec::reserve`.
    pub fn reserve(&mut self, additional: usize) {
        self.keys.reserve(additional);
    }

    /// Where `key` is, or where it would go, by a walk in order.
    fn find<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        for (i, k) in self.keys.iter().enumerate() {
            match k.borrow().cmp(key) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => return Ok(i),
                std::cmp::Ordering::Greater => return Err(i),
            }
        }
        Err(self.keys.len())
    }

    /// As `BTreeSet::insert`: whether the key was not there before.
    pub fn insert(&mut self, key: K) -> bool {
        match self.find(&key) {
            Ok(_) => false,
            Err(i) => {
                self.keys.insert(i, key);
                true
            }
        }
    }

    pub fn contains<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        self.find(key).is_ok()
    }

    /// As `BTreeSet::remove`: whether the key was there.
    pub fn remove<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Ord + ?Sized,
    {
        match self.find(key) {
            Ok(i) => {
                self.keys.remove(i);
                true
            }
            Err(_) => false,
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &K> {
        self.keys.iter()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
    }

    /// As `BTreeSet::is_subset`: whether every key here is in `other`.
    pub fn is_subset(&self, other: &Self) -> bool {
        self.keys.iter().all(|k| other.contains(k))
    }
}

impl<'a, K> IntoIterator for &'a VecSet<K> {
    type Item = &'a K;
    type IntoIter = std::slice::Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.keys.iter()
    }
}

/// `VecMap` and `VecSet` against `BTreeMap` and `BTreeSet`, under `cargo
/// test`. The Kani proofs in `proofs::vec_map` hold them to an ordered map
/// for one operation from any map of three keys; these play long random
/// sequences of every operation over more keys, so a change to the maps is
/// caught by every test run and not only by the proofs, which run in their
/// own job (and for the weekly mutation sweep, which runs only `cargo
/// test`).
#[cfg(test)]
mod tests {
    use super::{VecMap, VecSet};
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::collections::{BTreeMap, BTreeSet};

    /// Distinct keys the sequences draw from: enough that a map holds some
    /// and lacks others at every step, and that a key goes in before, between
    /// and after the ones there.
    const KEYS: u8 = 12;
    const SEEDS: u64 = 64;
    const STEPS: usize = 400;

    /// The map holds what the reference holds, read every way the map is.
    fn agrees(map: &VecMap<u8, String>, reference: &BTreeMap<u8, String>, what: &str) {
        let entries: Vec<(&u8, &String)> = map.iter().collect();
        let expected: Vec<(&u8, &String)> = reference.iter().collect();
        assert_eq!(entries, expected, "{what}: entries");
        assert_eq!(map.into_iter().collect::<Vec<_>>(), expected, "{what}: entries by `for`");
        assert_eq!(map.keys().collect::<Vec<_>>(), reference.keys().collect::<Vec<_>>(), "{what}: keys");
        assert_eq!(map.values().collect::<Vec<_>>(), reference.values().collect::<Vec<_>>(), "{what}: values");
        assert_eq!(map.len(), reference.len(), "{what}: len");
        assert_eq!(map.is_empty(), reference.is_empty(), "{what}: is_empty");
        for k in 0..KEYS {
            assert_eq!(map.get(&k), reference.get(&k), "{what}: get {k}");
            assert_eq!(map.contains_key(&k), reference.contains_key(&k), "{what}: contains_key {k}");
        }
    }

    /// Every operation, in random sequences, answers as `BTreeMap` does and
    /// leaves the map holding what `BTreeMap` holds. The values are strings,
    /// each its own allocation, so an entry moved by `exchange` and read
    /// twice or not at all is a double free or a value lost, not a quiet
    /// copy of bytes.
    #[test]
    fn a_vec_map_answers_every_operation_as_a_btree_map_does() {
        for seed in 0..SEEDS {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut map: VecMap<u8, String> = VecMap::new();
            let mut reference: BTreeMap<u8, String> = BTreeMap::new();
            for step in 0..STEPS {
                let k = rng.gen_range(0..KEYS);
                let v = format!("{seed}.{step}");
                let what = format!("seed {seed} step {step}");
                match rng.gen_range(0..10) {
                    0..=2 => assert_eq!(map.insert(k, v.clone()), reference.insert(k, v), "{what}: insert {k}"),
                    3 | 4 => assert_eq!(map.remove(&k), reference.remove(&k), "{what}: remove {k}"),
                    5 => {
                        let (got, want) = (map.get_mut(&k), reference.get_mut(&k));
                        assert_eq!(got.is_some(), want.is_some(), "{what}: get_mut {k}");
                        if let (Some(got), Some(want)) = (got, want) {
                            got.push('m');
                            want.push('m');
                        }
                    }
                    6 => {
                        map.entry(k).or_insert(v.clone()).push('e');
                        reference.entry(k).or_insert(v).push('e');
                    }
                    7 => {
                        map.entry(k).or_default().push('d');
                        reference.entry(k).or_default().push('d');
                    }
                    8 => {
                        // Each written through on its own: zipped, a
                        // `values_mut` that yields nothing writes to neither.
                        for v in map.values_mut() {
                            v.push('v');
                        }
                        for v in reference.values_mut() {
                            v.push('v');
                        }
                    }
                    _ if rng.gen_range(0..8) == 0 => {
                        map.clear();
                        reference.clear();
                    }
                    _ => map.reserve(usize::from(k)),
                }
                agrees(&map, &reference, &what);
            }
        }
    }

    /// Keys looked up by what they borrow as, as the engine looks up a card
    /// name held as a `String` by a `&str`.
    #[test]
    fn a_vec_map_of_strings_is_looked_up_by_str() {
        let mut map: VecMap<String, u32> = VecMap::new();
        let mut reference: BTreeMap<String, u32> = BTreeMap::new();
        for (n, name) in ["Moon Heron", "Forest", "Brimstone Volley", "Swamp", "Forest", "Abattoir Ghoul"].into_iter().enumerate() {
            let n = u32::try_from(n).expect("a few");
            assert_eq!(map.insert(name.to_string(), n), reference.insert(name.to_string(), n), "insert {name}");
        }
        for name in ["Abattoir Ghoul", "Forest", "Island", "Moon Heron", "Zombie"] {
            assert_eq!(map.get(name), reference.get(name), "get {name}");
            assert_eq!(map.contains_key(name), reference.contains_key(name), "contains_key {name}");
        }
        if let Some(n) = map.get_mut("Swamp") {
            *n += 10;
        }
        *reference.get_mut("Swamp").expect("there") += 10;
        assert_eq!(map.remove("Forest"), reference.remove("Forest"));
        assert_eq!(map.remove("Island"), reference.remove("Island"));
        assert_eq!(map.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>(),
            reference.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>());
    }

    /// Every operation of `VecSet`, in random sequences, answers as
    /// `BTreeSet` does and leaves the set holding what it holds; `is_subset`
    /// against a second set built alongside.
    #[test]
    fn a_vec_set_answers_every_operation_as_a_btree_set_does() {
        for seed in 0..SEEDS {
            let mut rng = StdRng::seed_from_u64(seed);
            let (mut set, mut other) = (VecSet::new(), VecSet::new());
            let (mut reference, mut other_reference) = (BTreeSet::new(), BTreeSet::new());
            for step in 0..STEPS {
                let k = rng.gen_range(0..KEYS);
                let what = format!("seed {seed} step {step}");
                match rng.gen_range(0..8) {
                    0..=2 => assert_eq!(set.insert(k), reference.insert(k), "{what}: insert {k}"),
                    3 | 4 => assert_eq!(set.remove(&k), reference.remove(&k), "{what}: remove {k}"),
                    5 => assert_eq!(other.insert(k), other_reference.insert(k), "{what}: insert {k} into the other"),
                    6 => assert_eq!(other.remove(&k), other_reference.remove(&k), "{what}: remove {k} from the other"),
                    _ if rng.gen_range(0..8) == 0 => {
                        set.clear();
                        reference.clear();
                    }
                    _ => set.reserve(usize::from(k)),
                }
                assert_eq!(set.iter().collect::<Vec<_>>(), reference.iter().collect::<Vec<_>>(), "{what}: keys");
                assert_eq!((&set).into_iter().collect::<Vec<_>>(), reference.iter().collect::<Vec<_>>(), "{what}: keys by `for`");
                assert_eq!((set.len(), set.is_empty()), (reference.len(), reference.is_empty()), "{what}: len");
                for k in 0..KEYS {
                    assert_eq!(set.contains(&k), reference.contains(&k), "{what}: contains {k}");
                }
                assert_eq!(set.is_subset(&other), reference.is_subset(&other_reference), "{what}: is_subset");
                assert_eq!(other.is_subset(&set), other_reference.is_subset(&reference), "{what}: is_subset the other way");
            }
        }
    }
}
