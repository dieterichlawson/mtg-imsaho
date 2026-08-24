//! Things that happen "at once" have to actually happen at once.
//!
//! CR 700.2c: an effect that destroys several permanents destroys them
//! simultaneously. CR 101.4: when several players each make a choice for one
//! effect, they choose in APNAP order and the results take effect together.
//!
//! Both were implemented as loops, and a loop is observably different. Each
//! step of a destruction loop changes the battlefield the next step is judged
//! against, and each step of a choice loop hands control back to the game
//! loop — which collects triggers — before the remaining players have chosen.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ---------------------------------------------------------------------------
// CR 700.2c — simultaneous destruction.
// ---------------------------------------------------------------------------

/// Angelic Overseer is "indestructible as long as you control a Human". When
/// a sweeper catches the Overseer and its controller's last Human together,
/// the Human is still on the battlefield at the moment destruction happens,
/// so the Overseer survives. Destroying one at a time gets this wrong
/// whenever the Human is reached first.
#[test]
fn conditional_indestructible_survives_when_last_protector_dies_simultaneously() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let human = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let overseer = named_creature(&mut state, &reg, "Angelic Overseer", P0);
    assert!(state.has_subtype(human, "Human", &reg), "test precondition");
    assert!(state.has_keyword(overseer, Keyword::Indestructible, &reg),
        "test precondition: the Overseer is indestructible while the Human is around");

    // Doomed in the order that used to break it: Human first.
    mtg_engine::destruction::try_destroy_all(&mut state, &[human, overseer], &reg);

    assert_eq!(state.get_object(human).unwrap().zone, Zone::Graveyard,
        "the Human is destroyed");
    assert_eq!(state.get_object(overseer).unwrap().zone, Zone::Battlefield,
        "the Overseer was indestructible at the moment destruction happened, so it \
         survives even though its Human died in the same event (CR 700.2c)");
}

/// Order must not matter. Same board, Overseer listed first.
#[test]
fn simultaneous_destruction_is_order_independent() {
    let reg = registry();
    for overseer_first in [true, false] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let human = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
        let overseer = named_creature(&mut state, &reg, "Angelic Overseer", P0);

        let doomed = if overseer_first { [overseer, human] } else { [human, overseer] };
        mtg_engine::destruction::try_destroy_all(&mut state, &doomed, &reg);

        assert_eq!(state.get_object(overseer).unwrap().zone, Zone::Battlefield,
            "overseer_first={overseer_first}: the outcome must not depend on list order");
    }
}

/// Nothing conditional in play: everything doomed dies, once each.
#[test]
fn all_unchosen_creatures_destroyed_without_intermediate_state_changes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doomed: Vec<_> = (0..3)
        .map(|_| named_creature(&mut state, &reg, "Walking Corpse", P0))
        .collect();

    let results = mtg_engine::destruction::try_destroy_all(&mut state, &doomed, &reg);

    assert_eq!(results.len(), 3);
    for (id, result) in results {
        assert_eq!(result, mtg_engine::destruction::DestroyResult::Died);
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Graveyard);
    }
    let deaths = state.events.iter()
        .filter(|e| matches!(e, mtg_engine::events::GameEvent::CreatureDied { .. }))
        .count();
    assert_eq!(deaths, 3, "one death announced per creature, no more and no fewer");
}

/// Divine Reckoning's "destroy the rest" goes through the same path.
#[test]
fn divine_reckoning_spares_a_conditionally_indestructible_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 keeps a Walking Corpse; the Overseer and the last Human are "the rest".
    let keeper = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let human = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let overseer = named_creature(&mut state, &reg, "Angelic Overseer", P0);

    let reckoning = spell_in_hand(&mut state, &reg, "Divine Reckoning", P0);
    state.move_object(reckoning, Zone::Stack, &reg);
    reg.get(state.get_object(reckoning).unwrap().card_id).unwrap()
        .on_resolve(&mut state, reckoning, &[], &reg);

    // P0 is asked which creature to keep; P1 controls nothing, so P0 is the
    // only chooser.
    if state.awaiting_action.is_some() {
        state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(mtg_engine::actions::Target::Object(keeper))),
        }, &reg);
    }

    assert_eq!(state.get_object(keeper).unwrap().zone, Zone::Battlefield,
        "the kept creature survives");
    assert_eq!(state.get_object(human).unwrap().zone, Zone::Graveyard,
        "the unchosen Human is destroyed");
    assert_eq!(state.get_object(overseer).unwrap().zone, Zone::Battlefield,
        "the Overseer's Human was alive when 'destroy the rest' happened (CR 700.2c)");
}

// ---------------------------------------------------------------------------
// CR 101.4 — each player chooses, then it all happens at once.
// ---------------------------------------------------------------------------

/// Liliana's +1 is "Each player discards a card". Discarding as each player
/// chooses lets the game loop collect a discard trigger — Murder of Crows'
/// "whenever another creature dies, you may draw a card then discard a card"
/// is the classic watcher — while a later player is still being asked.
#[test]
fn liliana_plus_one_holds_every_discard_until_the_last_player_has_chosen() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);

    let p0_a = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    let p0_b = spell_in_hand(&mut state, &reg, "Chapel Geist", P0);
    let p1_a = spell_in_hand(&mut state, &reg, "Walking Corpse", P1);
    let p1_b = spell_in_hand(&mut state, &reg, "Chapel Geist", P1);

    reg.get(state.get_object(liliana).unwrap().card_id).unwrap()
        .on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // P0 chooses first.
    state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p0_a),
    }, &reg);

    assert_eq!(state.get_object(p0_a).unwrap().zone, Zone::Hand,
        "P0's card must not leave their hand while P1 is still choosing (CR 101.4)");
    assert!(!state.events.iter().any(|e|
        matches!(e, mtg_engine::events::GameEvent::Discarded { .. })),
        "and no discard may be announced yet — a watcher would see it early");
    assert!(matches!(&state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice {
            player, choice: ResolutionChoiceKind::ChooseCardFromHand { .. }, .. }) if *player == P1),
        "P1 is asked next; got {:?}", state.awaiting_action);

    // P1 chooses; now both cards go at once.
    state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p1_b),
    }, &reg);

    assert_eq!(state.get_object(p0_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_b).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p0_b).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(p1_a).unwrap().zone, Zone::Hand);
}

/// The auto-discard path — a player with exactly one card gets no choice —
/// must wait too.
#[test]
fn auto_discard_also_waits_for_the_other_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);

    // P0 has exactly one card (no choice); P1 has two (a real choice).
    let p0_only = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    let p1_a = spell_in_hand(&mut state, &reg, "Walking Corpse", P1);
    let p1_b = spell_in_hand(&mut state, &reg, "Chapel Geist", P1);

    reg.get(state.get_object(liliana).unwrap().card_id).unwrap()
        .on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    assert_eq!(state.get_object(p0_only).unwrap().zone, Zone::Hand,
        "P0's forced discard must still wait for P1's choice (CR 101.4)");
    assert!(matches!(&state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice { player, .. }) if *player == P1),
        "P1 is being asked; got {:?}", state.awaiting_action);

    state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p1_a),
    }, &reg);

    assert_eq!(state.get_object(p0_only).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_b).unwrap().zone, Zone::Hand);
}

/// An ordinary single-player discard is unaffected — the card goes as soon as
/// it is chosen.
#[test]
fn a_single_player_discard_still_applies_immediately() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let weevil = named_creature(&mut state, &reg, "Brain Weevil", P0);
    let a = spell_in_hand(&mut state, &reg, "Walking Corpse", P1);
    spell_in_hand(&mut state, &reg, "Chapel Geist", P1);
    spell_in_hand(&mut state, &reg, "Avacyn's Pilgrim", P1);

    reg.get(state.get_object(weevil).unwrap().card_id).unwrap()
        .on_activate_ability(&mut state, weevil, 0,
            &[mtg_engine::actions::Target::Player(P1)], &reg);

    state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(a),
    }, &reg);

    assert_eq!(state.get_object(a).unwrap().zone, Zone::Graveyard,
        "one player, one choice — nothing to wait for");
}
