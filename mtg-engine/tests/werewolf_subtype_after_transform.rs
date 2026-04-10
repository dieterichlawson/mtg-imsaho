/// Regression test for Bug D (BUG_REPORT_8SEAT.md): werewolf on_upkeep
/// transforms were flipping `is_transformed` and updating `obj.name` but
/// NOT `obj.subtypes` or `obj.keywords`. Cards that read obj.subtypes
/// (like Bonds of Faith's AttachedHasSubtype condition) misbehaved on
/// transformed werewolves — they were still treated as Humans, so Bonds
/// of Faith buffed them (+2/+2) and did not restrict their attacks.
///
/// This test exercises the transform path via the shared
/// helpers::apply_transform helper (which is what the werewolf on_upkeep
/// code now uses) and verifies that obj.subtypes and obj.keywords are
/// correctly switched to the back face.

mod common;
use common::*;

use mtg_engine::cards::helpers;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::PlayerId;
use mtg_engine::types::{Keyword, Step};

const P0: PlayerId = PlayerId(0);

#[test]
fn transformed_villagers_of_estwald_has_werewolf_subtype_not_human() {
    // Villagers of Estwald front: Human Werewolf. Back (Howlpack of Estwald): Werewolf only.
    // After transform, obj.subtypes must be populated with the back-face subtypes so
    // that EffectCondition::AttachedHasSubtype (used by Bonds of Faith) resolves to
    // "not a Human" for the transformed creature.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &registry, "Villagers of Estwald", P0);

    helpers::apply_transform(&mut state, id, &registry);

    let post = state.get_object(id).unwrap();
    assert!(post.is_transformed);
    assert_eq!(post.name, "Howlpack of Estwald");
    assert!(post.subtypes.iter().any(|s| s == "Werewolf"),
        "back face must still be a Werewolf");
    assert!(!post.subtypes.iter().any(|s| s == "Human"),
        "back face is NOT a Human per oracle");
}

#[test]
fn transformed_kruin_outlaw_swaps_first_strike_for_double_strike() {
    // Kruin Outlaw front: First Strike. Back (Terror of Kruin Pass): Double Strike.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &registry, "Kruin Outlaw", P0);

    assert!(state.has_keyword(id, Keyword::FirstStrike, &registry),
        "front face must have First Strike");
    assert!(!state.has_keyword(id, Keyword::DoubleStrike, &registry));

    helpers::apply_transform(&mut state, id, &registry);

    assert!(!state.has_keyword(id, Keyword::FirstStrike, &registry),
        "transformed Terror of Kruin Pass must lose First Strike");
    assert!(state.has_keyword(id, Keyword::DoubleStrike, &registry),
        "transformed Terror of Kruin Pass must have Double Strike");
}

#[test]
fn bonds_of_faith_prevents_attack_on_transformed_werewolf() {
    // Core reproduction of the log sequence from BUG_REPORT_8SEAT.md Bug D:
    //   - Howlpack of Estwald is NOT a Human per oracle.
    //   - Bonds of Faith should NOT grant +2/+2, and SHOULD prevent it from attacking.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wolf = named_creature(&mut state, &registry, "Villagers of Estwald", P0);
    helpers::apply_transform(&mut state, wolf, &registry);

    // Attach Bonds of Faith to the transformed wolf.
    let bonds = named_creature(&mut state, &registry, "Bonds of Faith", P0);
    state.get_object_mut(bonds).unwrap().attached_to = Some(wolf);

    // Effective P/T should be 4/6 (Howlpack base). NOT 6/8 (with +2/+2 bonus).
    assert_eq!(state.effective_power(wolf, &registry), Some(4),
        "Bonds of Faith must not grant +2/+2 to a non-Human");
    assert_eq!(state.effective_toughness(wolf, &registry), Some(6));

    // Attack/block should be prevented.
    assert!(!state.can_attack(wolf, &registry),
        "Bonds of Faith must prevent a non-Human from attacking");
    assert!(!state.can_block(wolf, &registry),
        "Bonds of Faith must prevent a non-Human from blocking");
}

#[test]
fn bonds_of_faith_still_buffs_human_on_front_face() {
    // Regression in the opposite direction: a front-face werewolf IS a Human
    // and should still receive +2/+2 from Bonds of Faith.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let villagers = named_creature(&mut state, &registry, "Villagers of Estwald", P0);
    let bonds = named_creature(&mut state, &registry, "Bonds of Faith", P0);
    state.get_object_mut(bonds).unwrap().attached_to = Some(villagers);

    // Villagers front: 2/3 + 2/+2 = 4/5.
    assert_eq!(state.effective_power(villagers, &registry), Some(4),
        "Front-face Human should get +2/+2 from Bonds of Faith");
    assert_eq!(state.effective_toughness(villagers, &registry), Some(5));
    assert!(state.can_attack(villagers, &registry),
        "Front-face Human should not be restricted by Bonds of Faith");
}
