mod common;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::TemporaryEffect;
use mtg_engine::types::*;

use common::*;

/// Daybreak Ranger's front face ability "{T}: This creature deals 2 damage to
/// target creature with flying" bypasses the central damage handler. When the
/// target has protection from Werewolves, the damage should be prevented
/// (protection prevents damage from sources with matching qualities). But the
/// inline `obj.damage_marked += 2` in daybreak_ranger.rs skips the protection
/// check that apply_pending_effect performs at engine.rs:3054.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn daybreak_ranger_damage_bypasses_protection_from_werewolves() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Daybreak Ranger on the battlefield for P0 (front face, untransformed).
    let ranger = named_creature(&mut state, &registry, "Daybreak Ranger", P0);

    // Create a flying creature for P1. Use a generic creature and add flying.
    let flyer = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(flyer).unwrap().keywords.push(Keyword::Flying);

    // Grant the flying creature protection from Werewolves (until end of turn).
    // Daybreak Ranger has the "Werewolf" subtype, so this protection should
    // prevent all damage from it. The central apply_pending_effect handler at
    // engine.rs:3054 checks has_protection_from, which evaluates GrantProtection
    // temporary effects via matches_filter. HasSubtype("Werewolf") matches
    // Daybreak Ranger's subtypes.
    state.until_end_of_turn.push(TemporaryEffect::GrantProtection {
        target: flyer,
        filter: CreatureFilter::HasSubtype("Werewolf".into()),
    });

    // Verify the flying creature has no damage before activation.
    assert_eq!(
        state.get_object(flyer).unwrap().damage_marked, 0,
        "Flying creature should start with 0 damage"
    );

    // Activate Daybreak Ranger's ability targeting the flying creature.
    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: ranger,
            ability_index: 0,
            targets: vec![Target::Object(flyer)],
            tap_plan: vec![],
            sacrifice: None,
            x_value: None,
        },
        &registry,
    );

    // Oracle rules: protection prevents damage from sources the creature has
    // protection from. The flying creature has protection from Werewolves, and
    // Daybreak Ranger is a Werewolf, so this damage should be prevented.
    // The assertion encodes correct behavior per Oracle rules (0 damage).
    // This FAILS because the inline damage path skips protection checks.
    assert_eq!(
        state.get_object(flyer).unwrap().damage_marked, 0,
        "Protection from Werewolves should prevent damage from Daybreak Ranger (a Werewolf)"
    );
}
