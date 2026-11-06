//! Whether a cost is within reach of a pool and the mana sources beside it,
//! by counting alone: the question the engine's auto-pass gate asks before
//! passing a seat by (`mtg-engine`'s `mana::within_reach`). It may say yes
//! to a cost no plan of taps pays; it must never say no to one that some
//! plan does, or a seat is passed by a spell it could have cast. That is
//! proved here for every pool, every board of sources and every plan
//! ([`reach_is_necessary`]).
//!
//! The count ([`in_reach`]): what the pool holds and what each source's
//! best ability nets (what it adds less what it costs, never below none),
//! in all, covers all the cost asks; and of each type, what the pool holds
//! and the most of that type any one ability of each source adds covers
//! what the cost asks of that type. Each source counts once, as each is
//! tapped once, for one of its abilities.
//!
//! A plan ([`runs`]) taps sources one after another, no source twice, each
//! for one of its abilities: the ability's own cost is paid out of the pool
//! as it stands, in any way a payment may leave it ([`paid_exactly`]), and
//! what the ability adds is added. A plan pays the cost when the pool it
//! leaves can pay it ([`payable`]).

use vstd::prelude::*;
// A glob: the specifications named from `mana` (`sum`, `paid_exactly`,
// `payable`) are erased from an ordinary build, and a list naming them
// would not resolve there.
#[allow(unused_imports, clippy::wildcard_imports)]
use crate::mana::*;

verus! {

/// One mana ability, as the count and a plan see it: the mana cost it asks
/// beyond its {T}, and what it adds of each type.
#[derive(Clone, Copy)]
pub struct Ability {
    pub cost: Pips,
    pub adds: Amounts,
}

/// All a cost asks, in mana.
pub open spec fn asks(cost: Pips) -> int {
    sum(cost.specific) + cost.generic
}

/// What an ability nets: all it adds less all its cost asks, and none if
/// that is less than none.
pub open spec fn net(a: Ability) -> int {
    if sum(a.adds) > asks(a.cost) { sum(a.adds) - asks(a.cost) } else { 0 }
}

/// The best the first `j` of a source's abilities net; none for none.
pub open spec fn best_net(abs: Seq<Ability>, j: int) -> int
    decreases j,
{
    if j <= 0 {
        0
    } else {
        let r = best_net(abs, j - 1);
        if net(abs[j - 1]) > r { net(abs[j - 1]) } else { r }
    }
}

/// The most of type `slot` any of the first `j` of a source's abilities
/// adds; none for none.
pub open spec fn best_adds(abs: Seq<Ability>, slot: int, j: int) -> int
    decreases j,
{
    if j <= 0 {
        0
    } else {
        let r = best_adds(abs, slot, j - 1);
        if abs[j - 1].adds[slot] > r { abs[j - 1].adds[slot] as int } else { r }
    }
}

/// The best nets of the first `n` sources, each once.
pub open spec fn total_net(srcs: Seq<Vec<Ability>>, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { total_net(srcs, n - 1) + best_net(srcs[n - 1]@, srcs[n - 1]@.len() as int) }
}

/// The most of type `slot` each of the first `n` sources adds, in all.
pub open spec fn total_adds(srcs: Seq<Vec<Ability>>, slot: int, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { total_adds(srcs, slot, n - 1) + best_adds(srcs[n - 1]@, slot, srcs[n - 1]@.len() as int) }
}

/// The count: the cost is within reach of the pool and the sources.
pub open spec fn in_reach(cost: Pips, pool: Amounts, srcs: Seq<Vec<Ability>>) -> bool {
    &&& sum(pool) + total_net(srcs, srcs.len() as int) >= asks(cost)
    &&& forall|i: int| 0 <= i < 6 ==> pool[i] + #[trigger] total_adds(srcs, i, srcs.len() as int) >= cost.specific[i]
}

/// One tap of a plan: source `tap.0` for its ability `tap.1`, from the pool
/// `before` to the pool `after`: the ability's cost paid out of `before` as
/// a payment may pay it, then what the ability adds added.
pub open spec fn steps(srcs: Seq<Vec<Ability>>, tap: (int, int), before: Amounts, after: Amounts) -> bool {
    &&& 0 <= tap.0 < srcs.len()
    &&& 0 <= tap.1 < srcs[tap.0]@.len()
    &&& exists|left: Amounts| #[trigger] paid_exactly(before, left, srcs[tap.0]@[tap.1].cost)
        && forall|i: int| 0 <= i < 6 ==> after[i] == left[i] + srcs[tap.0]@[tap.1].adds[i]
}

/// `pools` is a run of the plan `taps` from `pool`: `pools[k]` the pool
/// before the `k`th tap and the last the pool it leaves, no source tapped
/// twice.
pub open spec fn runs(pool: Amounts, srcs: Seq<Vec<Ability>>, taps: Seq<(int, int)>, pools: Seq<Amounts>) -> bool {
    &&& pools.len() == taps.len() + 1
    &&& pools[0] == pool
    &&& forall|k: int| 0 <= k < taps.len() ==> #[trigger] steps(srcs, taps[k], pools[k], pools[k + 1])
    &&& forall|j: int, k: int| 0 <= j < k < taps.len() ==> taps[j].0 != taps[k].0
}

/// Whether source `s` is one the first `k` taps of `taps` tap.
pub open spec fn tapped(taps: Seq<(int, int)>, k: int) -> spec_fn(int) -> bool {
    |s: int| exists|j: int| 0 <= j < k && taps[j].0 == s
}

/// `used`, and `s` besides.
pub open spec fn with(used: spec_fn(int) -> bool, s: int) -> spec_fn(int) -> bool {
    |x: int| used(x) || x == s
}

/// The best nets of the first `n` sources that are in `used`.
pub open spec fn used_net(srcs: Seq<Vec<Ability>>, used: spec_fn(int) -> bool, n: int) -> int
    decreases n,
{
    if n <= 0 {
        0
    } else {
        used_net(srcs, used, n - 1) + if used(n - 1) { best_net(srcs[n - 1]@, srcs[n - 1]@.len() as int) } else { 0 }
    }
}

/// The most of type `slot` each of the first `n` sources in `used` adds.
pub open spec fn used_adds(srcs: Seq<Vec<Ability>>, slot: int, used: spec_fn(int) -> bool, n: int) -> int
    decreases n,
{
    if n <= 0 {
        0
    } else {
        used_adds(srcs, slot, used, n - 1) + if used(n - 1) { best_adds(srcs[n - 1]@, slot, srcs[n - 1]@.len() as int) } else { 0 }
    }
}

proof fn lemma_best_net(abs: Seq<Ability>, a: int, j: int)
    requires 0 <= a < j,
    ensures best_net(abs, j) >= net(abs[a]), best_net(abs, j) >= 0,
    decreases j,
{
    if a < j - 1 {
        lemma_best_net(abs, a, j - 1);
    } else {
        lemma_best_net_nonneg(abs, j - 1);
    }
}

proof fn lemma_best_net_nonneg(abs: Seq<Ability>, j: int)
    ensures best_net(abs, j) >= 0,
    decreases j,
{
    if j > 0 {
        lemma_best_net_nonneg(abs, j - 1);
    }
}

proof fn lemma_best_adds(abs: Seq<Ability>, slot: int, a: int, j: int)
    requires 0 <= a < j,
    ensures best_adds(abs, slot, j) >= abs[a].adds[slot],
    decreases j,
{
    if a < j - 1 {
        lemma_best_adds(abs, slot, a, j - 1);
    } else {
        lemma_best_adds_nonneg(abs, slot, j - 1);
    }
}

proof fn lemma_best_adds_nonneg(abs: Seq<Ability>, slot: int, j: int)
    ensures best_adds(abs, slot, j) >= 0,
    decreases j,
{
    if j > 0 {
        lemma_best_adds_nonneg(abs, slot, j - 1);
    }
}

/// Tapping a source not yet tapped adds its best net, once.
proof fn lemma_used_net_insert(srcs: Seq<Vec<Ability>>, used: spec_fn(int) -> bool, s: int, n: int)
    requires !used(s), 0 <= s,
    ensures used_net(srcs, with(used, s), n)
        == used_net(srcs, used, n) + if s < n { best_net(srcs[s]@, srcs[s]@.len() as int) } else { 0 },
    decreases n,
{
    if n > 0 {
        lemma_used_net_insert(srcs, used, s, n - 1);
    }
}

proof fn lemma_used_adds_insert(srcs: Seq<Vec<Ability>>, slot: int, used: spec_fn(int) -> bool, s: int, n: int)
    requires !used(s), 0 <= s,
    ensures used_adds(srcs, slot, with(used, s), n)
        == used_adds(srcs, slot, used, n) + if s < n { best_adds(srcs[s]@, slot, srcs[s]@.len() as int) } else { 0 },
    decreases n,
{
    if n > 0 {
        lemma_used_adds_insert(srcs, slot, used, s, n - 1);
    }
}

proof fn lemma_used_net_none(srcs: Seq<Vec<Ability>>, used: spec_fn(int) -> bool, n: int)
    requires forall|x: int| 0 <= x < n ==> !#[trigger] used(x),
    ensures used_net(srcs, used, n) == 0,
    decreases n,
{
    if n > 0 {
        lemma_used_net_none(srcs, used, n - 1);
    }
}

proof fn lemma_used_adds_none(srcs: Seq<Vec<Ability>>, slot: int, used: spec_fn(int) -> bool, n: int)
    requires forall|x: int| 0 <= x < n ==> !#[trigger] used(x),
    ensures used_adds(srcs, slot, used, n) == 0,
    decreases n,
{
    if n > 0 {
        lemma_used_adds_none(srcs, slot, used, n - 1);
    }
}

/// Two predicates that agree on the first `n` sources count the same.
proof fn lemma_used_same(srcs: Seq<Vec<Ability>>, a: spec_fn(int) -> bool, b: spec_fn(int) -> bool, n: int)
    requires forall|x: int| 0 <= x < n ==> #[trigger] a(x) == b(x),
    ensures
        used_net(srcs, a, n) == used_net(srcs, b, n),
        forall|slot: int| #[trigger] used_adds(srcs, slot, a, n) == used_adds(srcs, slot, b, n),
    decreases n,
{
    if n > 0 {
        lemma_used_same(srcs, a, b, n - 1);
        assert(a(n - 1) == b(n - 1));
        assert forall|slot: int| #[trigger] used_adds(srcs, slot, a, n) == used_adds(srcs, slot, b, n) by {
            assert(used_adds(srcs, slot, a, n - 1) == used_adds(srcs, slot, b, n - 1));
        }
    }
}

/// The sources tapped count for no more than all of them.
proof fn lemma_used_net_le(srcs: Seq<Vec<Ability>>, used: spec_fn(int) -> bool, n: int)
    ensures used_net(srcs, used, n) <= total_net(srcs, n),
    decreases n,
{
    if n > 0 {
        lemma_used_net_le(srcs, used, n - 1);
        lemma_best_net_nonneg(srcs[n - 1]@, srcs[n - 1]@.len() as int);
    }
}

proof fn lemma_used_adds_le(srcs: Seq<Vec<Ability>>, slot: int, used: spec_fn(int) -> bool, n: int)
    ensures used_adds(srcs, slot, used, n) <= total_adds(srcs, slot, n),
    decreases n,
{
    if n > 0 {
        lemma_used_adds_le(srcs, slot, used, n - 1);
        lemma_best_adds_nonneg(srcs[n - 1]@, slot, srcs[n - 1]@.len() as int);
    }
}

/// After the first `k` taps, the pool holds no more in all than it did and
/// the best nets of the sources tapped, and of each type no more than it
/// did and the most of that type each source tapped adds.
proof fn lemma_run_bound(pool: Amounts, srcs: Seq<Vec<Ability>>, taps: Seq<(int, int)>, pools: Seq<Amounts>, k: int)
    requires runs(pool, srcs, taps, pools), 0 <= k <= taps.len(),
    ensures
        sum(pools[k]) <= sum(pool) + used_net(srcs, tapped(taps, k), srcs.len() as int),
        forall|i: int| 0 <= i < 6 ==> pools[k][i] <= pool[i] + #[trigger] used_adds(srcs, i, tapped(taps, k), srcs.len() as int),
    decreases k,
{
    let n = srcs.len() as int;
    if k == 0 {
        lemma_used_net_none(srcs, tapped(taps, 0), n);
        assert forall|i: int| 0 <= i < 6 implies pools[0][i] <= pool[i] + #[trigger] used_adds(srcs, i, tapped(taps, 0), n) by {
            lemma_used_adds_none(srcs, i, tapped(taps, 0), n);
        }
    } else {
        lemma_run_bound(pool, srcs, taps, pools, k - 1);
        let tap = taps[k - 1];
        let (s, a) = tap;
        let p = k - 1;
        assert(steps(srcs, taps[p], pools[p], pools[p + 1]));
        assert(pools[p + 1] == pools[k]);
        let ab = srcs[s]@[a];
        let left = choose|left: Amounts| #[trigger] paid_exactly(pools[k - 1], left, ab.cost)
            && forall|i: int| 0 <= i < 6 ==> pools[k][i] == left[i] + ab.adds[i];
        // The source is tapped here and not before.
        let before = tapped(taps, k - 1);
        assert(!before(s)) by {
            if before(s) {
                let j = choose|j: int| 0 <= j < k - 1 && taps[j].0 == s;
                assert(taps[j].0 != taps[k - 1].0);
            }
        }
        assert forall|x: int| 0 <= x < n implies #[trigger] tapped(taps, k)(x) == with(before, s)(x) by {
            if tapped(taps, k)(x) {
                let j = choose|j: int| 0 <= j < k && taps[j].0 == x;
                if j < k - 1 { assert(before(x)); }
            }
            if before(x) {
                let j = choose|j: int| 0 <= j < k - 1 && taps[j].0 == x;
                assert(0 <= j < k && taps[j].0 == x);
            }
            if x == s {
                assert(0 <= k - 1 < k && taps[k - 1].0 == x);
            }
        }
        lemma_used_same(srcs, tapped(taps, k), with(before, s), n);
        lemma_used_net_insert(srcs, before, s, n);
        lemma_best_net(srcs[s]@, a, srcs[s]@.len() as int);
        // In all: what was there, less the cost, and what the ability adds.
        assert(sum(pools[k]) == sum(left) + sum(ab.adds));
        assert(sum(pools[k - 1]) - sum(left) == asks(ab.cost));
        assert forall|i: int| 0 <= i < 6 implies pools[k][i] <= pool[i] + #[trigger] used_adds(srcs, i, tapped(taps, k), n) by {
            lemma_used_adds_insert(srcs, i, before, s, n);
            lemma_best_adds(srcs[s]@, i, a, srcs[s]@.len() as int);
            assert(pools[k - 1][i] <= pool[i] + used_adds(srcs, i, before, n));
            assert(left[i] <= pools[k - 1][i]);
        }
    }
}

/// The theorem: a cost some plan pays is within reach. For every pool,
/// every board of sources, and every plan that taps no source twice and
/// pays each ability's cost as a payment may.
pub proof fn reach_is_necessary(cost: Pips, pool: Amounts, srcs: Seq<Vec<Ability>>, taps: Seq<(int, int)>, pools: Seq<Amounts>)
    requires
        runs(pool, srcs, taps, pools),
        payable(pools[taps.len() as int], cost),
    ensures in_reach(cost, pool, srcs),
{
    let n = srcs.len() as int;
    let k = taps.len() as int;
    lemma_run_bound(pool, srcs, taps, pools, k);
    lemma_used_net_le(srcs, tapped(taps, k), n);
    assert forall|i: int| 0 <= i < 6 implies pool[i] + #[trigger] total_adds(srcs, i, n) >= cost.specific[i] by {
        lemma_used_adds_le(srcs, i, tapped(taps, k), n);
        assert(pools[k][i] <= pool[i] + used_adds(srcs, i, tapped(taps, k), n));
    }
}

/// How much is in six counts, as one number.
#[allow(clippy::cast_lossless)] // `as`: what Verus specifies
fn total_of(a: &Amounts) -> (t: u64)
    ensures t == sum(*a), t <= 6 * 0xffff_ffff,
{
    a[0] as u64 + a[1] as u64 + a[2] as u64 + a[3] as u64 + a[4] as u64 + a[5] as u64
}

/// What an ability nets.
#[allow(clippy::cast_lossless, clippy::implicit_saturating_sub)] // the arithmetic Verus reads
fn net_of(a: &Ability) -> (r: u64)
    ensures r == net(*a), r <= 6 * 0xffff_ffff,
{
    let adds = total_of(&a.adds);
    let asks = total_of(&a.cost.specific) + a.cost.generic as u64;
    if adds > asks { adds - asks } else { 0 }
}

/// The best nets of the sources, each once.
// `&Vec`, not a slice: the specifications read each source's abilities as
// a `Vec`'s view.
#[allow(clippy::ptr_arg)]
fn total_net_of(sources: &Vec<Vec<Ability>>) -> (t: u64)
    requires sources.len() <= 0x1_0000,
    ensures t == total_net(sources@, sources.len() as int), t <= sources.len() * (6 * 0xffff_ffff),
{
    let mut t: u64 = 0;
    let mut s: usize = 0;
    while s < sources.len()
        invariant
            s <= sources.len() <= 0x1_0000,
            t == total_net(sources@, s as int),
            t <= s * (6 * 0xffff_ffff),
        decreases sources.len() - s,
    {
        let abs = &sources[s];
        let mut best: u64 = 0;
        let mut j: usize = 0;
        while j < abs.len()
            invariant
                j <= abs.len(),
                abs@ == sources@[s as int]@,
                best == best_net(abs@, j as int),
                best <= 6 * 0xffff_ffff,
            decreases abs.len() - j,
        {
            let n = net_of(&abs[j]);
            if n > best {
                best = n;
            }
            j += 1;
        }
        assert(t + best <= (s + 1) * (6 * 0xffff_ffff)) by (nonlinear_arith)
            requires t <= s * (6 * 0xffff_ffff), best <= 6 * 0xffff_ffff;
        t += best;
        s += 1;
    }
    t
}

/// The most of type `slot` each source adds, in all.
#[allow(clippy::ptr_arg, clippy::cast_lossless)]
fn total_adds_of(sources: &Vec<Vec<Ability>>, slot: usize) -> (t: u64)
    requires sources.len() <= 0x1_0000, slot < 6,
    ensures t == total_adds(sources@, slot as int, sources.len() as int), t <= sources.len() * 0xffff_ffff,
{
    let mut t: u64 = 0;
    let mut s: usize = 0;
    while s < sources.len()
        invariant
            s <= sources.len() <= 0x1_0000,
            slot < 6,
            t == total_adds(sources@, slot as int, s as int),
            t <= s * 0xffff_ffff,
        decreases sources.len() - s,
    {
        let abs = &sources[s];
        let mut most: u64 = 0;
        let mut j: usize = 0;
        while j < abs.len()
            invariant
                j <= abs.len(),
                slot < 6,
                abs@ == sources@[s as int]@,
                most == best_adds(abs@, slot as int, j as int),
                most <= 0xffff_ffff,
            decreases abs.len() - j,
        {
            let v = abs[j].adds[slot] as u64;
            if v > most {
                most = v;
            }
            j += 1;
        }
        assert(t + most <= (s + 1) * 0xffff_ffff) by (nonlinear_arith)
            requires t <= s * 0xffff_ffff, most <= 0xffff_ffff;
        t += most;
        s += 1;
    }
    t
}

/// Whether `cost` is within reach of `pool` and `sources`, each source's
/// abilities its own list: [`in_reach`], which [`reach_is_necessary`]
/// proves every cost some plan pays is.
#[allow(clippy::ptr_arg, clippy::cast_lossless)]
#[must_use]
pub fn within_reach(cost: &Pips, pool: &Amounts, sources: &Vec<Vec<Ability>>) -> (r: bool)
    requires sources.len() <= 0x1_0000,
    ensures r == in_reach(*cost, *pool, sources@),
{
    let all = total_of(pool) + total_net_of(sources);
    if all < total_of(&cost.specific) + cost.generic as u64 {
        return false;
    }
    let mut i: usize = 0;
    while i < 6
        invariant
            i <= 6,
            sources.len() <= 0x1_0000,
            sum(*pool) + total_net(sources@, sources.len() as int) >= asks(*cost),
            forall|k: int| 0 <= k < i ==> pool[k] + #[trigger] total_adds(sources@, k, sources.len() as int) >= cost.specific[k],
        decreases 6 - i,
    {
        if (pool[i] as u64) + total_adds_of(sources, i) < cost.specific[i] as u64 {
            return false;
        }
        i += 1;
    }
    true
}

} // verus!
