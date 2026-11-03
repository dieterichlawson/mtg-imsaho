//! A spell's additional cost paid (`engine::costs::pay_additional_cost`,
//! CR 601.2b, 601.2h, 118.8): the sacrifice of Altar's Reap and Infernal
//! Plunge, the creature cards Makeshift Mauler and Stitched Drake exile,
//! and the X cards Harvest Pyre exiles. Sacrificing and moving are their
//! own proofs, recorded here.

use crate::cards::{AdditionalCost, CardBehavior, CardData, CardRegistry};
use crate::ids::{ObjectId, PlayerId};
use crate::proofs::cards::{acted, asked_for, card_proof, were, Act, Blank};
use crate::state::GameState;
use crate::types::Zone;

/// The cost of each harness's spell: 0 none, 1 a sacrifice, 2 two
/// creature cards exiled, 3 X cards exiled.
static mut COST: u8 = 0;

/// A spell whose additional cost is the one [`COST`] names.
struct Costly;

impl CardBehavior for Costly {
    fn card_data(&self) -> CardData {
        // SAFETY: a proof runs on one thread, and COST is set before the card is registered.
        let additional_cost = match unsafe { *std::ptr::addr_of!(COST) } {
            0 => None,
            1 => Some(AdditionalCost::SacrificeCreature),
            2 => Some(AdditionalCost::ExileCreaturesFromGraveyard(2)),
            _ => Some(AdditionalCost::ExileXFromGraveyard),
        };
        CardData { name: "Costly".into(), additional_cost, ..CardData::default() }
    }
}

/// The spell (object 1, the first player's, on the stack) and three more
/// of the first player's objects: two cards in the graveyard (2 and 3)
/// and a creature on the battlefield (4).
fn board(cost: u8) -> (CardRegistry, GameState, crate::ids::CardId, [ObjectId; 4]) {
    // SAFETY: as in `Costly::card_data`, before the card is registered.
    unsafe { *std::ptr::addr_of_mut!(COST) = cost };
    let mut registry = CardRegistry::new();
    let card = registry.register(Box::new(Costly));
    let blank = registry.register(Box::new(Blank));
    let mut state = crate::proofs::cards::new_game();
    let ids = [
        state.create_object(card, PlayerId(0), Zone::Stack, None, None),
        state.create_object(blank, PlayerId(0), Zone::Graveyard, Some(1), Some(1)),
        state.create_object(blank, PlayerId(0), Zone::Graveyard, Some(1), Some(1)),
        state.create_object(blank, PlayerId(0), Zone::Battlefield, Some(1), Some(1)),
    ];
    (registry, state, card, ids)
}

card_proof! {
    /// A sacrifice named for the cast is made, as an additional cost
    /// (601.2h), once, whether or not the card's own cost is one (a cast
    /// offering it only where one is owed, `legal::casting`), and nothing
    /// else is paid for a card with no additional cost or a sacrifice. For
    /// a spell of no additional cost or a sacrifice, and the creature named
    /// or none.
    fn a_sacrifice_named_for_a_cast_is_made_once() {
        let cost = kani::any::<u8>() % 2;
        let (registry, mut state, card, ids) = board(cost);
        let named: bool = kani::any();

        crate::engine::costs::pay_additional_cost(&mut state, &registry, card, ids[0], PlayerId(0),
            named.then_some(ids[3]), None, &[]);

        // With none named for a sacrifice, the creature there is taken
        // (a cast built by hand: no offer names none).
        if named || cost == 1 {
            assert!(asked_for(&[Act::Sacrifice(ids[3])]));
        } else {
            assert!(acted() == 0);
        }
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// Creature cards exiled from the graveyard as an additional cost
    /// (Makeshift Mauler's two): those named, each exiled once, in the
    /// order named, and nothing else. For either order of the two.
    fn the_creature_cards_named_for_an_exile_cost_are_exiled_in_order() {
        let (registry, mut state, card, ids) = board(2);
        let order = if kani::any() { [ids[1], ids[2]] } else { [ids[2], ids[1]] };

        crate::engine::costs::pay_additional_cost(&mut state, &registry, card, ids[0], PlayerId(0), None, None, &order);

        assert!(acted() == 2 && were(0, 2, |i| Act::Move(order[i], Zone::Exile)));
        std::mem::forget((registry, state));
    }
}

card_proof! {
    /// X cards exiled from the graveyard as an additional cost (Harvest
    /// Pyre's): those named, each exiled once in the order named, or, with
    /// none named, the first X of the caster's other graveyard cards in the
    /// board's order (as many as there are); and their number is recorded
    /// on the spell as the X it resolves with. For none, one or both of the
    /// two named, or none named and an X of none to three.
    fn the_cards_named_for_an_exile_of_x_are_exiled_and_counted() {
        let (registry, mut state, card, ids) = board(3);
        let named = kani::any::<u8>() % 3;
        let x = u32::from(kani::any::<u8>() % 4);
        let chosen: &[ObjectId] = match named {
            0 => &[],
            1 => &ids[2..3],
            _ => &ids[1..3],
        };

        crate::engine::costs::pay_additional_cost(&mut state, &registry, card, ids[0], PlayerId(0), None, Some(x), chosen);

        let exiled: &[ObjectId] = if named == 0 { &ids[1..1 + x.min(2) as usize] } else { chosen };
        assert!(acted() == exiled.len() && were(0, exiled.len(), |i| Act::Move(exiled[i], Zone::Exile)));
        let count = state.get_object(ids[0]).expect("on the board").card_state.get(crate::cards::EXILE_COUNT).copied();
        assert!(count == Some(ObjectId(exiled.len() as u64)));
        std::mem::forget((registry, state));
    }
}
