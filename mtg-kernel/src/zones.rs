//! The order a player's zone is listed in, proved against CR 400.3 and
//! 404.2 for every board.
//!
//! The engine keeps every object in one map, in id order, and lists a zone
//! by picking out the objects in it (which objects those are is CR 400.3's:
//! a library, hand or graveyard by owner, the battlefield by controller).
//! A graveyard is an ordered pile (404.2), whose order is the order its
//! cards arrived in, which the engine records as each one arrives. So
//! [`zone_order`] lists a graveyard in the order of its pile, and whatever
//! the pile does not list after it, in the objects' own order; every other
//! zone is given no pile, and so is listed in the objects' own order.

use vstd::prelude::*;

verus! {

/// Where `id` first appears among the first `n` entries of `pile`, or `n`
/// if it is not among them.
pub open spec fn first_in(pile: Seq<u64>, id: u64, n: int) -> int
    decreases n,
{
    if n <= 0 {
        0
    } else if first_in(pile, id, n - 1) < n - 1 {
        first_in(pile, id, n - 1)
    } else if pile[n - 1] == id {
        n - 1
    } else {
        n
    }
}

/// Where `id` first appears in `pile`, or `pile.len()`, past every entry,
/// if it does not appear: a card the pile does not list is listed after
/// every card it does.
pub open spec fn place(pile: Seq<u64>, id: u64) -> int {
    first_in(pile, id, pile.len() as int)
}

/// Whether the object at position `k` is listed before the one at `l`: by
/// place in the pile, and at the same place (which two objects share only
/// when the pile lists neither) by their own order.
pub open spec fn before(ids: Seq<u64>, pile: Seq<u64>, k: int, l: int) -> bool {
    place(pile, ids[k]) < place(pile, ids[l]) || (place(pile, ids[k]) == place(pile, ids[l]) && k < l)
}

/// No two objects share an id.
pub open spec fn distinct(ids: Seq<u64>) -> bool {
    forall|a: int, b: int| 0 <= a < b < ids.len() ==> ids[a] != ids[b]
}

/// `first_in` is where `id` first appears among the first `n` entries, or
/// `n`.
proof fn lemma_first_in(pile: Seq<u64>, id: u64, n: int)
    requires 0 <= n <= pile.len(),
    ensures
        0 <= first_in(pile, id, n) <= n,
        first_in(pile, id, n) < n ==> pile[first_in(pile, id, n)] == id,
        forall|q: int| 0 <= q < first_in(pile, id, n) ==> pile[q] != id,
    decreases n,
{
    if n > 0 {
        lemma_first_in(pile, id, n - 1);
    }
}

/// So the place of an id is where it first appears, or past the pile when
/// no entry is that id.
proof fn lemma_place(pile: Seq<u64>, id: u64)
    ensures
        0 <= place(pile, id) <= pile.len(),
        place(pile, id) < pile.len() ==> pile[place(pile, id)] == id,
        forall|q: int| 0 <= q < place(pile, id) ==> pile[q] != id,
{
    lemma_first_in(pile, id, pile.len() as int);
}

/// The place of `pile[i]` is `i` when no entry before it is the same id.
proof fn lemma_first_here(pile: Seq<u64>, i: int)
    requires
        0 <= i < pile.len(),
        forall|q: int| 0 <= q < i ==> pile[q] != pile[i],
    ensures place(pile, pile[i]) == i,
{
    lemma_place(pile, pile[i]);
}

/// The place of an id no entry is, is past the pile.
proof fn lemma_unlisted(pile: Seq<u64>, id: u64)
    requires forall|q: int| 0 <= q < pile.len() ==> pile[q] != id,
    ensures place(pile, id) == pile.len(),
{
    lemma_place(pile, id);
}

proof fn lemma_push_keeps(s: Seq<usize>, x: usize, y: usize)
    requires s.contains(y),
    ensures s.push(x).contains(y),
{
    let a = choose|a: int| 0 <= a < s.len() && s[a] == y;
    assert(s.push(x)[a] == y);
}

proof fn lemma_push_has(s: Seq<usize>, x: usize)
    ensures s.push(x).contains(x),
{
    assert(s.push(x)[s.len() as int] == x);
}

/// Whether `pile[i]` is the same id as an entry before it.
fn seen_before(pile: &Vec<u64>, i: usize) -> (seen: bool)
    requires i < pile.len(),
    ensures seen <==> exists|q: int| 0 <= q < i && pile@[q] == pile@[i as int],
{
    let mut j: usize = 0;
    while j < i
        invariant
            i < pile.len(), j <= i,
            forall|q: int| 0 <= q < j ==> pile@[q] != pile@[i as int],
        decreases i - j,
    {
        if pile[j] == pile[i] {
            return true;
        }
        j += 1;
    }
    false
}

/// Whether `id` is an entry of `pile`.
fn in_pile(pile: &Vec<u64>, id: u64) -> (listed: bool)
    ensures listed <==> exists|q: int| 0 <= q < pile.len() && pile@[q] == id,
{
    let mut j: usize = 0;
    while j < pile.len()
        invariant
            j <= pile.len(),
            forall|q: int| 0 <= q < j ==> pile@[q] != id,
        decreases pile.len() - j,
    {
        if pile[j] == id {
            return true;
        }
        j += 1;
    }
    false
}

/// The position of the member whose id is `id`, if there is one.
fn member_with(ids: &Vec<u64>, members: &Vec<bool>, id: u64) -> (k: Option<usize>)
    requires ids.len() == members.len(),
    ensures
        k matches Some(k) ==> k < ids.len() && ids@[k as int] == id && members@[k as int],
        k is None ==> forall|q: int| 0 <= q < ids.len() ==> !(ids@[q] == id && members@[q]),
{
    let mut k: usize = 0;
    while k < ids.len()
        invariant
            ids.len() == members.len(), k <= ids.len(),
            forall|q: int| 0 <= q < k ==> !(ids@[q] == id && members@[q]),
        decreases ids.len() - k,
    {
        if ids[k] == id && members[k] {
            return Some(k);
        }
        k += 1;
    }
    None
}

/// The positions in `ids` of the objects for which `members` holds, in
/// the order the zone is listed: by where each first appears in `pile`, and
/// those `pile` does not list after, in their order in `ids`.
///
/// - Every position listed is a member's.
/// - Every member's position is listed.
/// - Each is listed before every one after it in the order above, so each
///   once, and the listing is the one order there is.
pub fn zone_order(ids: &Vec<u64>, members: &Vec<bool>, pile: &Vec<u64>) -> (order: Vec<usize>)
    requires
        ids.len() == members.len(),
        distinct(ids@),
    ensures
        forall|a: int| 0 <= a < order.len() ==> (#[trigger] order@[a] as int) < ids.len() && members@[order@[a] as int],
        forall|k: int| 0 <= k < ids.len() && members@[k] ==> order@.contains(k as usize),
        forall|a: int, b: int| 0 <= a < b < order.len()
            ==> #[trigger] before(ids@, pile@, order@[a] as int, order@[b] as int) && order@[a] != order@[b],
{
    let n = ids.len();
    let m = pile.len();
    let mut order: Vec<usize> = Vec::new();
    proof {
        assert forall|k: int| 0 <= k < n implies place(pile@, ids@[k]) >= 0 by {
            lemma_place(pile@, ids@[k]);
        }
    }

    // First the members the pile lists, each at its first place in it.
    let mut i: usize = 0;
    while i < m
        invariant
            n == ids.len(), n == members.len(), m == pile.len(), distinct(ids@),
            i <= m,
            forall|a: int| 0 <= a < order.len() ==> (#[trigger] order@[a] as int) < n && members@[order@[a] as int]
                && place(pile@, ids@[order@[a] as int]) < i,
            forall|k: int| 0 <= k < n && members@[k] && place(pile@, ids@[k]) < i ==> order@.contains(k as usize),
            forall|a: int, b: int| 0 <= a < b < order.len()
                ==> place(pile@, ids@[#[trigger] order@[a] as int]) < place(pile@, ids@[#[trigger] order@[b] as int]),
        decreases m - i,
    {
        let id = pile[i];
        let ghost old = order@;
        if !seen_before(pile, i) {
            proof {
                lemma_first_here(pile@, i as int);
            }
            match member_with(ids, members, id) {
                Some(k) => {
                    order.push(k);
                    proof {
                        assert(order@ == old.push(k));
                        assert forall|q: int| 0 <= q < n && members@[q] && place(pile@, ids@[q]) < i + 1
                            implies order@.contains(q as usize) by {
                            lemma_place(pile@, ids@[q]);
                            if place(pile@, ids@[q]) == i {
                                assert(ids@[q] == ids@[k as int]);
                                assert(q == k as int);
                                lemma_push_has(old, k);
                            } else {
                                lemma_push_keeps(old, k, q as usize);
                            }
                        }
                    }
                },
                None => {
                    proof {
                        assert forall|q: int| 0 <= q < n && members@[q] && place(pile@, ids@[q]) < i + 1
                            implies order@.contains(q as usize) by {
                            lemma_place(pile@, ids@[q]);
                            if place(pile@, ids@[q]) == i {
                                assert(ids@[q] == id);
                            }
                        }
                    }
                },
            }
        } else {
            proof {
                assert forall|q: int| 0 <= q < n && members@[q] && place(pile@, ids@[q]) < i + 1
                    implies order@.contains(q as usize) by {
                    lemma_place(pile@, ids@[q]);
                    if place(pile@, ids@[q]) == i {
                        let r = choose|r: int| 0 <= r < i && pile@[r] == pile@[i as int];
                        assert(pile@[r] == ids@[q]);
                    }
                }
            }
        }
        i += 1;
    }

    // Then the members the pile does not list, in their own order.
    let mut k: usize = 0;
    while k < n
        invariant
            n == ids.len(), n == members.len(), m == pile.len(), distinct(ids@),
            k <= n,
            forall|a: int| 0 <= a < order.len() ==> (#[trigger] order@[a] as int) < n && members@[order@[a] as int]
                && (place(pile@, ids@[order@[a] as int]) < m || (order@[a] as int) < k),
            forall|q: int| 0 <= q < n && members@[q] && (place(pile@, ids@[q]) < m || q < k) ==> order@.contains(q as usize),
            forall|a: int, b: int| 0 <= a < b < order.len()
                ==> #[trigger] before(ids@, pile@, order@[a] as int, order@[b] as int) && order@[a] != order@[b],
        decreases n - k,
    {
        let id = ids[k];
        let ghost old = order@;
        let listed = in_pile(pile, id);
        if members[k] && !listed {
            proof {
                lemma_unlisted(pile@, id);
                assert forall|a: int| 0 <= a < old.len()
                    implies #[trigger] before(ids@, pile@, old[a] as int, k as int) && old[a] != k by {
                    lemma_place(pile@, ids@[old[a] as int]);
                }
            }
            order.push(k);
            proof {
                assert(order@ == old.push(k));
                assert forall|a: int, b: int| 0 <= a < b < order.len()
                    implies #[trigger] before(ids@, pile@, order@[a] as int, order@[b] as int) && order@[a] != order@[b] by {
                    assert(order@[a] == old[a]);
                    if b < old.len() {
                        assert(order@[b] == old[b]);
                    } else {
                        assert(order@[b] == k);
                    }
                }
                assert forall|q: int| 0 <= q < n && members@[q] && (place(pile@, ids@[q]) < m || q < k + 1)
                    implies order@.contains(q as usize) by {
                    if q == k as int && !(place(pile@, ids@[q]) < m) {
                        lemma_push_has(old, k);
                    } else {
                        lemma_push_keeps(old, k, q as usize);
                    }
                }
            }
        } else {
            proof {
                assert forall|q: int| 0 <= q < n && members@[q] && (place(pile@, ids@[q]) < m || q < k + 1)
                    implies order@.contains(q as usize) by {
                    if q == k as int && !(place(pile@, ids@[q]) < m) {
                        lemma_place(pile@, id);
                        let r = choose|r: int| 0 <= r < m && pile@[r] == id;
                        assert(false);
                    }
                }
                assert(order@ == old);
            }
        }
        proof {
            assert forall|a: int, b: int| 0 <= a < b < order.len()
                implies #[trigger] before(ids@, pile@, order@[a] as int, order@[b] as int) && order@[a] != order@[b] by {
                if b < old.len() {
                    assert(order@[a] == old[a] && order@[b] == old[b]);
                    assert(before(ids@, pile@, old[a] as int, old[b] as int));
                }
            }
        }
        k += 1;
    }
    order
}

/// A pile whose entries are in the objects' own order, and which lists
/// every member it lists ahead of every member it does not (as a pile does
/// that is the first few members, in order): then the zone is listed in
/// the objects' own order. Every board a card proof builds has such piles
/// (`mtg-engine`'s `proofs::cards` puts each card on its graveyard's pile
/// as it makes it), and a card proof lists a zone in id order, asserting
/// that its piles are such, rather than running `zone_order`, which CBMC
/// does not get through.
pub proof fn lemma_a_pile_in_order_lists_in_order(ids: Seq<u64>, members: Seq<bool>, pile: Seq<u64>, order: Seq<usize>)
    requires
        ids.len() == members.len(),
        distinct(ids),
        // What `zone_order` ensures.
        forall|a: int, b: int| 0 <= a < b < order.len()
            ==> #[trigger] before(ids, pile, order[a] as int, order[b] as int),
        forall|a: int| 0 <= a < order.len() ==> (#[trigger] order[a] as int) < ids.len() && members[order[a] as int],
        // The pile: its entries in the objects' order, and every member it
        // does not list after every one it does.
        forall|p: int, q: int, k: int, l: int| 0 <= p < q < pile.len() && 0 <= k < ids.len() && 0 <= l < ids.len()
            && pile[p] == ids[k] && pile[q] == ids[l] ==> k < l,
        forall|k: int, l: int| 0 <= k < ids.len() && 0 <= l < ids.len() && members[k] && members[l]
            && place(pile, ids[k]) == pile.len() && place(pile, ids[l]) < pile.len() ==> l < k,
    ensures
        forall|a: int, b: int| 0 <= a < b < order.len() ==> order[a] < order[b],
{
    assert forall|a: int, b: int| 0 <= a < b < order.len() implies order[a] < order[b] by {
        let (k, l) = (order[a] as int, order[b] as int);
        assert(before(ids, pile, k, l));
        lemma_place(pile, ids[k]);
        lemma_place(pile, ids[l]);
        let (pk, pl) = (place(pile, ids[k]), place(pile, ids[l]));
        if pk < pile.len() && pl < pile.len() {
            assert(pile[pk] == ids[k] && pile[pl] == ids[l]);
            if pk < pl {
                assert(k < l);
            } else {
                assert(pk == pl);
                assert(ids[k] == ids[l]);
                if k != l {
                    if k < l { assert(ids[k] != ids[l]); } else { assert(ids[l] != ids[k]); }
                }
            }
        } else if pk < pile.len() {
            // `l` is not listed, `k` is: the pile's last condition puts
            // `l` after `k`.
            assert(pl == pile.len());
            assert(members[k] && members[l]);
            assert(k < l);
        } else if pl < pile.len() {
            assert(pk == pile.len());
            assert(l < k);
            assert(false);
        }
    }
}

} // verus!
