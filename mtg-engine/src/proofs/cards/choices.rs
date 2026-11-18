//! The cards whose effect stops to ask a player something and goes on with
//! the answer: what each asks, of whom, and what it does with each answer.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Nevermore, {1}{W}{W} Enchantment: "As this enchantment enters, choose
//!   a nonland card name. Spells with the chosen name can't be cast."
//! - Caravan Vigil, {G} Sorcery: "Search your library for a basic land
//!   card, reveal it, put it into your hand, then shuffle. Morbid — You may
//!   put that card onto the battlefield instead of putting it into your
//!   hand if a creature died this turn."
//! - Frightful Delusion, {2}{U} Instant: "Counter target spell unless its
//!   controller pays {1}. That player discards a card."
//! - Divine Reckoning, {2}{W}{W} Sorcery: "Each player chooses a creature
//!   they control. Destroy the rest. Flashback {5}{W}{W}"
//!
//! An answer comes back through the engine's handler for the question
//! asked (`engine::actions::choices`), which hands it to the card; where
//! the handler does part of what the card says (paying, or not, for
//! "unless its controller pays"), it is run here with the card, and where
//! it only passes the answer on (a chosen target, to the card's
//! `resolve_card_effect`, through `engine::apply_pending_effect`), the
//! card's own hook is called with every answer the handler accepts.

use super::{acted, board, card_proof, case_by_case, Snapshot};
use crate::actions::{ResolvedChoice, Target};
use crate::cards::{CardBehavior, CardRegistry};
use crate::ids::{CardId, ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::{ContinuousEffect as E, Zone};

use crate::cards::isd;

fn any_player() -> PlayerId {
    PlayerId(u8::from(kani::any::<bool>()))
}

/// This snapshot, waiting on an answer or not.
fn asking(before: &Snapshot, asking: bool) -> Snapshot {
    Snapshot { asking, ..before.clone() }
}

/// Nevermore: its name is chosen as it enters (CR 614.12), which is no
/// trigger; its controller is asked to choose among every nonland card
/// name there is, in order; the name chosen is the one whose spells can't
/// be cast (the `PreventCastingNamed` effect it then has, which the
/// engine's casting rules read); an answer past the end of the names is
/// refused, and the question stands. Nothing else. Over a registry of a
/// nonland card, a land and Nevermore, Nevermore under player `you`
/// answering `index`, each a constant in its harness: with both the
/// solver's, the answer was every path through `resolve_choice` at once,
/// and the proof was still in symbolic execution after half an hour.
fn nevermore_names_a_nonland_card((you, index): (u8, usize)) {
    let nevermore = isd::nevermore::Nevermore;
    assert!(nevermore.chooses_as_it_enters() && !nevermore.has_etb_handler());
    let (mut registry, mut state, _blank, card) = board(Box::new(nevermore));
    registry.register(Box::new(isd::gavony_township::GavonyTownship));
    let you = PlayerId(you);
    let me = state.create_object(card, PlayerId(0), Zone::Battlefield, None, None);
    state.get_object_mut(me).expect("just created").controller = you;
    let before = Snapshot::of(&state);
    registry.get(card).expect("registered").on_enter_battlefield(&mut state, me, &[], &registry);
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player, source, choice: ResolutionChoiceKind::ChooseCardName { options, source_id, .. },
    }) if *player == you && *source == me && *source_id == me
        && options.len() == 2 && options[0] == "Blank" && options[1] == "Nevermore");
    assert!(asked && acted() == 0 && Snapshot::of(&state) == asking(&before, true));
    let _ = crate::engine::actions::choices::resolve_choice(&mut state, &ResolvedChoice::ChosenIndex(index, String::new()), &registry);
    let effects = state.get_object(me).expect("on the board").instance_continuous_effects.as_deref();
    if index < 2 {
        let name = if index == 0 { "Blank" } else { "Nevermore" };
        assert!(matches!(effects, Some([E::PreventCastingNamed { name: n }]) if n == name));
        assert!(Snapshot::of(&state) == before);
    } else {
        assert!(effects.is_none() && Snapshot::of(&state) == asking(&before, true));
    }
    assert!(acted() == 0);
    std::mem::forget((registry, state));
}

case_by_case! { nevermore_names_a_nonland_card;
    nevermore_under_the_first_player_names_the_first_card: (0, 0),
    nevermore_under_the_first_player_names_the_second_card: (0, 1),
    nevermore_under_the_first_player_refuses_an_answer_past_the_names: (0, 2),
    nevermore_under_the_second_player_names_the_first_card: (1, 0),
    nevermore_under_the_second_player_names_the_second_card: (1, 1),
    nevermore_under_the_second_player_refuses_an_answer_past_the_names: (1, 2),
}

/// Frightful Delusion's board: the spell, and its target, a spell under
/// `them` on the stack or gone from it to its owner's graveyard; `them`
/// with `in_hand` cards in hand, and the other player with one.
fn delusion(them: PlayerId, gone: bool, in_hand: usize) -> (CardRegistry, GameState, CardId, ObjectId, ObjectId, Vec<ObjectId>) {
    let (registry, mut state, blank, card) = board(Box::new(isd::frightful_delusion::FrightfulDelusion));
    let me = state.create_object(card, PlayerId(0), Zone::Stack, None, None);
    let spell = state.create_object(blank, PlayerId(0), if gone { Zone::Graveyard } else { Zone::Stack }, None, None);
    if gone {
        state.players[0].graveyard_order.push(spell);
    } else {
        state.get_object_mut(spell).expect("just created").controller = them;
    }
    let mut cards = Vec::with_capacity(2);
    for _ in 0..in_hand {
        cards.push(state.create_object(blank, them, Zone::Hand, None, None));
    }
    state.create_object(blank, PlayerId(1 - them.0), Zone::Hand, None, None);
    (registry, state, card, me, spell, cards)
}

card_proof! {
    /// Frightful Delusion, its target gone from the stack or illegal:
    /// nothing at all (CR 608.2b). For a spell under either player.
    fn frightful_delusion_does_nothing_with_its_target_gone() {
        let them = any_player();
        let gone: bool = kani::any();
        let (registry, mut state, card, me, spell, _) = delusion(them, gone, 1);
        let target = if gone && kani::any() { Target::Object(spell) } else { Target::Illegal };
        let before = Snapshot::of(&state);
        registry.get(card).expect("registered").on_resolve(&mut state, me, &[target], &registry);
        assert!(acted() == 0 && Snapshot::of(&state) == before);
        std::mem::forget((registry, state));
    }
}

