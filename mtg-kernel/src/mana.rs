//! Paying a mana cost out of a mana pool, proved against CR 107.4 and
//! 601.2h for every pool, every cost and every reserve.
//!
//! A pool is six counts ([`Amounts`]), one per type of mana; `mtg-engine`
//! decides which slot is which (colorless, then white, blue, black, red and
//! green) and reads its `ManaPool` into this shape and back. A cost is the
//! same six counts of pips only one type can pay (a coloured pip, `{C}`)
//! plus a count of generic pips that any type pays ([`Pips`]); `{X}` is not
//! a cost until it is announced (CR 107.3e).
//!
//! The specification is [`payable`]: a pool can pay a cost exactly when it
//! holds enough of each type for that type's own pips, and enough in all
//! for every pip. Every pip but a generic one accepts exactly one type, so
//! this is Hall's condition for matching pips to mana, and it is all that
//! "can pay" means. [`paid_exactly`] says what a payment that succeeds owes:
//! every pip paid, each from its own type, and nothing more. And
//! [`payable_together`] and [`reserve_payable`] state what paying around a
//! reserve promises (issues #252, #678, #840): when the pool could pay the
//! cost and the reserve together, paying the cost leaves the reserve
//! payable.

use vstd::prelude::*;

verus! {

/// A pool's contents, or what a cost asks of each type in particular, as
/// six counts, one slot per type of mana.
pub type Amounts = [u32; 6];

/// A cost as payment sees it: what it asks of each type in particular, and
/// how much generic mana on top.
#[derive(Clone, Copy)]
pub struct Pips {
    pub specific: Amounts,
    pub generic: u32,
}

/// The slots in their own order: what [`pay_fixed`] spends generic mana in.
const FIXED_ORDER: [usize; 6] = [0, 1, 2, 3, 4, 5];

const NO_FLOOR: Amounts = [0, 0, 0, 0, 0, 0];

/// Everything in six counts.
pub open spec fn sum(a: Amounts) -> int {
    a[0] + a[1] + a[2] + a[3] + a[4] + a[5]
}

/// The rules: `have` can pay `cost` exactly when it holds enough of each
/// type for that type's own pips, and enough in all for every pip.
pub open spec fn payable(have: Amounts, cost: Pips) -> bool {
    &&& forall|i: int| 0 <= i < 6 ==> have[i] >= cost.specific[i]
    &&& sum(have) >= sum(cost.specific) + cost.generic
}

/// `left` is what paying `cost` out of `have` should leave: nothing was
/// added, each type gave at least its own pips, and exactly the cost's pips
/// were taken in all.
pub open spec fn paid_exactly(have: Amounts, left: Amounts, cost: Pips) -> bool {
    &&& forall|i: int| 0 <= i < 6 ==> left[i] <= have[i] && have[i] - left[i] >= cost.specific[i]
    &&& sum(have) - sum(left) == sum(cost.specific) + cost.generic
}

/// `have` can pay `cost` and a reserve (`keep` of each type in particular,
/// `keep_generic` on top) at the same time.
pub open spec fn payable_together(have: Amounts, cost: Pips, keep: Amounts, keep_generic: int) -> bool {
    &&& forall|i: int| 0 <= i < 6 ==> have[i] >= cost.specific[i] + keep[i]
    &&& sum(have) >= sum(cost.specific) + cost.generic + sum(keep) + keep_generic
}

/// `left` can still pay the reserve.
pub open spec fn reserve_payable(left: Amounts, keep: Amounts, keep_generic: int) -> bool {
    &&& forall|i: int| 0 <= i < 6 ==> left[i] >= keep[i]
    &&& sum(left) >= sum(keep) + keep_generic
}

/// `i` is one of the slots `order` names.
pub open spec fn appears(order: [usize; 6], i: int) -> bool {
    order[0] == i || order[1] == i || order[2] == i || order[3] == i || order[4] == i || order[5] == i
}

/// `order` names every slot exactly once.
pub open spec fn is_permutation(order: [usize; 6]) -> bool {
    &&& forall|p: int| 0 <= p < 6 ==> order[p] < 6
    &&& forall|p: int, q: int| 0 <= p < 6 && 0 <= q < 6 && p != q ==> order[p] != order[q]
    &&& forall|i: int| 0 <= i < 6 ==> #[trigger] appears(order, i)
}

/// Pay `cost` out of `have` with the generic part in slot order: what is
/// left, or None when the cost cannot be paid.
#[must_use]
pub fn pay_fixed(have: Amounts, cost: &Pips) -> (left: Option<Amounts>)
    ensures
        left.is_some() == payable(have, *cost),
        left.is_some() ==> paid_exactly(have, left.unwrap(), *cost),
{
    let mut left = pay_specific(have, cost)?;
    let mut remaining = cost.generic;
    spend_generic(&mut left, &mut remaining, &FIXED_ORDER, &NO_FLOOR);
    if remaining == 0 {
        Some(left)
    } else {
        assert(sum(left) == 0);
        None
    }
}

/// Pay `cost` out of `have` with the generic part spent around `keep`, what
/// another cost needs of each type: what is left, or None when the cost
/// cannot be paid. Whenever the pool could pay the cost and the reserve
/// together, what is left can still pay the reserve.
///
/// The spare is counted after the cost has paid its own pips, since those
/// come out first whatever the order: `{1}{W}` out of `{W}{W}{U}` beside a
/// `{W}` in hand pays its `{W}`, and then the other White is the hand's,
/// not spare. And generic takes only what is spare until nothing spare is
/// left: `{2}` out of the same pool takes one White and the Blue, where
/// taking every White first left the hand nothing to cast with. Which spare
/// goes first is [`surplus_order`]'s preference; the promise holds for any
/// order, so the proof asks only that it name every slot once.
#[must_use]
pub fn pay_around(have: Amounts, cost: &Pips, keep: &Amounts) -> (left: Option<Amounts>)
    ensures
        left.is_some() == payable(have, *cost),
        left.is_some() ==> paid_exactly(have, left.unwrap(), *cost),
        forall|g: int| 0 <= g && #[trigger] payable_together(have, *cost, *keep, g)
            ==> left.is_some() && reserve_payable(left.unwrap(), *keep, g),
{
    let mut left = pay_specific(have, cost)?;
    let order = surplus_order(&left, keep);
    let mut remaining = cost.generic;
    let ghost after_pips = left;
    spend_generic(&mut left, &mut remaining, &order, keep);
    let ghost after_spare = left;
    let ghost remaining_after_spare = remaining;
    spend_generic(&mut left, &mut remaining, &order, &NO_FLOOR);
    proof {
        // With the cost and the reserve payable together, the spare alone
        // covers the generic: the first pass finishes, and the second takes
        // nothing.
        if payable_together(have, *cost, *keep, 0) {
            if remaining_after_spare != 0 {
                assert forall|i: int| 0 <= i < 6 implies after_spare[i] == keep[i] by {
                    assert(after_pips[i] >= keep[i]);
                }
                assert(sum(after_spare) == sum(*keep));
                assert(false);
            }
            assert forall|i: int| 0 <= i < 6 implies left[i] == after_spare[i] by {}
            assert forall|i: int| 0 <= i < 6 implies left[i] >= keep[i] by {
                assert(after_pips[i] >= keep[i]);
            }
        }
        assert forall|g: int| 0 <= g && #[trigger] payable_together(have, *cost, *keep, g)
            implies payable_together(have, *cost, *keep, 0) by {}
    }
    if remaining == 0 {
        Some(left)
    } else {
        assert(sum(left) == 0);
        None
    }
}

/// The slots of `have`, biggest surplus over `keep` first, ties in slot
/// order: which spare mana a generic cost spends first, so that what another
/// cost depends on most goes last. A stable insertion sort, written out
/// rather than `sort_by_key` so that it can be proved to return every slot
/// exactly once.
#[must_use]
pub fn surplus_order(have: &Amounts, keep: &Amounts) -> (order: [usize; 6])
    ensures
        is_permutation(order),
{
    let mut order: [usize; 6] = FIXED_ORDER;
    assert(appears(order, 0) && appears(order, 1) && appears(order, 2));
    assert(appears(order, 3) && appears(order, 4) && appears(order, 5));
    let mut i: usize = 1;
    while i < 6
        invariant
            1 <= i <= 6,
            is_permutation(order),
        decreases 6 - i,
    {
        let mut j: usize = i;
        while j > 0 && surplus(have, keep, order[j - 1]) < surplus(have, keep, order[j])
            invariant
                j <= i < 6,
                is_permutation(order),
            decreases j,
        {
            let ghost before = order;
            let (earlier, later) = (order[j - 1], order[j]);
            order[j - 1] = later;
            order[j] = earlier;
            proof {
                assert forall|x: int| 0 <= x < 6 implies #[trigger] appears(order, x) by {
                    assert(appears(before, x));
                }
            }
            j -= 1;
        }
        i += 1;
    }
    order
}

fn surplus(have: &Amounts, keep: &Amounts, slot: usize) -> i64
    requires
        slot < 6,
{
    i64::from(have[slot]) - i64::from(keep[slot])
}

/// Each type's own pips from that type: what is left, or None when some
/// type is short.
fn pay_specific(have: Amounts, cost: &Pips) -> (left: Option<Amounts>)
    ensures
        left.is_some() == (forall|i: int| 0 <= i < 6 ==> have[i] >= cost.specific[i]),
        left.is_some() ==> forall|i: int| 0 <= i < 6 ==> left.unwrap()[i] == have[i] - cost.specific[i],
{
    let mut left = have;
    let mut i: usize = 0;
    while i < 6
        invariant
            i <= 6,
            forall|j: int| 0 <= j < i ==> (#[trigger] have[j]) >= cost.specific[j],
            forall|j: int| 0 <= j < i ==> (#[trigger] left[j]) == have[j] - cost.specific[j],
            forall|j: int| i <= j < 6 ==> (#[trigger] left[j]) == have[j],
        decreases 6 - i,
    {
        assert(left[i as int] == have[i as int]);
        if left[i] < cost.specific[i] {
            assert(!(have[i as int] >= cost.specific[i as int]));
            return None;
        }
        left[i] -= cost.specific[i];
        assert(have[i as int] >= cost.specific[i as int]);
        i += 1;
    }
    Some(left)
}

/// Take up to `remaining` generic mana out of `have`, slot by slot in
/// `order`, never taking a slot below `floor`. What it took is what came off
/// `remaining`, and if it could not take all of it, every slot is at or
/// below its floor.
fn spend_generic(have: &mut Amounts, remaining: &mut u32, order: &[usize; 6], floor: &Amounts)
    requires
        is_permutation(*order),
    ensures
        forall|i: int| 0 <= i < 6 ==> final(have)[i] <= old(have)[i],
        forall|i: int| 0 <= i < 6 ==> final(have)[i] == old(have)[i] || final(have)[i] >= floor[i],
        sum(*old(have)) - sum(*final(have)) == *old(remaining) - *final(remaining),
        *final(remaining) <= *old(remaining),
        *final(remaining) == 0 || forall|i: int| 0 <= i < 6 ==> final(have)[i] <= floor[i],
{
    let ghost h0 = *have;
    let ghost r0 = *remaining;
    let mut k: usize = 0;
    while k < 6
        invariant
            k <= 6,
            is_permutation(*order),
            forall|i: int| 0 <= i < 6 ==> have[i] <= h0[i],
            forall|i: int| 0 <= i < 6 ==> have[i] == h0[i] || have[i] >= floor[i],
            sum(h0) - sum(*have) == r0 - *remaining,
            *remaining <= r0,
            forall|p: int| 0 <= p < k ==> *remaining == 0 || have[order[p] as int] <= floor[order[p] as int],
            forall|p: int| k <= p < 6 ==> have[order[p] as int] == h0[order[p] as int],
        decreases 6 - k,
    {
        let slot = order[k];
        let spare = have[slot].saturating_sub(floor[slot]);
        let take = spare.min(*remaining);
        have[slot] -= take;
        *remaining -= take;
        k += 1;
    }
    proof {
        if *remaining != 0 {
            // A full pass over a permutation visits every slot.
            assert forall|i: int| 0 <= i < 6 implies have[i] <= floor[i] by {
                assert(appears(*order, i));
                if order[0] == i {
                    assert(have[order[0] as int] <= floor[order[0] as int]);
                } else if order[1] == i {
                    assert(have[order[1] as int] <= floor[order[1] as int]);
                } else if order[2] == i {
                    assert(have[order[2] as int] <= floor[order[2] as int]);
                } else if order[3] == i {
                    assert(have[order[3] as int] <= floor[order[3] as int]);
                } else if order[4] == i {
                    assert(have[order[4] as int] <= floor[order[4] as int]);
                } else {
                    assert(have[order[5] as int] <= floor[order[5] as int]);
                }
            }
        }
    };
}

} // verus!

#[cfg(test)]
mod tests {
    use super::*;

    /// `surplus_order` is the stable descending sort by surplus that
    /// `sort_by_key` gave before it was written out to be proved: same
    /// order, ties in slot order, for every pool and reserve of up to two of
    /// each type.
    #[test]
    fn surplus_order_is_the_stable_sort_by_surplus() {
        let mut have = [0u32; 6];
        let mut keep = [0u32; 6];
        for code in 0..3u32.pow(12) {
            let mut c = code;
            for slot in 0..6 {
                have[slot] = c % 3;
                c /= 3;
                keep[slot] = c % 3;
                c /= 3;
            }
            let mut expected = [0, 1, 2, 3, 4, 5];
            expected.sort_by_key(|&slot| std::cmp::Reverse(i64::from(have[slot]) - i64::from(keep[slot])));
            assert_eq!(surplus_order(&have, &keep), expected, "have {have:?} keep {keep:?}");
        }
    }
}
