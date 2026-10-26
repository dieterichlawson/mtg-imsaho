//! Funding X: which sums of mana the board's sources can tap for exactly,
//! which values of X can therefore be paid, and an allocation that pays one
//! (`funding::fundable_x_values` and `allocate_for_x` in the engine).
//!
//! The sources are grouped: `quanta[j]` mana for each tap of a group-`j`
//! source, and `counts[j]` sources in the group. Floating mana tops a tap
//! sum up by anything from nothing to the whole pool. The specification is
//! [`reachable`]: some number of each group's sources, none past its count,
//! adds up to the sum. Everything below is proved against it for every
//! board: every group, every count, every pool and every amount of X.

use vstd::prelude::*;

verus! {



/// What tapping `taps[j]` sources of each group `j` from `from` on adds up
/// to, at `quanta[j]` mana a tap.
pub open spec fn suffix_total(quanta: Seq<u32>, taps: Seq<u32>, from: int) -> int
    decreases quanta.len() - from,
{
    if from < 0 || from >= quanta.len() { 0 } else { taps[from] * quanta[from] + suffix_total(quanta, taps, from + 1) }
}

pub open spec fn fits(counts: Seq<u32>, taps: Seq<u32>, from: int) -> bool {
    &&& taps.len() == counts.len()
    &&& forall|j: int| from <= j < counts.len() ==> taps[j] <= counts[j]
}

/// Some choice of taps, at most `counts[j]` sources of each group `j` from
/// `from` on, adds up to exactly `s`.
pub open spec fn reachable(quanta: Seq<u32>, counts: Seq<u32>, from: int, s: int) -> bool {
    exists|taps: Seq<u32>| #[trigger] fits(counts, taps, from) && suffix_total(quanta, taps, from) == s
}

proof fn lemma_suffix_nonneg(quanta: Seq<u32>, taps: Seq<u32>, from: int)
    requires 0 <= from,
    ensures suffix_total(quanta, taps, from) >= 0,
    decreases quanta.len() - from,
{
    if from < quanta.len() {
        lemma_suffix_nonneg(quanta, taps, from + 1);
        assert(taps[from] * quanta[from] >= 0) by (nonlinear_arith);
    }
}

/// The total from `from` on reads only the taps from `from` on.
proof fn lemma_suffix_same(quanta: Seq<u32>, a: Seq<u32>, b: Seq<u32>, from: int)
    requires
        0 <= from,
        forall|j: int| from <= j < quanta.len() ==> a[j] == b[j],
    ensures suffix_total(quanta, a, from) == suffix_total(quanta, b, from),
    decreases quanta.len() - from,
{
    if from < quanta.len() {
        lemma_suffix_same(quanta, a, b, from + 1);
    }
}

/// Reachable from `i` is: some number of group `i`'s sources, up to its
/// count, on top of something reachable from `i + 1`.
proof fn lemma_reachable_step(quanta: Seq<u32>, counts: Seq<u32>, i: int, s: int)
    requires
        0 <= i < quanta.len(),
        quanta.len() == counts.len(),
    ensures
        reachable(quanta, counts, i, s) <==> exists|t: u32| t <= counts[i] && s - t * quanta[i] >= 0
            && #[trigger] reachable(quanta, counts, i + 1, s - t * quanta[i]),
{
    if reachable(quanta, counts, i, s) {
        let taps = choose|taps: Seq<u32>| #[trigger] fits(counts, taps, i) && suffix_total(quanta, taps, i) == s;
        let t = taps[i];
        lemma_suffix_nonneg(quanta, taps, i + 1);
        assert(fits(counts, taps, i + 1));
        assert(reachable(quanta, counts, i + 1, s - t * quanta[i]));
    }
    if exists|t: u32| t <= counts[i] && s - t * quanta[i] >= 0 && #[trigger] reachable(quanta, counts, i + 1, s - t * quanta[i]) {
        let t = choose|t: u32| t <= counts[i] && s - t * quanta[i] >= 0 && #[trigger] reachable(quanta, counts, i + 1, s - t * quanta[i]);
        let rest = choose|taps: Seq<u32>| #[trigger] fits(counts, taps, i + 1) && suffix_total(quanta, taps, i + 1) == s - t * quanta[i];
        let taps = rest.update(i, t);
        lemma_suffix_same(quanta, taps, rest, i + 1);
        assert(fits(counts, taps, i));
        assert(suffix_total(quanta, taps, i) == s);
    }
}

/// Nothing past the last group but the empty sum.
proof fn lemma_reachable_end(quanta: Seq<u32>, counts: Seq<u32>, s: int)
    requires quanta.len() == counts.len(),
    ensures reachable(quanta, counts, quanta.len() as int, s) <==> s == 0,
{
    let zeros = Seq::new(counts.len(), |j: int| 0u32);
    assert(fits(counts, zeros, quanta.len() as int));
    assert(suffix_total(quanta, zeros, quanta.len() as int) == 0);
}

/// `row` answers, for every sum up to its last index, whether the groups
/// from `i` on can tap for it exactly.
pub open spec fn is_row(quanta: Seq<u32>, counts: Seq<u32>, i: int, row: Seq<bool>) -> bool {
    forall|s: int| 0 <= s < row.len() ==> (row[s] <==> #[trigger] reachable(quanta, counts, i, s))
}

/// The sums `prev` reaches, moved up by up to `max_taps` steps of `q`: one
/// more group on top of the ones `prev` describes.
///
/// One pass, remembering for each sum how many steps of `q` back the
/// nearest sum `prev` reaches is, so a group of four thousand one-mana lands
/// costs a pass over the sums and not four thousand of them (#565).
// `&Vec`, not a slice: Verus specifies `Vec::clone` and not `slice::to_vec`.
#[allow(clippy::ptr_arg)]
fn step(prev: &Vec<bool>, q: u32, max_taps: u32) -> (next: Vec<bool>)
    ensures
        next.len() == prev.len(),
        forall|s: int| 0 <= s < next.len() ==> (next@[s] <==> exists|t: u32| t <= max_taps && s - t * q >= 0
            && #[trigger] prev@[s - t * q]),
{
    let width = prev.len();
    if q == 0 {
        let next = prev.clone();
        proof {
            assert forall|s: int| 0 <= s < next.len() implies (next@[s] <==> exists|t: u32| t <= max_taps && s - t * q >= 0
                && #[trigger] prev@[s - t * q]) by {
                if prev@[s] { assert(0u32 <= max_taps && s - 0u32 * q >= 0 && prev@[s - 0u32 * q]); }
            }
        }
        return next;
    }
    // dist[s]: how many steps of q back from s the nearest sum `prev`
    // reaches is, or none in reach.
    let mut dist: Vec<Option<usize>> = Vec::new();
    let mut next: Vec<bool> = Vec::new();
    let mut s: usize = 0;
    while s < width
        invariant
            q > 0,
            width == prev.len(),
            s <= width,
            dist.len() == s,
            next.len() == s,
            forall|p: int| 0 <= p < s ==> match #[trigger] dist@[p] {
                Some(d) => d * q <= p && prev@[p - d * q]
                    && forall|e: int| 0 <= e < d ==> !#[trigger] prev@[p - e * q],
                None => forall|e: int| 0 <= e && e * q <= p ==> !#[trigger] prev@[p - e * q],
            },
            forall|p: int| 0 <= p < s ==> (next@[p] <==> (#[trigger] dist@[p] matches Some(d) && d <= max_taps)),
        decreases width - s,
    {
        let here: Option<usize> = if prev[s] {
            Some(0)
        } else if s >= q as usize {
            match dist[s - q as usize] {
                Some(d) => {
                    // d steps of q back fit under s - q, so d < s.
                    proof {
                        assert((d as int) <= (d as int) * (q as int)) by (nonlinear_arith) requires q >= 1;
                    }
                    Some(d + 1)
                },
                None => None,
            }
        } else {
            None
        };
        proof {
            let si = s as int;
            let qi = q as int;
            if !prev@[si] {
                if si >= qi {
                    let back = si - qi;
                    match dist@[back] {
                        Some(d) => {
                            let di = d as int;
                            assert(di <= di * qi) by (nonlinear_arith) requires qi >= 1, di >= 0;
                            assert((di + 1) * qi == di * qi + qi) by (nonlinear_arith);
                            assert(si - (di + 1) * qi == back - di * qi);
                            assert forall|e: int| 0 <= e < di + 1 implies !#[trigger] prev@[si - e * qi] by {
                                if e > 0 {
                                    assert(si - e * qi == back - (e - 1) * qi) by (nonlinear_arith) requires back == si - qi;
                                    assert(!prev@[back - (e - 1) * qi]);
                                } else {
                                    assert(si - e * qi == si) by (nonlinear_arith) requires e == 0;
                                }
                            }
                        },
                        None => {
                            assert forall|e: int| 0 <= e && e * qi <= si implies !#[trigger] prev@[si - e * qi] by {
                                if e > 0 {
                                    assert(si - e * qi == back - (e - 1) * qi) by (nonlinear_arith) requires back == si - qi;
                                    assert((e - 1) * qi <= back) by (nonlinear_arith) requires e * qi <= si, back == si - qi;
                                    assert(!prev@[back - (e - 1) * qi]);
                                } else {
                                    assert(si - e * qi == si) by (nonlinear_arith) requires e == 0;
                                }
                            }
                        },
                    }
                } else {
                    assert forall|e: int| 0 <= e && e * qi <= si implies !#[trigger] prev@[si - e * qi] by {
                        if e > 0 {
                            assert(e * qi >= qi) by (nonlinear_arith) requires e >= 1, qi >= 1;
                        } else {
                            assert(si - e * qi == si) by (nonlinear_arith) requires e == 0;
                        }
                    }
                }
            } else {
                assert(si - 0 * qi == si);
            }
        }
        dist.push(here);
        next.push(match here { Some(d) => d <= max_taps as usize, None => false });
        s += 1;
    }
    proof {
        assert forall|p: int| 0 <= p < next.len() implies (next@[p] <==> exists|t: u32| t <= max_taps && p - t * q >= 0
            && #[trigger] prev@[p - t * q]) by {
            match dist@[p] {
                Some(d) => {
                    if d <= max_taps {
                        let t = d as u32;
                        assert(t <= max_taps && p - t * q >= 0 && prev@[p - t * q]);
                    } else {
                        if exists|t: u32| t <= max_taps && p - t * q >= 0 && #[trigger] prev@[p - t * q] {
                            let t = choose|t: u32| t <= max_taps && p - t * q >= 0 && #[trigger] prev@[p - t * q];
                            assert(!prev@[p - (t as int) * q]);
                        }
                    }
                },
                None => {
                    if exists|t: u32| t <= max_taps && p - t * q >= 0 && #[trigger] prev@[p - t * q] {
                        let t = choose|t: u32| t <= max_taps && p - t * q >= 0 && #[trigger] prev@[p - t * q];
                        assert(!prev@[p - (t as int) * q]);
                    }
                },
            }
        }
    }
    next
}

/// For each group `i` (and one past the last), which sums up to `limit`
/// the groups from `i` on can tap for exactly.
#[must_use]
pub fn reachability(quanta: &[u32], counts: &[u32], limit: u32) -> (rows: Vec<Vec<bool>>)
    requires
        quanta.len() == counts.len(),
    ensures
        rows.len() == quanta.len() + 1,
        forall|i: int| 0 <= i <= quanta.len() ==> #[trigger] rows@[i]@.len() == limit + 1
            && is_row(quanta@, counts@, i, rows@[i]@),
{
    let n = quanta.len();
    let width = limit as usize + 1;
    let mut last: Vec<bool> = vec![false; width];
    last.set(0, true);
    proof {
        assert forall|s: int| 0 <= s < last@.len() implies (last@[s] <==> #[trigger] reachable(quanta@, counts@, n as int, s)) by {
            lemma_reachable_end(quanta@, counts@, s);
        }
    }
    let mut rows: Vec<Vec<bool>> = Vec::new();
    rows.push(last);
    let mut i = n;
    while i > 0
        invariant
            n == quanta.len(),
            n == counts.len(),
            width == limit + 1,
            i <= n,
            rows.len() == n - i + 1,
            forall|k: int| 0 <= k < rows.len() ==> #[trigger] rows@[k]@.len() == width
                && is_row(quanta@, counts@, i + k, rows@[k]@),
        decreases i,
    {
        let q = quanta[i - 1];
        let count = counts[i - 1];
        let next = step(&rows[0], q, count);
        proof {
            let prev = rows@[0]@;
            assert(is_row(quanta@, counts@, i as int, prev));
            assert forall|s: int| 0 <= s < next@.len() implies (next@[s] <==> #[trigger] reachable(quanta@, counts@, i - 1, s)) by {
                lemma_reachable_step(quanta@, counts@, i - 1, s);
                assert(q == quanta@[i - 1] && count == counts@[i - 1]);
                if next@[s] {
                    let t = choose|t: u32| t <= count && s - t * q >= 0 && #[trigger] prev[s - t * q];
                    assert(s - t * q < prev.len());
                    assert(reachable(quanta@, counts@, i as int, s - t * q));
                    assert(t <= counts@[i - 1] && s - t * quanta@[i - 1] >= 0
                        && reachable(quanta@, counts@, i as int, s - t * quanta@[i - 1]));
                } else if reachable(quanta@, counts@, i - 1, s) {
                    let t = choose|t: u32| t <= counts@[i - 1] && s - t * quanta@[i - 1] >= 0
                        && #[trigger] reachable(quanta@, counts@, i as int, s - t * quanta@[i - 1]);
                    assert(s - t * q <= s);
                    assert(prev[s - t * q]);
                    assert(t <= count && s - t * q >= 0 && prev[s - t * q]);
                }
            }
        }
        rows.insert(0, next);
        i -= 1;
    }
    rows
}

/// What taps of the groups before `i` add up to.
pub open spec fn prefix_total(quanta: Seq<u32>, taps: Seq<u32>, i: int) -> int
    decreases i,
{
    if i <= 0 { 0 } else { prefix_total(quanta, taps, i - 1) + taps[i - 1] * quanta[i - 1] }
}

proof fn lemma_prefix_suffix(quanta: Seq<u32>, taps: Seq<u32>, i: int)
    requires 0 <= i <= quanta.len(),
    ensures prefix_total(quanta, taps, i) + suffix_total(quanta, taps, i) == suffix_total(quanta, taps, 0),
    decreases i,
{
    if i > 0 {
        lemma_prefix_suffix(quanta, taps, i - 1);
    }
}

/// Some allocation of these groups and up to `pool` floating mana funds
/// exactly `mana`.
pub open spec fn fundable(quanta: Seq<u32>, counts: Seq<u32>, pool: int, mana: int) -> bool {
    exists|s: int| mana - pool <= s <= mana && #[trigger] reachable(quanta, counts, 0, s)
}

/// How to fund `mana` (X less any cost reduction): how many sources of
/// each group to tap, how much floating mana to spend, and what is short.
pub struct Funding {
    pub taps: Vec<u32>,
    pub tap_sum: u32,
    pub from_pool: u32,
    pub shortfall: u32,
}

/// Fund `mana` out of `pool` floating mana and groups of sources (`quanta`
/// mana per tap, `counts` sources each), as `funding::allocate_for_x` does.
///
/// - The taps are an allocation the board has (no group past its count)
///   and add up to `tap_sum`; taps, pool and shortfall add up to `mana`.
/// - Nothing is short exactly when some allocation funds `mana` exactly.
/// - Then the pool is spent as fully as any exact allocation spends it:
///   no exact allocation taps less.
/// - And nothing funds more of `mana`: no tap sum the board has, topped up
///   with as much of the pool as fits, comes to more.
/// - Among the exact allocations with that tap sum, earlier groups give as
///   much as they can (a preference, which the proof does not state).
// Each `as` narrows a value proved to be at most a `u32` (`mana`).
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fund(pool: u32, quanta: &[u32], counts: &[u32], mana: u32) -> (f: Funding)
    requires
        quanta.len() == counts.len(),
    ensures
        f.taps.len() == quanta.len(),
        fits(counts@, f.taps@, 0),
        suffix_total(quanta@, f.taps@, 0) == f.tap_sum,
        f.tap_sum + f.from_pool + f.shortfall == mana,
        f.from_pool <= pool,
        (f.shortfall == 0) == fundable(quanta@, counts@, pool as int, mana as int),
        f.shortfall == 0 ==> forall|s: int| mana - pool <= s <= mana
            && #[trigger] reachable(quanta@, counts@, 0, s) ==> f.tap_sum <= s,
        forall|s: int| 0 <= s <= mana && #[trigger] reachable(quanta@, counts@, 0, s)
            ==> s + (if mana - s < pool { mana - s } else { pool as int }) <= f.tap_sum + f.from_pool,
{
    let n = quanta.len();
    let rows = reachability(quanta, counts, mana);
    let floor = mana.saturating_sub(pool);
    // The smallest tap sum the pool can top up to exactly `mana`.
    let mut sum: u64 = u64::from(floor);
    while sum <= u64::from(mana) && !rows[0][sum as usize]
        invariant
            rows.len() == n + 1,
            rows@[0]@.len() == mana + 1,
            is_row(quanta@, counts@, 0, rows@[0]@),
            floor <= sum <= mana + 1,
            forall|r: int| floor <= r < sum ==> !#[trigger] reachable(quanta@, counts@, 0, r),
        decreases mana + 1 - sum,
    {
        sum += 1;
    }
    let exact = sum <= u64::from(mana);
    let tap_sum: u32 = if exact {
        sum as u32
    } else {
        // None: the most the board reaches below the window, behind the
        // whole pool. Zero always is.
        proof {
            lemma_reachable_end(quanta@, counts@, 0);
            assert(reachable(quanta@, counts@, 0, 0)) by {
                let zeros = Seq::new(counts@.len(), |j: int| 0u32);
                assert(fits(counts@, zeros, 0));
                assert forall|k: int| 0 <= k <= n implies #[trigger] suffix_total(quanta@, zeros, k) == 0 by {
                    lemma_zero_suffix(quanta@, zeros, k);
                }
            }
        }
        let mut below: u32 = floor;
        while below > 0 && !rows[0][below as usize]
            invariant
                rows.len() == n + 1,
                rows@[0]@.len() == mana + 1,
                is_row(quanta@, counts@, 0, rows@[0]@),
                below <= floor,
                floor <= mana,
                reachable(quanta@, counts@, 0, 0),
                forall|r: int| below < r <= floor ==> !#[trigger] reachable(quanta@, counts@, 0, r),
            decreases below,
        {
            below -= 1;
        }
        below
    };
    let rest = mana - tap_sum;
    let from_pool = if rest < pool { rest } else { pool };
    let shortfall = rest - from_pool;

    // Rebuild the taps that add up to `tap_sum`, group by group, each
    // giving the most it can while what is left stays reachable behind it.
    let mut taps: Vec<u32> = Vec::new();
    let mut left: u32 = tap_sum;
    let mut i: usize = 0;
    while i < n
        invariant
            n == quanta.len(),
            n == counts.len(),
            rows.len() == n + 1,
            forall|k: int| 0 <= k <= n ==> #[trigger] rows@[k]@.len() == mana + 1
                && is_row(quanta@, counts@, k, rows@[k]@),
            i <= n,
            taps.len() == i,
            forall|j: int| 0 <= j < i ==> taps@[j] <= counts@[j],
            left <= tap_sum <= mana,
            prefix_total(quanta@, taps@, i as int) + left == tap_sum,
            reachable(quanta@, counts@, i as int, left as int),
        decreases n - i,
    {
        let quantum = quanta[i];
        let count = counts[i];
        proof { lemma_reachable_step(quanta@, counts@, i as int, left as int); }
        let taken: u32 = if quantum == 0 || left == 0 {
            proof {
                if quantum != 0 {
                    // left == 0: nothing from this group.
                    let t0 = choose|taken: u32| taken <= counts@[i as int] && left - taken * quanta@[i as int] >= 0
                        && #[trigger] reachable(quanta@, counts@, i + 1, left - taken * quanta@[i as int]);
                    assert(t0 * quantum == 0) by (nonlinear_arith) requires left - t0 * quantum >= 0, left == 0, quantum >= 1, t0 >= 0;
                    assert(left - 0u32 * quantum == left - t0 * quantum);
                } else {
                    let t0 = choose|taken: u32| taken <= counts@[i as int] && left - taken * quanta@[i as int] >= 0
                        && #[trigger] reachable(quanta@, counts@, i + 1, left - taken * quanta@[i as int]);
                    assert(left - t0 * quantum == left - 0u32 * quantum);
                }
            }
            0
        } else {
            let ghost t0 = choose|taken: u32| taken <= counts@[i as int] && left - taken * quanta@[i as int] >= 0
                && #[trigger] reachable(quanta@, counts@, i + 1, left - taken * quanta@[i as int]);
            proof {
                assert(t0 <= left / quantum) by (nonlinear_arith) requires t0 * quantum <= left, quantum >= 1;
            }
            let mut taken = if left / quantum < count { left / quantum } else { count };
            proof {
                assert((taken as int) * (quantum as int) <= left) by (nonlinear_arith) requires taken <= left / quantum, quantum >= 1;
            }
            let mut found = rows[i + 1][(left - taken * quantum) as usize];
            while !found
                invariant
                    quantum >= 1,
                    t0 <= taken,
                    taken <= count,
                    (taken as int) * (quantum as int) <= left,
                    t0 * quantum <= left,
                    rows.len() == n + 1,
                    i + 1 <= n,
                    rows@[i + 1]@.len() == mana + 1,
                    left <= mana,
                    is_row(quanta@, counts@, i + 1, rows@[i + 1]@),
                    reachable(quanta@, counts@, i + 1, left - t0 * quantum),
                    found == rows@[i + 1]@[left - taken * quantum],
                decreases taken,
            {
                proof {
                    if taken == t0 { assert(rows@[i + 1]@[left - t0 * quantum]); }
                    assert(taken > t0);
                    assert(((taken - 1) as int) * (quantum as int) <= left) by (nonlinear_arith)
                        requires (taken as int) * (quantum as int) <= left, taken >= 1, quantum >= 1;
                }
                taken -= 1;
                found = rows[i + 1][(left - taken * quantum) as usize];
            }
            taken
        };
        proof {
            assert(reachable(quanta@, counts@, i + 1, left - taken * quantum));
            assert(prefix_total(quanta@, taps@.push(taken), i + 1)
                == prefix_total(quanta@, taps@, i as int) + taken * quantum) by {
                lemma_prefix_push(quanta@, taps@, taken, i as int);
            }
        }
        taps.push(taken);
        left -= taken * quantum;
        i += 1;
    }
    proof {
        lemma_reachable_end(quanta@, counts@, left as int);
        lemma_prefix_suffix(quanta@, taps@, n as int);
        assert(suffix_total(quanta@, taps@, n as int) == 0);
        if shortfall == 0 {
            assert(reachable(quanta@, counts@, 0, tap_sum as int));
        }
        if fundable(quanta@, counts@, pool as int, mana as int) {
            let w = choose|r: int| mana - pool <= r <= mana && #[trigger] reachable(quanta@, counts@, 0, r);
            if !exact {
                assert(r_window(w, floor as int, mana as int));
            }
        }
        if exact {
            assert forall|r: int| mana - pool <= r <= mana && #[trigger] reachable(quanta@, counts@, 0, r)
                implies tap_sum <= r by {
                lemma_reachable_nonneg(quanta@, counts@, 0, r);
                if r < tap_sum { assert(floor <= r); }
            }
        }
    }
    Funding { taps, tap_sum, from_pool, shortfall }
}

proof fn lemma_reachable_nonneg(quanta: Seq<u32>, counts: Seq<u32>, from: int, s: int)
    requires 0 <= from, reachable(quanta, counts, from, s),
    ensures s >= 0,
{
    let taps = choose|taps: Seq<u32>| #[trigger] fits(counts, taps, from) && suffix_total(quanta, taps, from) == s;
    lemma_suffix_nonneg(quanta, taps, from);
}

/// For every amount of mana up to `limit`, whether some allocation funds
/// it exactly: `ok[m]` is `fundable(.., m)`.
// Each `as` narrows a value proved to be at most a `u32` (`limit`).
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn fundable_amounts(pool: u32, quanta: &[u32], counts: &[u32], limit: u32) -> (ok: Vec<bool>)
    requires
        quanta.len() == counts.len(),
    ensures
        ok.len() == limit + 1,
        forall|m: int| 0 <= m <= limit ==> (ok@[m] <==> #[trigger] fundable(quanta@, counts@, pool as int, m)),
{
    let rows = reachability(quanta, counts, limit);
    let mut ok: Vec<bool> = Vec::new();
    // The largest sum the board taps for exactly, at or below m.
    let mut best: Option<u32> = None;
    let mut m: u64 = 0;
    while m <= u64::from(limit)
        invariant
            rows.len() == quanta.len() + 1,
            rows@[0]@.len() == limit + 1,
            is_row(quanta@, counts@, 0, rows@[0]@),
            m <= limit + 1,
            ok.len() == m,
            match best {
                Some(b) => b < m && reachable(quanta@, counts@, 0, b as int)
                    && forall|r: int| b < r < m ==> !#[trigger] reachable(quanta@, counts@, 0, r),
                None => forall|r: int| 0 <= r < m ==> !#[trigger] reachable(quanta@, counts@, 0, r),
            },
            forall|k: int| 0 <= k < m ==> (ok@[k] <==> #[trigger] fundable(quanta@, counts@, pool as int, k)),
        decreases limit + 1 - m,
    {
        if rows[0][m as usize] {
            best = Some(m as u32);
        }
        let here = match best {
            Some(b) => (m as u32) - b <= pool,
            None => false,
        };
        proof {
            let mi = m as int;
            if here {
                assert(reachable(quanta@, counts@, 0, best.unwrap() as int));
            }
            if fundable(quanta@, counts@, pool as int, mi) {
                let w = choose|s: int| mi - pool <= s <= mi && #[trigger] reachable(quanta@, counts@, 0, s);
                lemma_reachable_nonneg(quanta@, counts@, 0, w);
                assert(best is Some);
                assert(w <= best.unwrap());
            }
        }
        ok.push(here);
        m += 1;
    }
    ok
}

spec fn r_window(r: int, floor: int, mana: int) -> bool { floor <= r <= mana }

proof fn lemma_zero_suffix(quanta: Seq<u32>, zeros: Seq<u32>, k: int)
    requires
        0 <= k,
        zeros.len() == quanta.len(),
        forall|j: int| 0 <= j < zeros.len() ==> zeros[j] == 0,
    ensures suffix_total(quanta, zeros, k) == 0,
    decreases quanta.len() - k,
{
    if k < quanta.len() {
        lemma_zero_suffix(quanta, zeros, k + 1);
    }
}

proof fn lemma_prefix_push(quanta: Seq<u32>, taps: Seq<u32>, t: u32, i: int)
    requires 0 <= i, taps.len() == i,
    ensures prefix_total(quanta, taps.push(t), i + 1) == prefix_total(quanta, taps, i) + t * quanta[i],
{
    lemma_prefix_same(quanta, taps.push(t), taps, i);
}

proof fn lemma_prefix_same(quanta: Seq<u32>, a: Seq<u32>, b: Seq<u32>, i: int)
    requires
        0 <= i,
        i <= a.len(),
        i <= b.len(),
        forall|j: int| 0 <= j < i ==> a[j] == b[j],
    ensures prefix_total(quanta, a, i) == prefix_total(quanta, b, i),
    decreases i,
{
    if i > 0 {
        lemma_prefix_same(quanta, a, b, i - 1);
    }
}

} // verus!
