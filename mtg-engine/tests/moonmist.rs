//! Tests for Moonmist.
//!
//! Oracle: {1}{G} Instant
//! Transform all Humans. Prevent all combat damage that would be dealt this turn
//! by creatures other than Werewolves and Wolves.

mod common;
use common::*;
use mtg_engine::types::*;

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

/// "Prevent all combat damage that would be dealt this turn by creatures that
/// aren't Werewolves or Wolves." One rule, reached three ways: damage to a
/// player, damage a non-Wolf attacker deals to its blocker, and damage that
/// blocker deals back.
///
/// Every row runs twice — with the prevention and without it — because "no
/// damage was dealt" is also what a combat that never happened looks like.
/// The blocked row used to key `blocker_assignments` by the *blocker*, so the
/// engine saw an unblocked attacker and two creatures that were never in
/// combat with each other; both halves read 0 with Moonmist deleted.
#[test]
fn moonmist_prevents_combat_damage_from_everything_but_wolves() {
    let reg = registry();

    /// Runs one combat, with or without Moonmist's prevention, and reports
    /// (life lost by P1, damage on the attacker, damage on the blocker).
    fn combat(reg: &mtg_engine::cards::CardRegistry, attacker_card: Option<&str>, blocked: bool,
              prevented: bool) -> (i32, u32, u32) {
        let mut state = game_at_step(Step::CombatDamage, P0);
        if prevented {
            // The real card, not a hand-built copy of its effect: a helper that
            // rebuilds the filter tests the filter and not Moonmist, and a card
            // that stopped naming Werewolves would still pass every row.
            //
            // Ruling: "Moonmist will prevent combat damage dealt by a creature
            // that isn't a Werewolf or a Wolf even if that creature wasn't on
            // the battlefield ... when Moonmist resolved." The spell resolves
            // before the attacker exists, which is that ruling.
            let moonmist = castable_spell(&mut state, reg, "Moonmist", P0);
            state = cast_and_resolve(&state, reg, moonmist, vec![]);
        }
        let attacker = match attacker_card {
            Some(name) => named_permanent(&mut state, reg, name, P0),
            None => ready_creature(&mut state, P0, 3, 3),
        };
        let blocker = ready_creature(&mut state, P1, 2, 2);
        if blocked {
            attacks_blocked_by(&mut state, attacker, P1, &[blocker]);
        } else {
            attacks_unblocked(&mut state, attacker, P1);
        }

        mtg_engine::combat::deal_combat_damage(&mut state, reg);

        (20 - state.players[1].life,
         state.get_object(attacker).unwrap().damage_marked,
         state.get_object(blocker).unwrap().damage_marked)
    }

    // Unblocked non-Wolf: no damage to the player — but it would have dealt some.
    assert_eq!(combat(&reg, None, false, true).0, 0,
        "a non-Wolf attacker deals no combat damage to the player");
    assert!(combat(&reg, None, false, false).0 > 0,
        "control: without Moonmist that same attacker does get through");

    // Blocked non-Wolf: neither side takes damage, and both would have.
    let (_, attacker_dmg, blocker_dmg) = combat(&reg, None, true, true);
    assert_eq!((attacker_dmg, blocker_dmg), (0, 0),
        "neither a non-Wolf attacker nor its non-Wolf blocker deals damage");
    let (_, attacker_dmg, blocker_dmg) = combat(&reg, None, true, false);
    assert!(attacker_dmg > 0 && blocker_dmg > 0,
        "control: without Moonmist the same block trades damage both ways \
         (got attacker {attacker_dmg}, blocker {blocker_dmg}) — if either is 0 \
         the combat was not set up, and the assertion above proves nothing");

    // "other than Werewolves **and Wolves**" — both exceptions, and both need
    // a row: a filter naming only Wolves passed every assertion above.
    for exempt in ["Darkthicket Wolf", "Village Ironsmith"] {
        assert!(combat(&reg, Some(exempt), false, true).0 > 0,
            "{exempt} is one of the two types Moonmist spares, so it still \
             deals its combat damage");
    }
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
