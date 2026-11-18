//! The London mulligan (`actions::mulligan`, `engine::advance_mulligan_phase`,
//! CR 103.5): who is asked to keep or mulligan, in what order, what a
//! mulligan and a keep do, and the cards put on the bottom after. Putting a
//! card into a library, shuffling and drawing stand in as recorded, each
//! its own proof (`library`, `simple_actions`).

use crate::proofs::held::Held;
use crate::cards::CardRegistry;
use crate::ids::PlayerId;
use crate::proofs::cards::{asked_for, card_proof, Act, Blank};
use crate::state::{AwaitingAction, GameState, LibraryPosition};
use crate::types::Zone;

/// How many times the mulligan phase was driven on.
static mut ADVANCED: Held<usize> = Held::new(0, (&raw const ADVANCED).cast());

#[allow(dead_code)] // named only by `kani::stub`
fn advanced(_: &mut GameState, _: &CardRegistry) {
    // SAFETY: a proof runs on one thread, and only this function writes ADVANCED.
    unsafe { *std::ptr::addr_of_mut!(ADVANCED.value) += 1 };
}

card_proof! {
    #[kani::stub(crate::engine::advance_mulligan_phase, advanced)]
    /// Keeping a hand (CR 103.5): the player asked has kept, with as many
    /// cards to put on the bottom as mulligans they took, but never more
    /// than the seven drawn, owed after every player has kept; the next
    /// player in the round is asked; and the passes in a row start again.
    /// For either player asked, after 0 to 8 mulligans.
    fn keeping_a_hand_owes_a_card_to_the_bottom_for_each_mulligan_taken() {
        let registry = CardRegistry::new();
        let mut state = crate::proofs::cards::new_game();
        let player = PlayerId(u8::from(kani::any::<bool>()));
        let taken: u32 = kani::any();
        kani::assume(taken <= 8);
        state.get_player_mut(player).mulligan_count = taken;
        state.awaiting_action = Some(AwaitingAction::MulliganDecision { player });
        state.mulligan_round_position = 0;
        state.consecutive_passes = 1;

        let applied = crate::engine::actions::mulligan::mulligan_keep(&mut state, &registry);

        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(state.get_player(player).mulligan_kept && state.get_player(player).mulligan_count == taken);
        assert!(state.pending_mulligan_bottoms.as_slice() == [(player, (taken as usize).min(7))]);
        // SAFETY: as in `advanced`.
        assert!(state.mulligan_round_position == 1 && unsafe { *std::ptr::addr_of!(ADVANCED.value) } == 1);
        assert!(state.consecutive_passes == 0);
        std::mem::forget((registry, state));
    }
}

card_proof! {
    #[kani::stub(crate::engine::advance_mulligan_phase, advanced)]
    /// Taking a mulligan (CR 103.5): the player's hand goes into their
    /// library, each card, the library is shuffled, and they draw seven
    /// cards; they have taken one more mulligan; the round has had one, so
    /// another round follows it; and the next player in the round is asked.
    /// For either player asked, with two cards in hand, after 0 to 2
    /// mulligans.
    fn a_mulligan_shuffles_the_hand_away_and_draws_seven() {
        let mut registry = CardRegistry::new();
        let blank = registry.register(Box::new(Blank));
        let mut state = crate::proofs::cards::new_game();
        let player = PlayerId(u8::from(kani::any::<bool>()));
        let hand = [
            state.create_object(blank, player, Zone::Hand, None, None),
            state.create_object(blank, player, Zone::Hand, None, None),
        ];
        let taken: u32 = kani::any();
        kani::assume(taken <= 2);
        state.get_player_mut(player).mulligan_count = taken;
        state.awaiting_action = Some(AwaitingAction::MulliganDecision { player });
        state.mulligan_round_position = 0;
        state.mulligan_round_mulled = false;

        let applied = crate::engine::actions::mulligan::mulligan_mull(&mut state, &registry);

        assert!(matches!(applied, crate::engine::Applied::Continue));
        assert!(asked_for(&[
            Act::ToLibrary(hand[0], LibraryPosition::Bottom),
            Act::ToLibrary(hand[1], LibraryPosition::Bottom),
            Act::Shuffle(player),
            Act::Draw(player, 7),
        ]));
        assert!(state.get_player(player).mulligan_count == taken + 1 && !state.get_player(player).mulligan_kept);
        assert!(state.mulligan_round_mulled && state.mulligan_round_position == 1);
        // SAFETY: as in `advanced`.
        assert!(unsafe { *std::ptr::addr_of!(ADVANCED.value) } == 1);
        std::mem::forget((registry, state, hand));
    }
}

card_proof! {
    /// The mulligan phase driven on (`advance_mulligan_phase`, CR 103.5):
    /// of the players in turn order from the starting player, from where
    /// the round has got to, the first who has not kept is asked to keep
    /// or mulligan; with none left in the round, a round in which someone
    /// took a mulligan is followed by another from the starting player;
    /// and once every player has kept, each owing cards to the bottom is
    /// asked to put them there, in the order they kept, those owing none
    /// passed over; then the phase is over and nothing is asked. For the
    /// starting player either player, each player kept or not, the round
    /// at either position or past both, someone having taken a mulligan in
    /// it or not, and up to two owings of 0 to 2 cards.
    fn the_mulligan_phase_asks_each_player_in_turn_then_has_each_bottom_their_cards() {
        let registry = CardRegistry::new();
        let mut state = crate::proofs::cards::new_game();
        let active = PlayerId(u8::from(kani::any::<bool>()));
        state.active_player = active;
        let kept: [bool; 2] = [kani::any(), kani::any()];
        state.players[0].mulligan_kept = kept[0];
        state.players[1].mulligan_kept = kept[1];
        let position: u8 = kani::any();
        kani::assume(position <= 2);
        state.mulligan_round_position = position;
        let mulled: bool = kani::any();
        state.mulligan_round_mulled = mulled;
        let mut owed = [(PlayerId(0), 0usize); 2];
        let owings: usize = kani::any();
        kani::assume(owings <= 2);
        state.pending_mulligan_bottoms = Vec::with_capacity(2);
        for o in owed.iter_mut().take(owings) {
            *o = (PlayerId(u8::from(kani::any::<bool>())), kani::any());
            kani::assume(o.1 <= 2);
            state.pending_mulligan_bottoms.push(*o);
        }

        crate::engine::advance_mulligan_phase(&mut state, &registry);

        let in_turn = |pos: u8| PlayerId((active.0 + pos) % 2);
        let first_from = |from: u8| (from..2).find(|&pos| !kept[usize::from(in_turn(pos).0)]);
        let asked = match first_from(position) {
            Some(pos) => Some(pos),
            None if mulled => first_from(0),
            None => None,
        };
        if let Some(pos) = asked {
            assert!(matches!(state.awaiting_action, Some(AwaitingAction::MulliganDecision { player }) if player == in_turn(pos)));
            assert!(state.mulligan_round_position == pos && state.pending_mulligan_bottoms.len() == owings);
        } else {
            match owed[..owings].iter().position(|&(_, n)| n > 0) {
                Some(k) => {
                    assert!(matches!(state.awaiting_action, Some(AwaitingAction::BottomAfterMulligan { player, count })
                        if player == owed[k].0 && count == owed[k].1));
                    assert!(state.pending_mulligan_bottoms.as_slice() == &owed[k + 1..owings]);
                }
                None => {
                    assert!(state.awaiting_action.is_none() && state.pending_mulligan_bottoms.is_empty());
                }
            }
        }
        std::mem::forget((registry, state));
    }
}
