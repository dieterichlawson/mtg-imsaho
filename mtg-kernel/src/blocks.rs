//! Which of a defending player's submitted blocks stand, proved against
//! CR 509.1a-b for every submission.
//!
//! A player (or a seat answering for one) submits (blocker, attacker)
//! pairs. The rules allow a declaration in which each pair is a block the
//! blocker may make, no blocker blocks two attackers (509.1a), and an
//! attacker that can't be blocked except by N or more creatures is blocked
//! by none or by at least N (509.1b). [`judge_blocks`] says, for every
//! submitted pair, whether it stands and if not why:
//!
//! - the pairs that stand are always a legal declaration;
//! - a submission that already is one stands whole.

use vstd::prelude::*;

verus! {

/// What became of one submitted pair.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum Verdict {
    /// It stands.
    Accepted,
    /// The same pair already stands earlier in the submission.
    Duplicate,
    /// The blocker already blocks another attacker earlier in the
    /// submission (509.1a).
    BlockerTaken,
    /// The rules do not let this blocker block this attacker.
    Illegal,
    /// Too few creatures blocked this attacker for its minimum (509.1b).
    UnderMinimum,
}

/// How many of the first `n` pairs judged `Accepted` block attacker `a`.
pub open spec fn standing(pairs: Seq<(usize, usize)>, v: Seq<Verdict>, a: usize, n: int) -> nat
    decreases n,
{
    if n <= 0 {
        0
    } else {
        standing(pairs, v, a, n - 1) + if v[n - 1] == Verdict::Accepted && pairs[n - 1].1 == a { 1nat } else { 0nat }
    }
}

/// How many of the first `n` pairs block attacker `a`.
pub open spec fn blocking(pairs: Seq<(usize, usize)>, a: usize, n: int) -> nat
    decreases n,
{
    if n <= 0 { 0 } else { blocking(pairs, a, n - 1) + if pairs[n - 1].1 == a { 1nat } else { 0nat } }
}

/// The pairs judged `Accepted` are a declaration the rules allow: every
/// one legal, no blocker in two, and every blocked attacker blocked by at
/// least its minimum.
pub open spec fn accepted_are_legal(pairs: Seq<(usize, usize)>, v: Seq<Verdict>, legal: Seq<Seq<bool>>, minimum: Seq<u32>) -> bool {
    &&& forall|k: int| 0 <= k < pairs.len() && #[trigger] v[k] == Verdict::Accepted ==> legal[pairs[k].0 as int][pairs[k].1 as int]
    &&& forall|i: int, j: int| 0 <= i < j < pairs.len() && #[trigger] v[i] == Verdict::Accepted && #[trigger] v[j] == Verdict::Accepted
        ==> pairs[i].0 != pairs[j].0
    &&& forall|k: int| 0 <= k < pairs.len() && #[trigger] v[k] == Verdict::Accepted
        ==> standing(pairs, v, pairs[k].1, pairs.len() as int) >= minimum[pairs[k].1 as int]
}

/// The whole submission is a declaration the rules allow.
pub open spec fn legal_declaration(pairs: Seq<(usize, usize)>, legal: Seq<Seq<bool>>, minimum: Seq<u32>) -> bool {
    &&& forall|k: int| 0 <= k < pairs.len() ==> #[trigger] legal[pairs[k].0 as int][pairs[k].1 as int]
    &&& forall|i: int, j: int| 0 <= i < j < pairs.len() ==> #[trigger] pairs[i].0 != #[trigger] pairs[j].0
    &&& forall|k: int| 0 <= k < pairs.len() ==> blocking(pairs, #[trigger] pairs[k].1, pairs.len() as int) >= minimum[pairs[k].1 as int]
}

/// Indices in range: every pair names a row of `legal` and a column of it
/// and of `minimum`.
pub open spec fn well_formed(pairs: Seq<(usize, usize)>, legal: Seq<Seq<bool>>, minimum: Seq<u32>) -> bool {
    &&& forall|b: int| 0 <= b < legal.len() ==> #[trigger] legal[b].len() == minimum.len()
    &&& forall|k: int| 0 <= k < pairs.len() ==> (#[trigger] pairs[k]).0 < legal.len() && pairs[k].1 < minimum.len()
}

/// The verdicts after the second pass, from the first pass's: a standing
/// pair whose attacker is under its minimum no longer stands.
pub open spec fn second(pairs: Seq<(usize, usize)>, v1: Seq<Verdict>, minimum: Seq<u32>, k: int) -> Verdict {
    if v1[k] == Verdict::Accepted && standing(pairs, v1, pairs[k].1, pairs.len() as int) < minimum[pairs[k].1 as int] {
        Verdict::UnderMinimum
    } else {
        v1[k]
    }
}

proof fn lemma_second_pass_counts(pairs: Seq<(usize, usize)>, v1: Seq<Verdict>, v2: Seq<Verdict>, minimum: Seq<u32>, a: usize, n: int)
    requires
        0 <= n <= pairs.len(),
        v1.len() == pairs.len(),
        v2.len() == pairs.len(),
        (a as int) < minimum.len(),
        forall|k: int| 0 <= k < pairs.len() ==> #[trigger] v2[k] == second(pairs, v1, minimum, k),
        forall|k: int| 0 <= k < pairs.len() ==> (pairs[k].1 as int) < minimum.len(),
    ensures
        standing(pairs, v2, a, n) == if standing(pairs, v1, a, pairs.len() as int) >= minimum[a as int] {
            standing(pairs, v1, a, n)
        } else {
            0
        },
    decreases n,
{
    if n > 0 {
        lemma_second_pass_counts(pairs, v1, v2, minimum, a, n - 1);
    }
}

proof fn lemma_all_accepted_counts(pairs: Seq<(usize, usize)>, v: Seq<Verdict>, a: usize, n: int)
    requires
        0 <= n <= pairs.len(),
        v.len() == pairs.len(),
        forall|k: int| 0 <= k < pairs.len() ==> #[trigger] v[k] == Verdict::Accepted,
    ensures standing(pairs, v, a, n) == blocking(pairs, a, n),
    decreases n,
{
    if n > 0 {
        lemma_all_accepted_counts(pairs, v, a, n - 1);
    }
}

proof fn lemma_standing_extend(pairs: Seq<(usize, usize)>, v: Seq<Verdict>, w: Seq<Verdict>, a: usize, n: int)
    requires
        0 <= n <= v.len(), n <= w.len(), n <= pairs.len(),
        forall|k: int| 0 <= k < n ==> #[trigger] v[k] == w[k],
    ensures standing(pairs, v, a, n) == standing(pairs, w, a, n),
    decreases n,
{
    if n > 0 {
        lemma_standing_extend(pairs, v, w, a, n - 1);
    }
}

proof fn lemma_standing_bounded(pairs: Seq<(usize, usize)>, v: Seq<Verdict>, a: usize, n: int)
    requires 0 <= n,
    ensures standing(pairs, v, a, n) <= n,
    decreases n,
{
    if n > 0 {
        lemma_standing_bounded(pairs, v, a, n - 1);
    }
}

fn is_accepted(x: Verdict) -> (r: bool)
    ensures r == (x == Verdict::Accepted),
{
    match x {
        Verdict::Accepted => true,
        _ => false,
    }
}

/// What became of each of `pairs`, `legal[b][a]` being whether blocker `b`
/// may block attacker `a` and `minimum[a]` the fewest creatures that may
/// block `a`.
///
/// - The pairs judged `Accepted` are a legal declaration.
/// - A pair is judged `Illegal` only when the rules do not let its blocker
///   block its attacker, and is so judged whenever that is true and its
///   blocker is not already taken.
/// - A submission that is a legal declaration is accepted whole.
pub fn judge_blocks(pairs: &Vec<(usize, usize)>, legal: &Vec<Vec<bool>>, minimum: &Vec<u32>) -> (v: Vec<Verdict>)
    requires well_formed(pairs@, legal@.map_values(|r: Vec<bool>| r@), minimum@),
    ensures
        v.len() == pairs.len(),
        accepted_are_legal(pairs@, v@, legal@.map_values(|r: Vec<bool>| r@), minimum@),
        forall|k: int| 0 <= k < pairs.len() && #[trigger] v@[k] == Verdict::Illegal
            ==> !legal@[pairs@[k].0 as int]@[pairs@[k].1 as int],
        legal_declaration(pairs@, legal@.map_values(|r: Vec<bool>| r@), minimum@)
            ==> forall|k: int| 0 <= k < pairs.len() ==> #[trigger] v@[k] == Verdict::Accepted,
{
    let ghost lg = legal@.map_values(|r: Vec<bool>| r@);
    let n = pairs.len();

    // First pass, in submission order: a pair stands for now unless the
    // same pair or another pair with its blocker already stands (509.1a),
    // or the rules refuse it.
    let mut v: Vec<Verdict> = Vec::new();
    let mut k: usize = 0;
    while k < n
        invariant
            n == pairs.len(),
            well_formed(pairs@, lg, minimum@),
            lg == legal@.map_values(|r: Vec<bool>| r@),
            k <= n,
            v.len() == k,
            forall|j: int| 0 <= j < k && #[trigger] v@[j] == Verdict::Accepted ==> lg[pairs@[j].0 as int][pairs@[j].1 as int],
            forall|j: int| 0 <= j < k && #[trigger] v@[j] == Verdict::Illegal ==> !lg[pairs@[j].0 as int][pairs@[j].1 as int],
            forall|j: int| 0 <= j < k ==> #[trigger] v@[j] != Verdict::UnderMinimum,
            forall|i: int, j: int| 0 <= i < j < k && #[trigger] v@[i] == Verdict::Accepted && #[trigger] v@[j] == Verdict::Accepted
                ==> pairs@[i].0 != pairs@[j].0,
            legal_declaration(pairs@, lg, minimum@) ==> forall|j: int| 0 <= j < k ==> #[trigger] v@[j] == Verdict::Accepted,
        decreases n - k,
    {
        let (b, a) = pairs[k];
        let mut taken = false;
        let mut same = false;
        let mut j: usize = 0;
        while j < k
            invariant
                k < n, n == pairs.len(), v.len() == k, j <= k,
                b == pairs@[k as int].0,
                !taken ==> forall|i: int| 0 <= i < j && #[trigger] v@[i] == Verdict::Accepted ==> pairs@[i].0 != b,
                taken ==> exists|i: int| 0 <= i < k && #[trigger] v@[i] == Verdict::Accepted && pairs@[i].0 == b,
            decreases k - j,
        {
            if is_accepted(v[j]) && pairs[j].0 == b {
                taken = true;
                if pairs[j].1 == a {
                    same = true;
                }
            }
            j += 1;
        }
        proof {
            assert(lg[b as int] == legal@[b as int]@);
            if taken && legal_declaration(pairs@, lg, minimum@) {
                let i = choose|i: int| 0 <= i < k && #[trigger] v@[i] == Verdict::Accepted && pairs@[i].0 == b;
                assert(pairs@[i].0 != pairs@[k as int].0);
            }
        }
        let verdict = if taken {
            if same { Verdict::Duplicate } else { Verdict::BlockerTaken }
        } else if !legal[b][a] {
            Verdict::Illegal
        } else {
            Verdict::Accepted
        };
        v.push(verdict);
        k += 1;
    }
    let ghost v1 = v@;

    // Count what stands of each attacker.
    let m = minimum.len();
    let mut count: Vec<u64> = Vec::new();
    let mut x: usize = 0;
    while x < m
        invariant x <= m, m == minimum.len(), count.len() == x,
            forall|y: int| 0 <= y < x ==> #[trigger] count@[y] == 0,
        decreases m - x,
    {
        count.push(0);
        x += 1;
    }
    let mut k: usize = 0;
    while k < n
        invariant
            n == pairs.len(), m == minimum.len(), v@ == v1, v1.len() == n,
            well_formed(pairs@, lg, minimum@),
            count.len() == m,
            k <= n,
            forall|y: int| 0 <= y < m ==> #[trigger] count@[y] == standing(pairs@, v1, y as usize, k as int),
        decreases n - k,
    {
        proof {
            assert forall|y: int| 0 <= y < m implies #[trigger] count@[y] <= k by {
                lemma_standing_bounded(pairs@, v1, y as usize, k as int);
            }
        }
        if is_accepted(v[k]) {
            let at = pairs[k].1;
            let c = count[at];
            count.set(at, c + 1);
        }
        k += 1;
    }

    // Second pass: an attacker blocked by fewer than its minimum is not
    // blocked at all (509.1b).
    let mut k: usize = 0;
    while k < n
        invariant
            n == pairs.len(), m == minimum.len(), v.len() == n, v1.len() == n,
            well_formed(pairs@, lg, minimum@),
            count.len() == m,
            k <= n,
            forall|y: int| 0 <= y < m ==> #[trigger] count@[y] == standing(pairs@, v1, y as usize, n as int),
            forall|j: int| 0 <= j < k ==> #[trigger] v@[j] == second(pairs@, v1, minimum@, j),
            forall|j: int| k <= j < n ==> #[trigger] v@[j] == v1[j],
        decreases n - k,
    {
        let at = pairs[k].1;
        if is_accepted(v[k]) && count[at] < u64::from(minimum[at]) {
            v.set(k, Verdict::UnderMinimum);
        }
        k += 1;
    }

    proof {
        let v2 = v@;
        // What stands now, by attacker: all its first-pass blocks if they
        // meet its minimum, none otherwise.
        assert forall|k2: int| 0 <= k2 < n && #[trigger] v2[k2] == Verdict::Accepted implies
            standing(pairs@, v2, pairs@[k2].1, n as int) >= minimum@[pairs@[k2].1 as int] by {
            lemma_second_pass_counts(pairs@, v1, v2, minimum@, pairs@[k2].1, n as int);
        }
        if legal_declaration(pairs@, lg, minimum@) {
            assert forall|k2: int| 0 <= k2 < n implies #[trigger] v2[k2] == Verdict::Accepted by {
                lemma_all_accepted_counts(pairs@, v1, pairs@[k2].1, n as int);
            }
        }
    }
    v
}

} // verus!
