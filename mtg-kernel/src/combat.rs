//! Dividing a blocked attacker's combat damage among its blockers, proved
//! against CR 510.1c-d for every power, every number of blockers and every
//! lethal amount.
//!
//! The engine follows the damage assignment order of CR 509.2 (the rules
//! of Innistrad's time): the attacking player orders the blockers, and a
//! blocker may be assigned damage only once every blocker before it has
//! lethal. Within that, how much each blocker gets is the attacking
//! player's to choose, one blocker at a time ([`divide`]).

use vstd::prelude::*;

verus! {


/// Everything in a sequence of amounts.
pub open spec fn seq_sum(s: Seq<u32>) -> int
    decreases s.len(),
{
    if s.len() == 0 { 0 } else { seq_sum(s.drop_last()) + s.last() }
}

proof fn lemma_sum_push(s: Seq<u32>, x: u32)
    ensures seq_sum(s.push(x)) == seq_sum(s) + x,
{
    assert(s.push(x).drop_last() =~= s);
}

proof fn lemma_sum_nonneg(s: Seq<u32>)
    ensures seq_sum(s) >= 0,
    decreases s.len(),
{
    if s.len() > 0 { lemma_sum_nonneg(s.drop_last()); }
}

proof fn lemma_sum_split(s: Seq<u32>, i: int)
    requires 0 <= i <= s.len(),
    ensures seq_sum(s) == seq_sum(s.subrange(0, i)) + seq_sum(s.subrange(i, s.len() as int)),
    decreases s.len(),
{
    if i == s.len() {
        assert(s.subrange(0, i) =~= s);
        assert(s.subrange(i, s.len() as int).len() == 0);
    } else {
        let t = s.drop_last();
        lemma_sum_split(t, i);
        assert(t.subrange(0, i) =~= s.subrange(0, i));
        assert(s.subrange(i, s.len() as int).drop_last() =~= t.subrange(i, t.len() as int));
    }
}

proof fn lemma_sum_zeros(s: Seq<u32>)
    requires forall|j: int| 0 <= j < s.len() ==> s[j] == 0,
    ensures seq_sum(s) == 0,
    decreases s.len(),
{
    if s.len() > 0 { lemma_sum_zeros(s.drop_last()); }
}

/// The sum of a non-empty sequence is its first element plus the rest.
proof fn lemma_sum_first(s: Seq<u32>)
    requires s.len() > 0,
    ensures seq_sum(s) == s[0] + seq_sum(s.subrange(1, s.len() as int)),
{
    lemma_sum_split(s, 1);
    let first = s.subrange(0, 1);
    assert(first.drop_last() =~= Seq::<u32>::empty());
    assert(seq_sum(Seq::<u32>::empty()) == 0);
    assert(seq_sum(first) == seq_sum(first.drop_last()) + first.last());
}

/// The rules: `amounts` (one per blocker, in damage assignment order) and
/// `overflow` (what tramples over to what is being attacked) are a legal
/// division of `power` among blockers with the given `lethal` damage.
///
/// - Every point of damage is assigned (CR 510.1c), and none at all when no
///   blocker is left and the attacker has no trample (CR 510.1c: a blocked
///   creature with no blockers assigns no combat damage; CR 702.19e: a
///   trampler with none assigns it all to what it attacked).
/// - A blocker is assigned damage only if every blocker before it in the
///   order is assigned lethal damage (CR 510.1c).
/// - Damage goes past the blockers only from a trampler, and only once every
///   blocker is assigned lethal damage (CR 510.1c-d, 702.19b).
pub open spec fn legal_division(power: u32, lethal: Seq<u32>, trample: bool, amounts: Seq<u32>, overflow: u32) -> bool {
    &&& amounts.len() == lethal.len()
    &&& seq_sum(amounts) + overflow == if lethal.len() == 0 && !trample { 0 } else { power as int }
    &&& forall|i: int, j: int| 0 <= i < j < amounts.len() && amounts[j] > 0 ==> amounts[i] >= lethal[i]
    &&& overflow > 0 ==> trample && forall|i: int| 0 <= i < amounts.len() ==> amounts[i] >= lethal[i]
}

/// Three things any legal division says about blocker `i`, given what the
/// blockers before it were assigned: it gets at most what is left; if it
/// gets less than lethal it gets everything left; and the last blocker of a
/// non-trampler gets everything left.
proof fn lemma_legal_at(power: u32, lethal: Seq<u32>, trample: bool, a: Seq<u32>, o: u32, i: int)
    requires
        legal_division(power, lethal, trample, a, o),
        0 <= i < a.len(),
    ensures
        a[i] <= power - seq_sum(a.subrange(0, i)),
        a[i] < lethal[i] ==> a[i] == power - seq_sum(a.subrange(0, i)),
        i == a.len() - 1 && !trample ==> a[i] == power - seq_sum(a.subrange(0, i)),
{
    let n = a.len() as int;
    let rest = a.subrange(i, n);
    lemma_sum_split(a, i);
    lemma_sum_first(rest);
    let after = rest.subrange(1, rest.len() as int);
    assert(after =~= a.subrange(i + 1, n));
    lemma_sum_nonneg(after);
    if a[i] < lethal[i] {
        assert forall|j: int| 0 <= j < after.len() implies after[j] == 0 by {
            assert(after[j] == a[i + 1 + j]);
        }
        lemma_sum_zeros(after);
    }
    if i == n - 1 && !trample {
        assert(after.len() == 0);
    }
}

/// Every answer the attacking player has given so far is the one `amounts`
/// gives that blocker.
pub open spec fn agrees(chosen: Seq<Option<u32>>, amounts: Seq<u32>) -> bool {
    forall|i: int| 0 <= i < chosen.len() ==> (chosen[i] is None || chosen[i] == Some(amounts[i]))
}

/// One attacker's combat damage divided among its blockers.
pub struct Division {
    /// What each blocker is assigned, in damage assignment order.
    pub amounts: Vec<u32>,
    /// What tramples over to what is being attacked; 0 without trample.
    pub overflow: u32,
    /// The first blocker whose amount is still the attacking player's to
    /// choose, by position, with the least and most it may be assigned.
    /// `amounts` gives it, and every blocker after it, the default.
    pub open: Option<(usize, u32, u32)>,
}

/// Divide `power` among blockers whose lethal damage is `lethal`, in damage
/// assignment order, taking the attacking player's answers in `chosen` (one
/// per blocker, None where they have not answered) as CR 510.1c-d allows.
///
/// A blocker is the attacking player's to decide whenever what is left is
/// more than its lethal damage, unless it is the last blocker of an
/// attacker without trample, which takes everything left: the player may
/// then assign it anything from lethal to everything left. Otherwise it is
/// assigned what is left up to lethal, and that is the only legal amount.
/// The default for an open blocker, and every one after it, is exactly
/// lethal and the rest forward.
///
/// Proved, for every power, every number of blockers and every lethal
/// amount:
/// - whatever the answers, the division is legal ([`legal_division`]);
/// - a question is a choice: an open blocker has at least two amounts;
/// - every legal division can be reached: if each answer so far agrees
///   with it, either nothing is left to ask and the division is it, or the
///   next question's range contains its amount for that blocker and
///   everything before that blocker already matches. So answering each
///   question with that division's amount arrives at exactly it.
#[must_use]
pub fn divide(power: u32, lethal: &[u32], trample: bool, chosen: &[Option<u32>]) -> (d: Division)
    requires
        chosen.len() == lethal.len(),
    ensures
        legal_division(power, lethal@, trample, d.amounts@, d.overflow),
        d.open matches Some((k, lo, hi)) ==> k < lethal.len() && chosen@[k as int] is None && lo < hi,
        forall|a: Seq<u32>, o: u32| #[trigger] legal_division(power, lethal@, trample, a, o) && agrees(chosen@, a) ==>
            match d.open {
                None => d.amounts@ == a && d.overflow == o,
                Some((k, lo, hi)) => lo <= a[k as int] <= hi && d.amounts@.subrange(0, k as int) == a.subrange(0, k as int),
            },
{
    let n = lethal.len();
    let mut remaining = power;
    let mut amounts: Vec<u32> = Vec::new();
    let mut open: Option<(usize, u32, u32)> = None;
    let mut i: usize = 0;
    while i < n
        invariant
            n == lethal.len(),
            chosen.len() == n,
            i <= n,
            amounts.len() == i,
            remaining == power - seq_sum(amounts@),
            forall|p: int, q: int| 0 <= p < q < i && amounts@[q] > 0 ==> amounts@[p] >= lethal@[p],
            remaining > 0 ==> forall|p: int| 0 <= p < i ==> amounts@[p] >= lethal@[p],
            i == n && n > 0 && !trample ==> remaining == 0,
            open matches Some((k, lo, hi)) ==> k < i && chosen@[k as int] is None && lo < hi
                && amounts@[k as int] == lo && lo == lethal@[k as int]
                && hi == power - seq_sum(amounts@.subrange(0, k as int)),
            forall|a: Seq<u32>, o: u32| #[trigger] legal_division(power, lethal@, trample, a, o) && agrees(chosen@, a) ==>
                match open {
                    None => amounts@ == a.subrange(0, i as int),
                    Some((k, lo, hi)) => lo <= a[k as int] <= hi && amounts@.subrange(0, k as int) == a.subrange(0, k as int),
                },
        decreases n - i,
    {
        let ghost before = amounts@;
        let ghost open_before = open;
        let ghost remaining_before = remaining;
        let lethal_here = lethal[i];
        let least = remaining.min(lethal_here);
        let amount = if i + 1 == n && !trample {
            remaining
        } else if remaining == least {
            least
        } else if let Some(c) = chosen[i] {
            c.clamp(least, remaining)
        } else {
            if open.is_none() {
                open = Some((i, least, remaining));
            }
            least
        };
        proof {
            lemma_sum_push(amounts@, amount);
        }
        amounts.push(amount);
        remaining -= amount;
        proof {
            assert(amounts@.subrange(0, i as int) =~= before);
            assert forall|k: int| 0 <= k <= i implies #[trigger] amounts@.subrange(0, k) == before.subrange(0, k) by {
                assert(amounts@.subrange(0, k) =~= before.subrange(0, k));
            }
            assert forall|a: Seq<u32>, o: u32| #[trigger] legal_division(power, lethal@, trample, a, o) && agrees(chosen@, a) implies
                (match open {
                    None => amounts@ == a.subrange(0, i + 1),
                    Some((k, lo, hi)) => lo <= a[k as int] <= hi && amounts@.subrange(0, k as int) == a.subrange(0, k as int),
                }) by {
                if open_before is None {
                    assert(before == a.subrange(0, i as int));
                    lemma_legal_at(power, lethal@, trample, a, o, i as int);
                    assert(remaining_before == power - seq_sum(before));
                    if open is None {
                        assert(amounts@ =~= a.subrange(0, i + 1));
                    } else {
                        assert(open == Some((i, least, remaining_before)));
                        assert(least == lethal@[i as int] && least < remaining_before);
                        assert(a[i as int] >= lethal@[i as int]);
                    }
                } else {
                    assert(open == open_before);
                    let (k, lo, hi) = open_before.unwrap();
                    assert(amounts@.subrange(0, k as int) == before.subrange(0, k as int));
                }
            }
        }
        i += 1;
    }
    proof {
        assert(amounts@.subrange(0, n as int) =~= amounts@);
        assert forall|a: Seq<u32>, o: u32| #[trigger] legal_division(power, lethal@, trample, a, o) && agrees(chosen@, a) && open is None
            implies (trample ==> remaining == o) && (!trample ==> o == 0) by {
            assert(a.subrange(0, n as int) =~= a);
        }
    }
    let overflow = if trample { remaining } else { 0 };
    Division { amounts, overflow, open }
}

} // verus!
