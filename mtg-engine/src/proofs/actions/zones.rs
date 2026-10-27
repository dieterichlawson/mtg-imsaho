//! `state::zone_order`, which `GameState::objects_in_zone` lists by: what
//! a player's zone holds, and in what order (CR 400.3, 404.2).
//! `objects_in_zone` is the objects at the positions it gives, in its
//! order.
//!
//! The order is generic over what it orders, and reads of an object only
//! its id, zone, owner and controller (`state::Zoned`); a generic function
//! can do nothing else with one. So it is proved here on objects that are
//! those four things and nothing else, and what it does with them is what
//! it does with a `GameObject` read the same way, which `Zoned for
//! GameObject` is.
//!
//! The specification is the rule, not the algorithm: the listing holds
//! each object in the zone exactly once and nothing else; and its order is
//! by where each first appears in the pile, those the pile does not list
//! coming after, in id order (a graveyard's pile is its order, CR 404.2;
//! every other zone is listed with no pile, so in id order).
//!
//! The card proofs read `objects_in_zone` through
//! `proofs::cards::objects_in_zone_in_id_order`, which lists the zone in
//! id order and asserts that each graveyard's pile lists a first run of its
//! cards, in id order: by the specification, the order `objects_in_zone`
//! gives such a board, since then each card's place in the pile is its
//! place in id order.

use crate::ids::{ObjectId, PlayerId};
use crate::state::{zone_order, Zoned};
use crate::types::Zone;

/// An object that is what a listing reads of one, and nothing else.
#[derive(Clone, Copy)]
struct Tiny {
    id: ObjectId,
    zone: Zone,
    owner: PlayerId,
    controller: PlayerId,
}

impl Zoned for Tiny {
    fn id(&self) -> ObjectId { self.id }
    fn zone(&self) -> Zone { self.zone }
    fn owner(&self) -> PlayerId { self.owner }
    fn controller(&self) -> PlayerId { self.controller }
}

/// How many objects, and pile entries, the boards here have: no card
/// proof's board has more objects, which its stand-in for
/// `objects_in_zone` asserts.
pub(crate) const N: usize = 8;

fn any_zone() -> Zone {
    match kani::any::<u8>() % 5 {
        0 => Zone::Battlefield,
        1 => Zone::Graveyard,
        2 => Zone::Hand,
        3 => Zone::Library,
        _ => Zone::Exile,
    }
}

fn any_player() -> PlayerId {
    PlayerId(u8::from(kani::any::<bool>()))
}

/// Whether `o` is in `player`'s `zone`: by owner in a library, a hand or a
/// graveyard, by controller on the battlefield, and whoever's in any other
/// zone (CR 400.3, 108.4).
fn in_zone(o: &Tiny, zone: Zone, player: PlayerId) -> bool {
    o.zone == zone && match zone {
        Zone::Library | Zone::Hand | Zone::Graveyard => o.owner == player,
        Zone::Battlefield => o.controller == player,
        _ => true,
    }
}

/// Where `id` first appears in `pile`, or `pile.len()` if it does not.
fn place(pile: &[ObjectId], id: ObjectId) -> usize {
    pile.iter().position(|p| *p == id).unwrap_or(pile.len())
}

#[kani::proof]
#[kani::unwind(10)]
/// Over eight objects, ids 1 to 8 in id order, each in any of five
/// zones, owned and controlled by either player; for a graveyard, a pile of
/// eight entries, each any of those ids or 0, which no object has, repeats
/// allowed; and every zone and player: the listing holds each object in the
/// zone once and no other, and orders them by place in the pile and then
/// by id (with no pile, which is every zone but a graveyard, by id).
///
/// Eight objects stand for any fewer: an object not in the zone asked
/// about is listed nowhere and moves no other, so a board of fewer is one
/// of these whose last objects are elsewhere. And eight entries for any
/// pile up to eight long: an entry naming no object is passed over, so a
/// shorter pile is one of these with entries of 0.
fn a_zone_listing_holds_the_zone_in_pile_then_id_order() {
    let board: [Tiny; N] = std::array::from_fn(|i| Tiny {
        id: ObjectId(i as u64 + 1), zone: any_zone(), owner: any_player(), controller: any_player(),
    });
    let zone = any_zone();
    let player = any_player();
    // A pile for a graveyard alone, as `objects_in_zone` passes it.
    let entries: [ObjectId; N] = std::array::from_fn(|_| {
        let id: u64 = kani::any();
        kani::assume(id <= N as u64);
        ObjectId(id)
    });
    let pile: &[ObjectId] = if zone == Zone::Graveyard { &entries } else { &[] };
    let refs: [&Tiny; N] = std::array::from_fn(|i| &board[i]);
    let order = zone_order(&refs, zone, player, pile);

    // Each object in the zone exactly once, and nothing else.
    for (k, o) in board.iter().enumerate() {
        let times = order.iter().filter(|i| **i == k).count();
        assert!(times == usize::from(in_zone(o, zone, player)));
    }
    assert!(order.iter().all(|i| *i < N));
    // In order of place in the pile, then id.
    for w in order.windows(2) {
        let (a, b) = (&board[w[0]], &board[w[1]]);
        assert!((place(pile, a.id), a.id) < (place(pile, b.id), b.id));
    }
    kani::cover!(order.len() >= 2 && order[0] > order[1], "a graveyard listed out of id order");
    std::mem::forget(order);
}
