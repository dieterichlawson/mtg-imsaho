//! Tests for Moonmist.
//!
//! Oracle: {1}{G} Instant
//! Transform all Humans. Prevent all combat damage that would be dealt this turn
//! by creatures other than Werewolves and Wolves.

mod common;
use common::*;
use mtg_engine::types::*;

/// The blanket prevention Moonmist puts up: everything except Wolves and
/// Werewolves stops dealing combat damage this turn. Built here the way the
/// card builds it, so the test exercises the same filter.
fn moonmist_prevention() -> mtg_engine::state::TemporaryEffect {
    mtg_engine::state::TemporaryEffect::PreventCombatDamageExcept {
        filter: CreatureFilter::Or(vec![
            CreatureFilter::HasSubtype("Wolf".into()),
            CreatureFilter::HasSubtype("Werewolf".into()),
        ]),
    }
}
/// After Moonmist resolves, the flag is set.
#[test]
fn sets_prevention_flag() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let moonmist = castable_spell(&mut state, &reg, "Moonmist", P0);
    let new_state = cast_and_resolve(&state, &reg, moonmist, vec![]);

    assert!(new_state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::PreventCombatDamageExcept { .. })),
        "Moonmist should set the prevention flag");
}

/// Non-Wolf/Werewolf creature deals no combat damage to player after Moonmist.
#[test]
fn prevents_non_wolf_combat_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.until_end_of_turn.push(moonmist_prevention());
    state.combat = Some(mtg_engine::state::CombatState::new());

    // A plain 3/3 creature (not Wolf or Werewolf).
    let attacker = ready_creature(&mut state, P0, 3, 3);
    state.combat.as_mut().unwrap().attackers.insert(attacker, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.players[1].life, 20,
        "Non-Wolf creature should deal no combat damage after Moonmist");
}

/// Wolf creature still deals combat damage after Moonmist.
#[test]
fn wolf_still_deals_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.until_end_of_turn.push(moonmist_prevention());
    state.combat = Some(mtg_engine::state::CombatState::new());

    // Use a named Wolf card.
    let wolf = named_permanent(&mut state, &reg, "Darkthicket Wolf", P0);
    state.combat.as_mut().unwrap().attackers.insert(wolf, P1);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    assert!(state.players[1].life < 20,
        "Wolf creature should still deal combat damage after Moonmist");
}

/// Non-Wolf creature deals no combat damage to creatures after Moonmist.
#[test]
fn prevents_non_wolf_combat_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    state.until_end_of_turn.push(moonmist_prevention());
    state.combat = Some(mtg_engine::state::CombatState::new());

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);
    state.combat.as_mut().unwrap().attackers.insert(attacker, P1);
    state.combat.as_mut().unwrap().blocker_assignments.insert(blocker, vec![attacker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // Neither creature should have taken damage (both are non-Wolf/Werewolf).
    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 0,
        "Attacker should take no damage when blocker's damage is prevented");
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 0,
        "Blocker should take no damage when attacker's damage is prevented");
}

/// Moonmist transforms a front-face Human DFC to its back face.
#[test]
fn transforms_front_face_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sentry = named_permanent(&mut state, &reg, "Thraben Sentry", P0);
    assert!(!state.get_object(sentry).unwrap().is_transformed);
    assert_eq!(state.get_object(sentry).unwrap().name, "Thraben Sentry");

    let moonmist = castable_spell(&mut state, &reg, "Moonmist", P0);
    let new_state = cast_and_resolve(&state, &reg, moonmist, vec![]);

    assert!(new_state.get_object(sentry).unwrap().is_transformed,
        "Thraben Sentry should transform to Thraben Militia");
    assert_eq!(new_state.get_object(sentry).unwrap().name, "Thraben Militia");
}

/// Moonmist transforms a back-face Human (e.g., Thraben Militia) back to front face.
/// This is the key fix — Thraben Militia is Human on its back face.
#[test]
fn transforms_back_face_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Already transformed, through the engine's own transform rather than by
    // writing the fields a transform happens to touch.
    let sentry = named_permanent(&mut state, &reg, "Thraben Sentry", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, sentry, &reg);
    assert!(state.has_subtype(sentry, "Human", &reg),
        "test precondition: Thraben Militia is a Human on the back face, which \
         is what makes it a Moonmist target again");

    let moonmist = castable_spell(&mut state, &reg, "Moonmist", P0);
    let new_state = cast_and_resolve(&state, &reg, moonmist, vec![]);

    // Should transform back to front face (Thraben Sentry).
    assert!(!new_state.get_object(sentry).unwrap().is_transformed,
        "Thraben Militia (back-face Human) should transform back to Thraben Sentry");
    assert_eq!(new_state.get_object(sentry).unwrap().name, "Thraben Sentry");
}

/// Non-DFC Humans should not be affected by Moonmist (only DFCs can transform).
#[test]
fn does_not_transform_non_dfc_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Elite Inquisitor is a Human but not a DFC.
    let inquisitor = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    assert!(!state.get_object(inquisitor).unwrap().is_transformed);

    let moonmist = castable_spell(&mut state, &reg, "Moonmist", P0);
    let new_state = cast_and_resolve(&state, &reg, moonmist, vec![]);

    // Should NOT be transformed (not a DFC).
    assert!(!new_state.get_object(inquisitor).unwrap().is_transformed,
        "Non-DFC Human should not be affected by Moonmist");
    assert_eq!(new_state.get_object(inquisitor).unwrap().name, "Elite Inquisitor");
}

// ---------------------------------------------------------------------------
// "Transform all Humans" means Humans, both times
// ---------------------------------------------------------------------------

/// Moonmist transforms Humans. A Werewolf's back face is a Werewolf, not a
/// Human — so casting Moonmist twice does not flip a werewolf there and back.
/// It goes over on the first cast and stays over; only returning to its Human
/// front face makes it a Moonmist target again.
///
/// This replaces a test whose body was a note working the rule out in prose
/// ("Let me check the oracle text... This test doesn't reproduce the exact
/// scenario. Mark as needs rework.") and which asserted nothing at all after
/// its first cast.
#[test]
fn moonmist_only_transforms_whatever_is_a_human_right_now() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Village Ironsmith is a Human Werewolf; its back face, Ironfang, is a
    // Werewolf.
    let ironsmith = named_permanent(&mut state, &reg, "Village Ironsmith", P0);
    assert!(state.has_subtype(ironsmith, "Human", &reg), "test precondition: a Human");

    let first = castable_spell(&mut state, &reg, "Moonmist", P0);
    let mut state = cast_and_resolve(&state, &reg, first, vec![]);
    assert!(state.get_object(ironsmith).unwrap().is_transformed,
        "the Human transforms");
    assert!(!state.has_subtype(ironsmith, "Human", &reg),
        "and the back face is not a Human");

    let second = castable_spell(&mut state, &reg, "Moonmist", P0);
    let mut state = cast_and_resolve(&state, &reg, second, vec![]);
    assert!(state.get_object(ironsmith).unwrap().is_transformed,
        "a second Moonmist finds no Human to transform, so it stays a Werewolf \
         — 'transform all Humans' is not 'transform all werewolf cards'");

    // Back to the front face by its own upkeep trigger, and Moonmist reaches
    // it again.
    mtg_engine::cards::helpers::apply_transform(&mut state, ironsmith, &reg);
    assert!(state.has_subtype(ironsmith, "Human", &reg), "a Human once more");

    let third = castable_spell(&mut state, &reg, "Moonmist", P0);
    let state = cast_and_resolve(&state, &reg, third, vec![]);
    assert!(state.get_object(ironsmith).unwrap().is_transformed,
        "so the next Moonmist transforms it again");
}
