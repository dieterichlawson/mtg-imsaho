mod common;
use common::*;

use mtg_engine::cards::helpers::apply_transform;
use mtg_engine::cards::CardRegistry;
use mtg_engine::types::*;
// Shared setup: place a named DFC on the battlefield, transform it, then
// move it to the target zone. Returns (state, object_id, registry) so each
// test can assert on the post-zone-change characteristics.
fn transform_then_move(
    front_name: &str,
    dest: Zone,
) -> (mtg_engine::state::GameState, mtg_engine::ids::ObjectId, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = named_creature(&mut state, &reg, front_name, P0);
    apply_transform(&mut state, id, &reg);
    state.move_object(id, dest, &reg);
    (state, id, reg)
}

// CR 712.8a: A double-faced card not on the battlefield or stack has only its
// front-face characteristics. When a transformed DFC changes zones, the
// engine must restore front-face name, keywords, and subtypes.

// Bloodline Keeper (front) / Lord of Lineage (back).
// After transform + death, graveyard name must be "Bloodline Keeper".
#[test]
fn test_bloodline_keeper_name_reverts_in_graveyard() {
    let (state, id, _reg) = transform_then_move("Bloodline Keeper", Zone::Graveyard);
    assert_eq!(state.get_object(id).unwrap().is_transformed, false);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Bloodline Keeper",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Lord of Lineage'"
    );
}

// Daybreak Ranger (front: Human Archer Ranger Werewolf) / Nightfall Predator (back: Werewolf).
// After transform + death, subtypes must revert to front face.
#[test]
fn test_daybreak_ranger_subtypes_revert_in_graveyard() {
    let (state, id, reg) = transform_then_move("Daybreak Ranger", Zone::Graveyard);
    // Asserted through the characteristics layer, which is where a card's
    // subtypes live — `obj.subtypes` holds only runtime grants.
    for sub in ["Human", "Archer"] {
        assert!(state.has_subtype(id, sub, &reg),
            "CR 712.8a: a DFC in the graveyard has its FRONT face, so it should \
             have subtype {sub:?}; subtypes_of = {:?}", state.subtypes_of(id, &reg));
    }
}

// Delver of Secrets (front: no Flying) / Insectile Aberration (back: Flying).
// After transform + death, keywords must not include Flying, name must be front face.
#[test]
fn test_delver_keywords_revert_in_graveyard() {
    let (state, id, _reg) = transform_then_move("Delver of Secrets", Zone::Graveyard);
    let obj = state.get_object(id).unwrap();
    assert_eq!(
        obj.keywords.contains(&Keyword::Flying),
        false,
        "CR 712.8a: DFC in graveyard should not have back-face keyword Flying"
    );
    assert_eq!(
        obj.name,
        "Delver of Secrets",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Insectile Aberration'"
    );
}

// Hanweir Watchkeep (front) / Bane of Hanweir (back).
// After transform + death, graveyard name must be "Hanweir Watchkeep".
#[test]
fn test_hanweir_watchkeep_name_reverts_in_graveyard() {
    let (state, id, _reg) = transform_then_move("Hanweir Watchkeep", Zone::Graveyard);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Hanweir Watchkeep",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Bane of Hanweir'"
    );
}

// Instigator Gang (front) / Wildblood Pack (back).
// After transform + death, graveyard name must be "Instigator Gang".
#[test]
fn test_instigator_gang_name_reverts_in_graveyard() {
    let (state, id, _reg) = transform_then_move("Instigator Gang", Zone::Graveyard);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Instigator Gang",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Wildblood Pack'"
    );
}

// Kruin Outlaw (front) / Terror of Kruin Pass (back).
// After transform + death, graveyard name must be "Kruin Outlaw".
#[test]
fn test_kruin_outlaw_name_reverts_in_graveyard() {
    let (state, id, _reg) = transform_then_move("Kruin Outlaw", Zone::Graveyard);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Kruin Outlaw",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Terror of Kruin Pass'"
    );
}

// Mayor of Avabruck (front) / Howlpack Alpha (back).
// After transform + death, graveyard name must be "Mayor of Avabruck".
#[test]
fn test_mayor_of_avabruck_name_reverts_in_graveyard() {
    let (state, id, _reg) = transform_then_move("Mayor of Avabruck", Zone::Graveyard);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Mayor of Avabruck",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Howlpack Alpha'"
    );
}

// Cloistered Youth (front: Human) / Unholy Fiend (back: Horror).
// After transform + death, graveyard name and subtypes must revert.
#[test]
fn test_cloistered_youth_name_reverts_on_death() {
    let (state, id, reg) = transform_then_move("Cloistered Youth", Zone::Graveyard);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Cloistered Youth",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Unholy Fiend'"
    );
    assert!(state.has_subtype(id, "Human", &reg),
        "CR 712.8a: DFC in graveyard has its front face, so it is a Human; \
         subtypes_of = {:?}", state.subtypes_of(id, &reg));
    assert!(!state.has_subtype(id, "Horror", &reg),
        "CR 712.8a: the back face's Horror subtype must not survive the zone \
         change; subtypes_of = {:?}", state.subtypes_of(id, &reg));
}

// Cloistered Youth exiled while transformed: same rule applies in exile.
#[test]
fn test_cloistered_youth_name_reverts_on_exile() {
    let (state, id, reg) = transform_then_move("Cloistered Youth", Zone::Exile);
    assert_eq!(
        state.get_object(id).unwrap().name,
        "Cloistered Youth",
        "CR 712.8a: DFC in exile should have front-face name, not 'Unholy Fiend'"
    );
    assert!(state.has_subtype(id, "Human", &reg),
        "CR 712.8a: DFC in exile has its front face, so it is a Human; \
         subtypes_of = {:?}", state.subtypes_of(id, &reg));
    assert!(!state.has_subtype(id, "Horror", &reg),
        "CR 712.8a: the back face's Horror subtype must not survive the zone \
         change; subtypes_of = {:?}", state.subtypes_of(id, &reg));
}

// Garruk Relentless (front) / Garruk, the Veil-Cursed (back).
// Garruk uses manual transform (state trigger sets is_transformed + name),
// not apply_transform. Simulate the same mechanism.
#[test]
fn garruk_name_resets_after_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);

    // Simulate state-trigger transform (matches on_state_trigger behavior)
    let obj = state.get_object_mut(garruk).unwrap();
    obj.is_transformed = true;
    obj.name = "Garruk, the Veil-Cursed".into();

    assert_eq!(state.get_object(garruk).unwrap().name, "Garruk, the Veil-Cursed");

    state.move_object(garruk, Zone::Graveyard, &reg);

    assert_eq!(state.get_object(garruk).unwrap().is_transformed, false);
    assert_eq!(
        state.get_object(garruk).unwrap().name,
        "Garruk Relentless",
        "CR 712.8a: DFC in graveyard should have front-face name, not 'Garruk, the Veil-Cursed'"
    );
}
