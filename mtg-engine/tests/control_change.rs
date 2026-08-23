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

    let olivia = named_creature(&mut state, &r, "Olivia Voldaren", P0);
    // An opponent's Vampire that's been in play (ready to attack for P1).
    let vamp = named_creature(&mut state, &r, "Bloodcrazed Neonate", P1);
    state.get_object_mut(vamp).unwrap().summoning_sick = false;

    // Drive Olivia's steal ability (index 1) directly.
    let behavior = r.get(state.get_object(olivia).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, olivia, 1, &[mtg_engine::actions::Target::Object(vamp)], &r);

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
