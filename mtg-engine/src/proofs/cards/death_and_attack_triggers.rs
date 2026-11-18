//! The creatures whose abilities trigger on a creature dying (CR 700.4,
//! 603.6c, 603.10a) or on attacking (CR 508.3a), and the statics printed
//! beside them.
//!
//! Scope: proved here is what each card declares (its abilities, costs,
//! targets, triggers and static effects) and what its own code writes to
//! the game state (a question asked, an until-end-of-turn effect added).
//! What a resolution does through the engine, the moves, damage, draws
//! and destruction, is not proved here, and where the text below speaks
//! of one it is of that part alone: each card is played end to end with
//! the real engine by `tests/card_semantics.rs`.
//!
//! - Doomed Traveler, 1/1: "When this creature dies, create a 1/1 white
//!   Spirit creature token with flying."
//! - Mausoleum Guard, 2/1: "When this creature dies, create two 1/1 white
//!   Spirit creature tokens with flying."
//! - Pitchburn Devils, 3/3: "When this creature dies, it deals 3 damage to
//!   any target."
//! - Moldgraf Monstrosity, 8/8 Trample: "When this creature dies, exile it,
//!   then return two creature cards at random from your graveyard to the
//!   battlefield."
//!   Its return is one event the engine makes
//!   (`GameState::put_onto_battlefield_together`, #1065), and is played,
//!   not proved: `tests/enters_as_copy.rs`,
//!   `tests/cards_complex_creatures.rs` and the generated boards of
//!   `tests/card_semantics.rs`.
//! - Elder Cathar, 2/2 Human Soldier: "When this creature dies, put a +1/+1
//!   counter on target creature you control. If that creature is a Human,
//!   put two +1/+1 counters on it instead."
//! - Bitterheart Witch, 1/2 Deathtouch: "When this creature dies, you may
//!   search your library for a Curse card, put it onto the battlefield
//!   attached to target player, then shuffle."
//! - Galvanic Juggernaut, 5/5: "This creature attacks each combat if able.
//!   This creature doesn't untap during your untap step. Whenever another
//!   creature dies, untap this creature."
//! - Rage Thrower, 3/2: "Whenever another creature dies, this creature
//!   deals 2 damage to target player or planeswalker."
//! - Geist of Saint Traft, 2/2 Hexproof: "Whenever Geist of Saint Traft
//!   attacks, create a 4/4 white Angel creature token with flying that's
//!   tapped and attacking. Exile that token at end of combat."
//! - Grimgrin, Corpse-Born, 5/5: "Grimgrin enters tapped and doesn't untap
//!   during your untap step. Sacrifice another creature: Untap Grimgrin and
//!   put a +1/+1 counter on it. Whenever Grimgrin attacks, destroy target
//!   creature defending player controls, then put a +1/+1 counter on
//!   Grimgrin." (Its entering tapped is `proofs::cards::replacements`', and
//!   its ability `proofs::cards::activated`'.)
//! - Kessig Cagebreakers, 3/4: "Whenever this creature attacks, create a
//!   2/2 green Wolf creature token that's tapped and attacking for each
//!   creature card in your graveyard."
//!
//! Each proof pins the trigger the card declares and runs the card's code
//! for it resolving. A "when this creature dies" trigger resolves with the
//! creature in its owner's graveyard or moved on from there by then, and
//! is controlled by whoever controlled the creature as it died (CR 603.3a,
//! 608.2h), either player here; the others resolve with their creature on
//! the battlefield or gone, under either player's control. Each is beside a
//! card and a token of any characteristics. "Another creature" is the
//! engine's to decide as the ability triggers (a creature's own death does
//! not trigger its watcher, `triggers::collect`), proved in
//! `proofs::actions::triggering`.

use super::{
    acted, board, card_proof, triggers, Snapshot,
};
use crate::actions::Target;
use crate::cards::{CardRegistry, TargetRequirement, TriggerKind};
use crate::ids::{ObjectId, PlayerId};
use crate::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use crate::types::Zone;

use crate::cards::isd;

/// Bitterheart Witch's board: the Witch in player 0's graveyard (it has
/// died), last controlled by `you`; player 0's library of any of a card of
/// the blank card and two of one Curse, in that order; player 1's of the
/// same Curse.
struct Witch {
    registry: CardRegistry,
    state: GameState,
    card: crate::ids::CardId,
    me: ObjectId,
    you: PlayerId,
}

fn witch() -> Witch {
    witch_of(PlayerId(u8::from(kani::any::<bool>())), [kani::any(), kani::any(), kani::any()])
}

/// [`witch`]'s board, last controlled by `you`, with each of player 0's
/// three library cards in the library if `in_library` says so and in
/// their hand otherwise.
fn witch_of(you: PlayerId, in_library: [bool; 3]) -> Witch {
    let (mut registry, mut state, blank, card) = board(Box::new(isd::bitterheart_witch::BitterheartWitch));
    let curse_card = registry.register(Box::new(isd::curse_of_the_pierced_heart::CurseOfThePiercedHeart));
    let me = state.create_object(card, PlayerId(0), Zone::Graveyard, None, None);
    state.players[0].graveyard_order.push(me);
    state.get_object_mut(me).expect("just created").last_controller = Some(you);
    let cards = [blank, curse_card, curse_card];
    for i in 0..3 {
        let id = state.create_object(cards[i], PlayerId(0), Zone::Library, None, None);
        if in_library[i] {
            state.players[0].library_order.push(id);
        } else {
            state.get_object_mut(id).expect("just created").zone = Zone::Hand;
        }
    }
    let theirs = state.create_object(curse_card, PlayerId(1), Zone::Library, None, None);
    state.players[1].library_order.push(theirs);
    Witch { registry, state, card, me, you }
}

/// Bitterheart Witch dying with `target` its trigger's target: whether it
/// asked whoever controlled it as it died whether to search ("you may"),
/// having kept the player targeted for the search's answer; and with no
/// player targeted, nothing at all. The game after, the Witch's board,
/// and the board before.
fn witch_dies(w: &mut Witch, target: &Target) -> Snapshot {
    assert!(triggers(&isd::bitterheart_witch::BitterheartWitch) == [(TriggerKind::SelfDies, Some(TargetRequirement::PlayerOnly))]);
    let before = Snapshot::of(&w.state);
    w.registry.get(w.card).expect("registered").on_dies(&mut w.state, w.me, &[target.clone()], &w.registry);
    let Target::Player(p) = target else {
        assert!(acted() == 0 && Snapshot::of(&w.state) == before);
        return before;
    };
    let asked = matches!(&w.state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player, source, choice: ResolutionChoiceKind::YesNo { source_card, .. },
    }) if *player == w.you && *source == w.me && *source_card == w.me);
    assert!(asked && acted() == 0);
    assert!(w.state.get_object(w.me).expect("on the board").card_state.get("curse_target") == Some(&ObjectId(u64::from(p.0))));
    crate::proofs::cards::clear_prompt(&mut w.state);
    before
}

card_proof! {
    /// Bitterheart Witch dying: "target player", chosen as it triggers
    /// (CR 603.3d); with no player targeted, nothing at all.
    fn bitterheart_witch_with_no_player_targeted_does_nothing() {
        let mut w = witch();
        witch_dies(&mut w, &Target::Illegal);
        std::mem::forget(w);
    }
}

card_proof! {
    /// Bitterheart Witch dying with a player targeted: whoever controlled
    /// the Witch as it died is asked whether to search ("you may");
    /// declining, nothing.
    fn bitterheart_witch_asks_whether_to_search_and_declined_does_nothing() {
        let mut w = witch();
        let before = witch_dies(&mut w, &Target::Player(PlayerId(u8::from(kani::any::<bool>()))));
        w.registry.get(w.card).expect("registered").on_yes_no_choice(&mut w.state, w.me, false, &w.registry);
        assert!(acted() == 0 && Snapshot::of(&w.state) == before);
        std::mem::forget(w);
    }
}

