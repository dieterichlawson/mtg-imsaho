//! Regression tests for control-change summoning sickness (CR 302.6 / 508.1a).
//!
//! Gaining control of an in-play permanent makes it summoning-sick for its
//! new controller — it can't attack or use tap abilities until their next
//! untap step, unless it has haste. `GameState::change_control` is the single
//! correct way to reassign controller; assigning the field directly (the old
//! Olivia Voldaren steal) skipped the reset and let a stolen creature attack
//! the turn it was taken.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat::eligible_attackers;
use mtg_engine::types::*;

fn reg() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// change_control sets summoning sickness on the new controller's side.
#[test]
fn change_control_applies_summoning_sickness() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // A creature P1 has controlled since their last turn (not summoning sick).
    let creature = ready_creature(&mut state, P1, 2, 2);
    assert!(!state.get_object(creature).unwrap().summoning_sick);

    state.change_control(creature, P0);

    assert_eq!(state.get_object(creature).unwrap().controller, P0);
    assert!(state.get_object(creature).unwrap().summoning_sick,
        "a creature that just changed control is summoning-sick for the new controller");
    assert!(!eligible_attackers(&state, P0, &r).contains(&creature),
        "the stolen creature cannot attack the turn control changed");
}

/// Olivia Voldaren's steal ({3}{B}{B}: gain control of target Vampire, no
/// haste) must leave the stolen Vampire summoning-sick.
#[test]
fn olivia_steal_leaves_vampire_summoning_sick() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &r, "Olivia Voldaren", P0);
    // An opponent's Vampire that's been in play (ready to attack for P1).
    let vamp = named_permanent(&mut state, &r, "Bloodcrazed Neonate", P1);
    state.get_object_mut(vamp).unwrap().summoning_sick = false;

    // Drive Olivia's steal ability (index 1) directly.
    activate_via_hooks(&mut state, &r, olivia, 1, &[mtg_engine::actions::Target::Object(vamp)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &r);

    assert_eq!(state.get_object(vamp).unwrap().controller, P0, "steal should succeed");
    assert!(state.get_object(vamp).unwrap().summoning_sick,
        "stolen Vampire must be summoning-sick for its new controller (no haste)");
    assert!(!eligible_attackers(&state, P0, &r).contains(&vamp),
        "stolen Vampire cannot attack the turn it was stolen");
}

/// Traitorous Blood grants haste alongside the steal, so its creature CAN
/// attack despite the summoning sickness that change_control now applies.
#[test]
fn traitorous_blood_creature_can_still_attack_via_haste() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(victim).unwrap().tapped = true; // Traitorous Blood untaps it

    let blood = castable_spell(&mut state, &r, "Traitorous Blood", P0);
    let state = cast_and_resolve(&state, &r, blood, vec![mtg_engine::actions::Target::Object(victim)]);

    assert_eq!(state.get_object(victim).unwrap().controller, P0);
    assert!(!state.get_object(victim).unwrap().tapped, "Traitorous Blood untaps the creature");
    assert!(eligible_attackers(&state, P0, &r).contains(&victim),
        "haste lets the stolen creature attack despite summoning sickness");
}

/// CR 108.4: a card has a controller only while it represents a permanent or a
/// spell. When a stolen creature leaves the battlefield the thief stops
/// controlling it — its owner does — and anything reading the card's
/// controller in its new zone has to see that.
///
/// Boneyard Wurm is the card that notices: its power counts creature cards in
/// *its controller's* graveyard, and a card whose controller was never reset
/// counted the wrong player's.
#[test]
fn a_card_leaving_the_battlefield_stops_having_a_controller() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P1's Boneyard Wurm, stolen by P0.
    let wurm = named_permanent(&mut state, &r, "Boneyard Wurm", P1);
    state.change_control(wurm, P0);
    assert_eq!(state.get_object(wurm).unwrap().controller, P0);
    assert_eq!(state.get_object(wurm).unwrap().owner, P1);

    // It dies. The card goes to its owner's graveyard, and the thief's control
    // of it ends with the permanent.
    state.move_object(wurm, Zone::Graveyard, &r);

    assert_eq!(state.get_object(wurm).unwrap().owner, P1,
        "a card in a graveyard is its owner's");
    assert_eq!(state.get_object(wurm).unwrap().controller, P1,
        "CR 108.4: off the battlefield the owner is treated as the controller");

    // And the characteristic-defining power reads the right graveyard. Only
    // the Wurm itself is in P1's; P0 has two creature cards in theirs.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &r);
    }
    assert_eq!(state.effective_power(wurm, &r), Some(1),
        "it counts itself in its owner's graveyard, not the thief's two");
}
