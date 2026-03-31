//! Tests for Tier 15 medium-complexity Innistrad cards.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::CardId;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Curse of Stalked Prey ────────────────────────────────────────

#[test]
fn curse_of_stalked_prey_gives_counter_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Place the curse on the battlefield attached to P1.
    let curse = named_creature(&mut state, &reg, "Curse of Stalked Prey", P0);
    if let Some(obj) = state.get_object_mut(curse) {
        obj.attached_to_player = Some(P1);
    }

    // Place an attacking creature.
    let attacker = ready_creature(&mut state, P0, 2, 2);

    // Simulate combat damage to P1.
    let behavior = reg.get(state.get_object(curse).unwrap().card_id).unwrap();
    behavior.on_any_combat_damage_to_player(&mut state, curse, attacker, P1, 2, &reg);

    // The attacker should have a +1/+1 counter.
    let counters = state.get_object(attacker).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 1, "Attacker should get a +1/+1 counter");
}

// ── Dearly Departed ──────────────────────────────────────────────

#[test]
fn dearly_departed_gives_counter_to_entering_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Dearly Departed in the graveyard.
    let dd = named_creature(&mut state, &reg, "Dearly Departed", P0);
    state.move_object(dd, Zone::Graveyard);

    // Place a Human creature on the battlefield (Champion of the Parish is a Human).
    let human = named_creature(&mut state, &reg, "Champion of the Parish", P0);

    // Trigger "any creature enters" on the Dearly Departed.
    let behavior = reg.get(state.get_object(dd).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, dd, human, P0, &reg);

    let counters = state.get_object(human).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 1, "Human should enter with a +1/+1 counter from Dearly Departed");
}

// ── Mentor of the Meek ───────────────────────────────────────────

#[test]
fn mentor_of_the_meek_draws_when_small_creature_enters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_creature(&mut state, &reg, "Mentor of the Meek", P0);

    // Give P0 mana to pay {1}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Put a card in library to draw.
    let lib_card = state.create_object(CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    // A 1/1 creature enters under our control.
    let small_creature = ready_creature(&mut state, P0, 1, 1);

    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small_creature, P0, &reg);

    // Should have drawn a card.
    let hand_count = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_count, 1, "Mentor of the Meek should have drawn a card");

    // Mana should be spent.
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "Should have spent 1 mana");
}

// ── Kessig Cagebreakers ─────────────────────────────────────────

#[test]
fn kessig_cagebreakers_creates_wolf_tokens_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let cage = named_creature(&mut state, &reg, "Kessig Cagebreakers", P0);

    // Set up combat: Kessig Cagebreakers is attacking P1.
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(cage, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    // Put 3 creatures in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.move_object(c, Zone::Graveyard);
    }

    let behavior = reg.get(state.get_object(cage).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, cage, &reg);

    // Should have 3 Wolf tokens on the battlefield.
    let wolves = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Wolf" && o.is_token)
        .count();
    assert_eq!(wolves, 3, "Should have created 3 Wolf tokens");

    // Wolves should be tapped and attacking.
    for wolf in state.objects.values().filter(|o| o.zone == Zone::Battlefield && o.name == "Wolf") {
        assert!(wolf.tapped, "Wolf tokens should be tapped");
    }
    let combat_attackers = state.combat.as_ref().unwrap().attackers.len();
    // Cage + 3 wolves = 4 attackers.
    assert_eq!(combat_attackers, 4, "Should have 4 attackers (cage + 3 wolves)");
}

// ── Galvanic Juggernaut ──────────────────────────────────────────

#[test]
fn galvanic_juggernaut_untaps_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let jug = named_creature(&mut state, &reg, "Galvanic Juggernaut", P0);
    // Tap it.
    state.get_object_mut(jug).unwrap().tapped = true;

    // A creature dies.
    let dead = ready_creature(&mut state, P1, 1, 1);
    let behavior = reg.get(state.get_object(jug).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, jug, dead, P1, &[], 1, &reg);

    assert!(!state.get_object(jug).unwrap().tapped, "Galvanic Juggernaut should untap when a creature dies");
}

// ── Creepy Doll ──────────────────────────────────────────────────

#[test]
fn creepy_doll_is_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doll = named_creature(&mut state, &reg, "Creepy Doll", P0);

    // Verify it has Indestructible.
    assert!(state.has_keyword(doll, Keyword::Indestructible, &reg),
        "Creepy Doll should have Indestructible");
}

// ── Bitterheart Witch ────────────────────────────────────────────

#[test]
fn bitterheart_witch_finds_curse_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_creature(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // Trigger death.
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &reg);

    // The curse should be on the battlefield attached to opponent.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Battlefield, "Curse should be on battlefield");
    assert_eq!(curse.attached_to_player, Some(P1), "Curse should be attached to opponent");
}

// ── Gutter Grime ─────────────────────────────────────────────────

#[test]
fn gutter_grime_creates_ooze_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_creature(&mut state, &reg, "Gutter Grime", P0);
    // Fix: Gutter Grime is an enchantment, not a creature. Clear creature stats.
    if let Some(obj) = state.get_object_mut(grime) {
        obj.power = None;
        obj.toughness = None;
    }

    // A nontoken creature we control dies.
    let dead = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(grime).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, grime, dead, P0, &[], 2, &reg);

    // Should have a counter on Gutter Grime.
    let counters = state.get_object(grime).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 1, "Gutter Grime should have 1 slime counter");

    // Should have created an Ooze token.
    let oozes = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Ooze" && o.is_token)
        .count();
    assert_eq!(oozes, 1, "Should have created 1 Ooze token");
}

// ── Heretic's Punishment ─────────────────────────────────────────

#[test]
fn heretics_punishment_deals_damage_from_revealed_cards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_creature(&mut state, &reg, "Heretic's Punishment", P0);
    if let Some(obj) = state.get_object_mut(hp) {
        obj.power = None;
        obj.toughness = None;
    }

    // Put cards in library. Use a known card so mana value is deterministic.
    // Kalonian Tusker costs {G}{G} = MV 2.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    let initial_life = state.get_player(P1).life;
    let behavior = reg.get(state.get_object(hp).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, hp, 0, &[mtg_engine::actions::Target::Player(P1)], &reg);

    // Should have dealt 2 damage (MV of Kalonian Tusker).
    let new_life = state.get_player(P1).life;
    assert_eq!(initial_life - new_life, 2, "Should deal damage equal to greatest MV (2)");
}

// ── Undead Alchemist ─────────────────────────────────────────────

#[test]
fn undead_alchemist_mills_instead_of_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let alchemist = named_creature(&mut state, &reg, "Undead Alchemist", P0);

    // Create a Zombie that dealt damage.
    let zombie = state.create_token_with_subtypes(
        "Zombie", P0, 2, 2,
        vec![Color::Black], vec![CardType::Creature], vec![],
        vec!["Zombie".into()],
    );
    state.get_object_mut(zombie).unwrap().summoning_sick = false;

    // Put creature cards in P1's library so milling creates tokens.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..2 {
        let card = state.create_object(tusker_id, P1, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P1).library_order.insert(0, card);
    }

    let initial_life = state.get_player(P1).life;

    // Simulate: Zombie dealt 2 combat damage to P1.
    // First, reduce life (as combat damage normally does), then the trigger restores it.
    state.get_player_mut(P1).life = initial_life - 2;

    let behavior = reg.get(state.get_object(alchemist).unwrap().card_id).unwrap();
    behavior.on_any_combat_damage_to_player(&mut state, alchemist, zombie, P1, 2, &reg);

    // Life should be restored (damage was replaced by mill).
    assert_eq!(state.get_player(P1).life, initial_life, "Life should be restored");

    // The creature cards should have been exiled (not in graveyard).
    let p1_exile = state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P1)
        .count();
    assert_eq!(p1_exile, 2, "Milled creature cards should be exiled");

    // Should have created Zombie tokens.
    let zombie_tokens = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Zombie" && o.is_token && o.controller == P0)
        .count();
    // 2 creature cards milled = 2 Zombie tokens + the original zombie we created.
    assert!(zombie_tokens >= 2, "Should create Zombie tokens for each milled creature");
}

// ── Creeping Renaissance ─────────────────────────────────────────

#[test]
fn creeping_renaissance_returns_creatures_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put creature cards in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.move_object(c, Zone::Graveyard);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    let new_state = cast_and_resolve(&state, &reg, spell, vec![]);

    // All 3 creatures should be in hand now.
    let hand_creatures = new_state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.power.is_some())
        .count();
    assert_eq!(hand_creatures, 3, "Should return all creature cards from graveyard to hand");
}

// ── Cellar Door ──────────────────────────────────────────────────

#[test]
fn cellar_door_creates_zombie_when_milling_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let door = named_creature(&mut state, &reg, "Cellar Door", P0);
    if let Some(obj) = state.get_object_mut(door) {
        obj.power = None;
        obj.toughness = None;
    }

    // Put a creature card on top of P1's library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let card = state.create_object(tusker_id, P1, Zone::Library, Some(3), Some(3));
    state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
    state.get_player_mut(P1).library_order.insert(0, card);

    let behavior = reg.get(state.get_object(door).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, door, 0, &[mtg_engine::actions::Target::Player(P1)], &reg);

    // Should have created a Zombie token (since a creature was milled).
    let zombies = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Zombie" && o.is_token)
        .count();
    assert_eq!(zombies, 1, "Should create a Zombie token when milling a creature");
}

// ── Skaab Ruinator ───────────────────────────────────────────────

#[test]
fn skaab_ruinator_exiles_creatures_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 creature cards in graveyard for the additional cost.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard);
    }

    let spell = castable_spell(&mut state, &reg, "Skaab Ruinator", P0);
    let new_state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Skaab Ruinator should be on the battlefield.
    let on_bf = new_state.objects.values()
        .any(|o| o.zone == Zone::Battlefield && o.name == "Skaab Ruinator");
    assert!(on_bf, "Skaab Ruinator should be on the battlefield");

    // 3 creatures should be exiled.
    let exiled = new_state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P0)
        .count();
    assert_eq!(exiled, 3, "Should exile 3 creatures from graveyard");
}

// ── Manor Gargoyle ───────────────────────────────────────────────

#[test]
fn manor_gargoyle_loses_defender_and_gains_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_creature(&mut state, &reg, "Manor Gargoyle", P0);

    // Should start with Defender.
    assert!(state.has_keyword(gargoyle, Keyword::Defender, &reg),
        "Manor Gargoyle should start with Defender");

    // Activate ability.
    let behavior = reg.get(state.get_object(gargoyle).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, gargoyle, 0, &[], &reg);

    // Should have lost Defender.
    let obj = state.get_object(gargoyle).unwrap();
    assert!(!obj.keywords.contains(&Keyword::Defender),
        "Manor Gargoyle should lose Defender after activation");

    // Should have gained Flying (as until-end-of-turn keyword).
    let has_flying = state.until_end_of_turn_keywords.iter()
        .any(|k| k.target == gargoyle && k.keyword == Keyword::Flying);
    assert!(has_flying, "Manor Gargoyle should gain Flying until end of turn");
}

// ── Tree of Redemption ───────────────────────────────────────────

#[test]
fn tree_of_redemption_swaps_life_and_toughness() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_creature(&mut state, &reg, "Tree of Redemption", P0);
    // P0 starts at 20 life, Tree base toughness is 13.

    let behavior = reg.get(state.get_object(tree).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, tree, 0, &[], &reg);

    // Life should now be 13 (Tree's toughness).
    assert_eq!(state.get_player(P0).life, 13, "Life should become Tree's toughness (13)");

    // Tree's base toughness should now be 20 (old life).
    assert_eq!(state.get_object(tree).unwrap().toughness, Some(20),
        "Tree's toughness should become old life total (20)");
}

// ── Unbreathing Horde ────────────────────────────────────────────

#[test]
fn unbreathing_horde_enters_with_counters_for_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 2 Zombie tokens on the battlefield.
    for _ in 0..2 {
        state.create_token_with_subtypes(
            "Zombie", P0, 2, 2,
            vec![Color::Black], vec![CardType::Creature], vec![],
            vec!["Zombie".into()],
        );
    }

    // Put 1 Zombie card in graveyard.
    let gy_zombie = named_creature(&mut state, &reg, "Diregraf Ghoul", P0);
    state.move_object(gy_zombie, Zone::Graveyard);

    // Now place Unbreathing Horde on the battlefield.
    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    let behavior = reg.get(state.get_object(horde).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, horde, &reg);

    // Should have 3 counters (2 battlefield + 1 graveyard).
    let counters = state.get_object(horde).unwrap()
        .counters.get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 3, "Unbreathing Horde should enter with 3 +1/+1 counters");
}

// ── Back from the Brink ──────────────────────────────────────────

#[test]
fn back_from_the_brink_creates_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_creature(&mut state, &reg, "Back from the Brink", P0);
    if let Some(obj) = state.get_object_mut(enchant) {
        obj.power = None;
        obj.toughness = None;
    }

    // Put a creature in graveyard.
    let dead = named_creature(&mut state, &reg, "Kalonian Tusker", P0);
    state.move_object(dead, Zone::Graveyard);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, enchant, 0, &[], &reg);

    // The creature should be exiled.
    assert_eq!(state.get_object(dead).unwrap().zone, Zone::Exile,
        "Original creature should be exiled");

    // A token copy should be on the battlefield.
    let token_copies = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.is_token && o.name == "Kalonian Tusker")
        .count();
    assert_eq!(token_copies, 1, "Should have created a token copy");
}
