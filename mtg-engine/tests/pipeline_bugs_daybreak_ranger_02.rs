mod common;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;

use common::*;

/// When a player controls two Daybreak Rangers in different transform states
/// (one untransformed front-face, one transformed as Nightfall Predator),
/// is_valid_target at daybreak_ranger.rs:127-131 uses
/// `state.objects.values().find()` to determine which face's targeting rules
/// apply. Since it receives only `caster: PlayerId` (not the activating
/// ObjectId), it returns the transform state of whichever copy the HashMap
/// iterator visits first and applies those rules to BOTH copies.
///
/// Oracle text - front face (Daybreak Ranger):
///   "{T}: This creature deals 2 damage to target creature with flying."
///   Can ONLY target creatures with flying.
///
/// Oracle text - back face (Nightfall Predator):
///   "{R}, {T}: This creature fights target creature."
///   Can target ANY creature.
///
/// The bug causes both copies to share the same targeting rules, violating
/// the oracle text for one of them.
#[test]
#[ignore] // Pipeline bug — awaiting fix
fn daybreak_ranger_multi_copy_targeting_uses_wrong_transform_state() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place two Daybreak Rangers on the battlefield for P0.
    let front_face = named_creature(&mut state, &registry, "Daybreak Ranger", P0);
    let back_face = named_creature(&mut state, &registry, "Daybreak Ranger", P0);

    // Transform the second copy into Nightfall Predator via on_upkeep.
    // Set conditions: no spells cast last turn, not first turn.
    state.is_first_turn = false;
    state.num_spells_cast_last_turn.clear();
    let behavior = registry.get(state.get_object(back_face).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, back_face, &registry);

    // Verify: front_face is untransformed, back_face is transformed.
    assert!(
        !state.get_object(front_face).unwrap().is_transformed,
        "First copy should remain untransformed (Daybreak Ranger)"
    );
    assert!(
        state.get_object(back_face).unwrap().is_transformed,
        "Second copy should be transformed (Nightfall Predator)"
    );

    // Create a non-flying creature for P1 as the target.
    let non_flyer = ready_creature(&mut state, P1, 2, 2);

    // Add Red mana so Nightfall Predator's {R} activation cost can be paid.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    // Generate legal actions and inspect ActivateAbility entries.
    let la = mtg_engine::engine::legal_actions(&state, &registry);

    // Can untransformed Daybreak Ranger target the non-flying creature?
    let front_targets_non_flyer = la.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == front_face && targets.contains(&Target::Object(non_flyer))
    ));

    // Can transformed Nightfall Predator target the non-flying creature?
    let back_targets_non_flyer = la.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
        if *object_id == back_face && targets.contains(&Target::Object(non_flyer))
    ));

    // Oracle says Daybreak Ranger (front) targets ONLY creatures with flying,
    // so front_targets_non_flyer should be false.
    // Oracle says Nightfall Predator (back) targets ANY creature,
    // so back_targets_non_flyer should be true.
    //
    // The bug in is_valid_target makes both copies use the same targeting rules
    // (whichever copy HashMap .find() returns first), so either both are true
    // or both are false — one of the two assertions below will fail.
    assert_eq!(
        (front_targets_non_flyer, back_targets_non_flyer),
        (false, true),
        "Daybreak Ranger (front) should NOT target non-flyer, \
         Nightfall Predator (back) SHOULD target non-flyer. \
         Bug: is_valid_target uses HashMap .find() with only PlayerId, \
         so both copies share the same targeting rules."
    );
}
