//! Tests for Tier 15 hard/complex Innistrad cards.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::actions::Action;
use mtg_engine::sba::check_state_based_actions_with_registry;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Delver of Secrets ──────────────────────────────────────────

#[test]
fn delver_transforms_when_top_card_is_instant() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    // Put Delver on the battlefield.
    let delver = named_creature(&mut state, &reg, "Delver of Secrets", P0);
    assert_eq!(state.get_object(delver).unwrap().power, Some(1));

    // Put a Lightning Bolt (instant) on top of library.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Library);
    state.players[0].library_order.insert(0, bolt);

    // Trigger upkeep.
    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &reg);

    // Should be transformed.
    assert!(state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Insectile Aberration");
    // Dynamic P/T should be 3/2.
    assert_eq!(behavior.dynamic_pt(&state, delver), Some((3, 2)));
}

#[test]
fn delver_does_not_transform_when_top_card_is_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let delver = named_creature(&mut state, &reg, "Delver of Secrets", P0);

    // Put a creature on top of library.
    let creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(creature, Zone::Library);
    state.players[0].library_order.insert(0, creature);

    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &reg);

    // Should NOT be transformed.
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Delver of Secrets");
}

// ── Cloistered Youth ──────────────────────────────────────────

#[test]
fn cloistered_youth_transforms_at_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);

    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, youth, &reg);

    assert!(state.get_object(youth).unwrap().is_transformed);
    assert_eq!(state.get_object(youth).unwrap().name, "Unholy Fiend");
    assert_eq!(behavior.dynamic_pt(&state, youth), Some((3, 3)));
}

#[test]
fn unholy_fiend_drains_life_at_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);
    // Pre-transform.
    state.get_object_mut(youth).unwrap().is_transformed = true;
    state.get_object_mut(youth).unwrap().name = "Unholy Fiend".into();

    let life_before = state.players[0].life;
    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, youth, &reg);

    assert_eq!(state.players[0].life, life_before - 1);
}

// ── Screeching Bat ──────────────────────────────────────────

#[test]
fn screeching_bat_transforms_with_activated_ability() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);
    assert_eq!(state.get_object(bat).unwrap().power, Some(2));
    assert!(!state.get_object(bat).unwrap().is_transformed);

    // Add mana for the transform ability: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, bat, 0, &[], &reg);

    assert!(state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Stalking Vampire");
    assert_eq!(behavior.dynamic_pt(&state, bat), Some((5, 5)));
}

// ── Ludevic's Test Subject ──────────────────────────────────────────

#[test]
fn ludevics_test_subject_transforms_at_five_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let subject = named_creature(&mut state, &reg, "Ludevic's Test Subject", P0);
    assert_eq!(state.get_object(subject).unwrap().power, Some(0));

    let behavior = reg.get(state.get_object(subject).unwrap().card_id).unwrap();

    // Activate 4 times — should not transform yet.
    for _ in 0..4 {
        behavior.on_activate_ability(&mut state, subject, 0, &[], &reg);
    }
    assert!(!state.get_object(subject).unwrap().is_transformed);

    // 5th activation — should transform.
    behavior.on_activate_ability(&mut state, subject, 0, &[], &reg);
    assert!(state.get_object(subject).unwrap().is_transformed);
    assert_eq!(state.get_object(subject).unwrap().name, "Ludevic's Abomination");
    assert_eq!(behavior.dynamic_pt(&state, subject), Some((13, 13)));
}

// ── Thraben Sentry ──────────────────────────────────────────

#[test]
fn thraben_sentry_transforms_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sentry = named_creature(&mut state, &reg, "Thraben Sentry", P0);
    let other = ready_creature(&mut state, P0, 1, 1);

    assert!(!state.get_object(sentry).unwrap().is_transformed);

    // Simulate another creature dying.
    let behavior = reg.get(state.get_object(sentry).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, sentry, other, P0, &[], 1, &reg);

    assert!(state.get_object(sentry).unwrap().is_transformed);
    assert_eq!(state.get_object(sentry).unwrap().name, "Thraben Militia");
    assert_eq!(behavior.dynamic_pt(&state, sentry), Some((5, 4)));
}

#[test]
fn thraben_sentry_does_not_transform_when_opponent_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sentry = named_creature(&mut state, &reg, "Thraben Sentry", P0);
    let opp_creature = ready_creature(&mut state, P1, 1, 1);

    let behavior = reg.get(state.get_object(sentry).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, sentry, opp_creature, P1, &[], 1, &reg);

    // Should NOT transform.
    assert!(!state.get_object(sentry).unwrap().is_transformed);
}

// ── Bloodline Keeper ──────────────────────────────────────────

#[test]
fn bloodline_keeper_creates_vampire_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let keeper = named_creature(&mut state, &reg, "Bloodline Keeper", P0);

    let behavior = reg.get(state.get_object(keeper).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, keeper, 0, &[], &reg);

    // Should have a Vampire token.
    let bf = state.objects_in_zone(Zone::Battlefield, P0);
    let tokens: Vec<_> = bf.iter()
        .filter(|o| o.is_token && o.name == "Vampire")
        .collect();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].power, Some(2));
    assert_eq!(tokens[0].toughness, Some(2));
}

// ── Mikaeus, the Lunarch ──────────────────────────────────────────

#[test]
fn mikaeus_enters_with_x_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create Mikaeus with x_value = 3.
    let card_id = reg.get_id_by_name("Mikaeus, the Lunarch").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, Some(0), Some(0));
    state.get_object_mut(id).unwrap().name = "Mikaeus, the Lunarch".into();
    state.get_object_mut(id).unwrap().x_value = Some(3);

    let behavior = reg.get(card_id).unwrap();
    behavior.on_resolve(&mut state, id, &[], &reg);

    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_counter_count(id, CounterType::PlusOnePlusOne), 3);
}

#[test]
fn mikaeus_distributes_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mikaeus = named_creature(&mut state, &reg, "Mikaeus, the Lunarch", P0);
    // Give Mikaeus 2 +1/+1 counters.
    state.add_counters(mikaeus, CounterType::PlusOnePlusOne, 2);

    let other1 = ready_creature(&mut state, P0, 2, 2);
    let other2 = ready_creature(&mut state, P0, 1, 1);

    // Use ability 1: remove a counter, give +1/+1 to each other creature.
    let behavior = reg.get(state.get_object(mikaeus).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, mikaeus, 1, &[], &reg);

    // Mikaeus should have lost a counter.
    assert_eq!(state.get_counter_count(mikaeus, CounterType::PlusOnePlusOne), 1);
    // Other creatures should each have a counter.
    assert_eq!(state.get_counter_count(other1, CounterType::PlusOnePlusOne), 1);
    assert_eq!(state.get_counter_count(other2, CounterType::PlusOnePlusOne), 1);
}

// ── Grimgrin, Corpse-Born ──────────────────────────────────────────

#[test]
fn grimgrin_enters_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimgrin, Corpse-Born").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, Some(5), Some(5));
    state.get_object_mut(id).unwrap().name = "Grimgrin, Corpse-Born".into();

    let behavior = reg.get(card_id).unwrap();
    behavior.on_resolve(&mut state, id, &[], &reg);

    assert!(state.get_object(id).unwrap().tapped);
    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
}

#[test]
fn grimgrin_sacrifice_untaps_and_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.get_object_mut(grimgrin).unwrap().tapped = true;

    let zombie = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, grimgrin, 0, &[], &reg);

    // Grimgrin should be untapped.
    assert!(!state.get_object(grimgrin).unwrap().tapped);
    // Grimgrin should have a +1/+1 counter.
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1);
    // Zombie should be dead.
    assert_eq!(state.get_object(zombie).unwrap().zone, Zone::Graveyard);
}

// ── Geist of Saint Traft ──────────────────────────────────────────

#[test]
fn geist_creates_angel_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);
    state.combat.as_mut().unwrap().attackers.insert(geist, P1);

    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, geist, &reg);

    // Should have an Angel token.
    let bf = state.objects_in_zone(Zone::Battlefield, P0);
    let angels: Vec<_> = bf.iter()
        .filter(|o| o.is_token && o.name == "Angel")
        .collect();
    assert_eq!(angels.len(), 1);
    assert_eq!(angels[0].power, Some(4));
    assert_eq!(angels[0].toughness, Some(4));
    assert!(angels[0].tapped);
}

#[test]
fn geist_angel_exiled_at_end_step() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    state.combat = Some(mtg_engine::state::CombatState::new());

    let geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);
    state.combat.as_mut().unwrap().attackers.insert(geist, P1);

    // Attack to create the angel.
    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, geist, &reg);

    let angel_id = state.objects_in_zone(Zone::Battlefield, P0)
        .iter()
        .find(|o| o.is_token && o.name == "Angel")
        .map(|o| o.id)
        .unwrap();

    // End step — angel should be exiled.
    behavior.on_end_step(&mut state, geist, &reg);
    assert_eq!(state.get_object(angel_id).unwrap().zone, Zone::Exile);
}

// ── Evil Twin ──────────────────────────────────────────

#[test]
fn evil_twin_copies_creature_on_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _opponent_creature = named_creature(&mut state, &reg, "Grizzly Bears", P1);
    let twin = named_creature(&mut state, &reg, "Evil Twin", P0);

    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &reg);

    // Evil Twin should have copied Grizzly Bears stats.
    assert_eq!(state.get_object(twin).unwrap().name, "Grizzly Bears");
    assert_eq!(state.get_object(twin).unwrap().power, Some(2));
    assert_eq!(state.get_object(twin).unwrap().toughness, Some(2));
    // Should still have the Evil Twin marker.
    assert!(state.get_object(twin).unwrap().card_state.contains_key("is_evil_twin"));
}

// ── Moldgraf Monstrosity ──────────────────────────────────────────

#[test]
fn moldgraf_monstrosity_returns_creatures_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monstrosity = named_creature(&mut state, &reg, "Moldgraf Monstrosity", P0);

    // Put two creatures in P0's graveyard.
    let gy1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(gy1).unwrap().name = "Creature 1".into();
    state.move_object(gy1, Zone::Graveyard);
    let gy2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(gy2).unwrap().name = "Creature 2".into();
    state.move_object(gy2, Zone::Graveyard);

    // Trigger death.
    let behavior = reg.get(state.get_object(monstrosity).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, monstrosity, &reg);

    // Monstrosity should be exiled.
    assert_eq!(state.get_object(monstrosity).unwrap().zone, Zone::Exile);
    // Both graveyard creatures should be on the battlefield.
    assert_eq!(state.get_object(gy1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy2).unwrap().zone, Zone::Battlefield);
}

// ── Liliana of the Veil ──────────────────────────────────────────

#[test]
fn liliana_enters_with_loyalty() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(id).unwrap().name = "Liliana of the Veil".into();

    let behavior = reg.get(card_id).unwrap();
    behavior.on_resolve(&mut state, id, &[], &reg);

    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_counter_count(id, CounterType::Loyalty), 3);
}

#[test]
fn liliana_plus_one_each_player_discards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    // Give both players cards in hand.
    let p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &reg);

    // Both players should have lost a card.
    assert_eq!(state.get_object(p0_card).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_two_opponent_sacrifices_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);

    let opp_creature = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &reg);

    // Opponent's creature should be dead.
    assert_eq!(state.get_object(opp_creature).unwrap().zone, Zone::Graveyard);
}

// ── Garruk Relentless ──────────────────────────────────────────

#[test]
fn garruk_creates_wolf_token() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 1, &reg);

    let bf = state.objects_in_zone(Zone::Battlefield, P0);
    let wolves: Vec<_> = bf.iter()
        .filter(|o| o.is_token && o.name == "Wolf")
        .collect();
    assert_eq!(wolves.len(), 1);
    assert_eq!(wolves[0].power, Some(2));
}

#[test]
fn garruk_transforms_at_two_or_fewer_loyalty() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 2); // Only 2 loyalty.
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // Use the wolf token ability (costs 0 loyalty).
    behavior.on_loyalty_ability(&mut state, garruk, 1, &reg);

    // Should have transformed.
    assert!(state.get_object(garruk).unwrap().is_transformed);
    assert_eq!(state.get_object(garruk).unwrap().name, "Garruk, the Veil-Cursed");
}

// ── Essence of the Wild ──────────────────────────────────────────

#[test]
fn essence_overrides_entering_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = named_creature(&mut state, &reg, "Essence of the Wild", P0);

    // Simulate another creature entering.
    let bear = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(bear).unwrap().name = "Grizzly Bears".into();

    let behavior = reg.get(state.get_object(essence).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, essence, bear, P0, &reg);

    // Bear should now be a 6/6 Essence copy.
    assert_eq!(state.get_object(bear).unwrap().power, Some(6));
    assert_eq!(state.get_object(bear).unwrap().toughness, Some(6));
    assert_eq!(state.get_object(bear).unwrap().name, "Essence of the Wild");
}

#[test]
fn essence_does_not_override_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let essence = named_creature(&mut state, &reg, "Essence of the Wild", P0);

    let opp_bear = ready_creature(&mut state, P1, 2, 2);

    let behavior = reg.get(state.get_object(essence).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, essence, opp_bear, P1, &reg);

    // Opponent's creature should be unchanged.
    assert_eq!(state.get_object(opp_bear).unwrap().power, Some(2));
}

// ── Mirror-Mad Phantasm ──────────────────────────────────────────

#[test]
fn mirror_mad_phantasm_mills_to_find_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let phantasm = named_creature(&mut state, &reg, "Mirror-Mad Phantasm", P0);

    // Set up library: some creatures, then Mirror-Mad Phantasm at the bottom.
    let card1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(card1, Zone::Library);
    let card2 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(card2, Zone::Library);
    state.players[0].library_order = vec![card1, card2];
    // Note: the phantasm will be shuffled into the library by the ability.

    let behavior = reg.get(state.get_object(phantasm).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, phantasm, 0, &[], &reg);

    // card1 and card2 should be milled (in graveyard).
    assert_eq!(state.get_object(card1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(card2).unwrap().zone, Zone::Graveyard);
    // Phantasm should be on the battlefield.
    assert_eq!(state.get_object(phantasm).unwrap().zone, Zone::Battlefield);
}

// ── Grimoire of the Dead ──────────────────────────────────────────

#[test]
fn grimoire_accumulates_study_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 cards to discard.
    let _c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let _c2 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    let _c3 = spell_in_hand(&mut state, &reg, "Giant Growth", P0);

    let behavior = reg.get(card_id).unwrap();

    // Activate 3 times.
    behavior.on_activate_ability(&mut state, grimoire, 0, &[], &reg);
    let counters = state.get_object(grimoire).unwrap().card_state.get("study_counters")
        .map(|id| id.0 as u32).unwrap_or(0);
    assert_eq!(counters, 1);

    behavior.on_activate_ability(&mut state, grimoire, 0, &[], &reg);
    behavior.on_activate_ability(&mut state, grimoire, 0, &[], &reg);

    let counters = state.get_object(grimoire).unwrap().card_state.get("study_counters")
        .map(|id| id.0 as u32).unwrap_or(0);
    assert_eq!(counters, 3);
}

#[test]
fn grimoire_reanimates_all_graveyard_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    state.get_object_mut(grimoire).unwrap().card_state.insert("study_counters".into(),
        mtg_engine::ids::ObjectId(3));

    // Put creatures in both graveyards.
    let gy1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(gy1).unwrap().name = "Creature A".into();
    state.move_object(gy1, Zone::Graveyard);

    let gy2 = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(gy2).unwrap().name = "Creature B".into();
    state.move_object(gy2, Zone::Graveyard);

    let behavior = reg.get(card_id).unwrap();
    behavior.on_activate_ability(&mut state, grimoire, 1, &[], &reg);

    // Both creatures should be on the battlefield under P0's control.
    assert_eq!(state.get_object(gy1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy1).unwrap().controller, P0);
    assert_eq!(state.get_object(gy2).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy2).unwrap().controller, P0);
    // They should have the Zombie subtype.
    assert!(state.get_object(gy2).unwrap().subtypes.contains(&"Zombie".into()));
    // Grimoire should be sacrificed (in graveyard).
    assert_eq!(state.get_object(grimoire).unwrap().zone, Zone::Graveyard);
}

// ── Civilized Scholar ──────────────────────────────────────────

#[test]
fn civilized_scholar_draw_discard_creature_transforms() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scholar = named_creature(&mut state, &reg, "Civilized Scholar", P0);

    // Put a card in the library (will be drawn).
    let lib_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(lib_card, Zone::Library);
    state.players[0].library_order = vec![lib_card];

    // Put a creature in hand (will be discarded).
    let _hand_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, scholar, 0, &[], &reg);

    // Should transform (discarded a creature).
    assert!(state.get_object(scholar).unwrap().is_transformed);
    assert_eq!(state.get_object(scholar).unwrap().name, "Homicidal Brute");
    // Should be untapped (was untapped after discard).
    assert!(!state.get_object(scholar).unwrap().tapped);
}

// ── Planeswalker SBA ──────────────────────────────────────────

#[test]
fn planeswalker_with_zero_loyalty_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.get_object_mut(liliana).unwrap().card_types = vec![CardType::Planeswalker];
    // 0 loyalty counters.

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Graveyard);
}

#[test]
fn planeswalker_with_loyalty_survives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.get_object_mut(liliana).unwrap().card_types = vec![CardType::Planeswalker];
    state.add_counters(liliana, CounterType::Loyalty, 3);

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Battlefield);
}

// ── Loyalty ability engine integration ──────────────────────────────────────────

#[test]
fn loyalty_abilities_appear_in_legal_actions() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.get_object_mut(liliana).unwrap().card_types = vec![CardType::Planeswalker];
    state.add_counters(liliana, CounterType::Loyalty, 3);

    let legal = engine::legal_actions(&state, &reg);

    // Should have loyalty ability actions.
    let loyalty_actions: Vec<_> = legal.actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { .. }))
        .collect();
    // +1 and -2 should be available (not -6, since loyalty is only 3).
    assert!(loyalty_actions.len() >= 2, "Expected at least 2 loyalty abilities, got {}", loyalty_actions.len());
}

#[test]
fn loyalty_ability_adjusts_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    state.get_object_mut(liliana).unwrap().card_types = vec![CardType::Planeswalker];
    state.add_counters(liliana, CounterType::Loyalty, 3);

    // Give both players cards to discard.
    let _p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let _p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    // Activate +1.
    let new_state = engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana,
        ability_index: 0,
    }, &reg);

    // Loyalty should be 4 (3 + 1).
    assert_eq!(new_state.get_counter_count(liliana, CounterType::Loyalty), 4);
}
