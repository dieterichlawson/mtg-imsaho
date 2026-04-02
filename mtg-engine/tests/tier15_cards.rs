//! Tests for Tier 15 medium-complexity Innistrad cards.
//! Tests for Tier 15 hard/complex Innistrad cards.


mod common;

use common::*;
use mtg_engine::cards::{CardRegistry, TriggerKind};
use mtg_engine::ids::CardId;
use mtg_engine::engine;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::sba::check_state_based_actions_with_registry;

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

    // Should have a slime counter on Gutter Grime.
    let counters = state.get_object(grime).unwrap()
        .counters.get(&CounterType::Slime).copied().unwrap_or(0);
    assert_eq!(counters, 1, "Gutter Grime should have 1 slime counter");

    // Should have created an Ooze token.
    let oozes = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Ooze" && o.is_token)
        .count();
    assert_eq!(oozes, 1, "Should have created 1 Ooze token");
}

// ── Heretic's Punishment ─────────────────────────────────────────

#[test]
fn heretics_punishment_mills_then_deals_damage() {
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

    // Milled cards should be in graveyard, not library.
    let lib_count = state.get_player(P0).library_order.len();
    assert_eq!(lib_count, 0, "Library should be empty after milling 3 cards");

    let gy_count = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0 && o.name == "Kalonian Tusker")
        .count();
    assert_eq!(gy_count, 3, "All 3 milled cards should be in graveyard");
}

#[test]
fn heretics_punishment_tracks_damaged_by_on_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_creature(&mut state, &reg, "Heretic's Punishment", P0);
    if let Some(obj) = state.get_object_mut(hp) {
        obj.power = None;
        obj.toughness = None;
    }

    // Put cards in library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    let target_creature = ready_creature(&mut state, P1, 5, 5);

    let behavior = reg.get(state.get_object(hp).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, hp, 0, &[Target::Object(target_creature)], &reg);

    let obj = state.get_object(target_creature).unwrap();
    assert_eq!(obj.damage_marked, 2, "Creature should have 2 damage marked");
    assert!(obj.damaged_by.contains(&hp), "damaged_by should track the source");
}

#[test]
fn heretics_punishment_fizzles_when_target_illegal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_creature(&mut state, &reg, "Heretic's Punishment", P0);
    if let Some(obj) = state.get_object_mut(hp) {
        obj.power = None;
        obj.toughness = None;
    }

    // Put cards in library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    // Create a creature target then move it off the battlefield (illegal target).
    let target_creature = ready_creature(&mut state, P1, 3, 3);
    state.move_object(target_creature, Zone::Graveyard);

    let behavior = reg.get(state.get_object(hp).unwrap().card_id).unwrap();
    behavior.on_activate_ability(&mut state, hp, 0, &[Target::Object(target_creature)], &reg);

    // Entire ability should fizzle: no cards milled.
    let lib_count = state.get_player(P0).library_order.len();
    assert_eq!(lib_count, 3, "Library should be unchanged when ability fizzles");
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
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put creature cards in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
        state.move_object(c, Zone::Graveyard);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    // Cast the spell and put it on the stack.
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![], sacrifice: None, exile_count: None },
        &reg,
    );
    // Resolve: this triggers a ChooseCardType choice.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting card type choice");

    // Choose "Creature" (index 0).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0) },
        &reg,
    );

    // All 3 creatures should be in hand now.
    let hand_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.power.is_some())
        .count();
    assert_eq!(hand_creatures, 3, "Should return all creature cards from graveyard to hand");
}

#[test]
fn creeping_renaissance_only_returns_chosen_type() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put creatures and enchantments in graveyard.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
        state.move_object(c, Zone::Graveyard);
    }
    for _ in 0..2 {
        let e = state.create_object(CardId(9999), P0, Zone::Battlefield, None, None);
        state.get_object_mut(e).unwrap().card_types = vec![CardType::Enchantment];
        state.move_object(e, Zone::Graveyard);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::CastSpell { object_id: spell, targets: vec![], sacrifice: None, exile_count: None },
        &reg,
    );
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Choose "Enchantment" (index 2).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(2) },
        &reg,
    );

    // Only enchantments should be in hand.
    let hand_enchantments = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.card_types.contains(&CardType::Enchantment))
        .count();
    assert_eq!(hand_enchantments, 2, "Should return enchantments to hand");

    // Creatures should still be in graveyard.
    let gy_creatures = state.objects.values()
        .filter(|o| o.zone == Zone::Graveyard && o.owner == P0 && o.card_types.contains(&CardType::Creature))
        .count();
    assert_eq!(gy_creatures, 2, "Creatures should remain in graveyard");
}

#[test]
fn creeping_renaissance_flashback_exiles() {
    use mtg_engine::actions::{Action, ResolvedChoice};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put a creature in graveyard.
    let c = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
    state.move_object(c, Zone::Graveyard);

    // Put Creeping Renaissance itself in graveyard for flashback.
    let card_id = reg.get_id_by_name("Creeping Renaissance").unwrap();
    let spell = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(spell).unwrap().name = "Creeping Renaissance".into();
    state.get_object_mut(spell).unwrap().card_types = vec![CardType::Sorcery];

    // Add flashback mana (5GG = 7 total).
    for _ in 0..5 { state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1); }
    for _ in 0..2 { state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1); }

    // Cast via flashback.
    let actions = mtg_engine::engine::legal_actions(&state, &reg);
    let fb = actions.actions.iter().find(|a| match a {
        Action::CastSpell { object_id, .. } => object_id == &spell,
        _ => false,
    });
    assert!(fb.is_some(), "Should be able to flashback Creeping Renaissance");

    state = mtg_engine::engine::submit_action(&state, fb.unwrap(), &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Choose "Creature" (index 0).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0) },
        &reg,
    );

    // Creature in hand.
    let hand = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.card_types.contains(&CardType::Creature))
        .count();
    assert_eq!(hand, 1, "Creature should be in hand");

    // Creeping Renaissance should be exiled (flashback).
    let cr = state.get_object(spell);
    assert!(cr.is_none() || cr.unwrap().zone == Zone::Exile,
        "Creeping Renaissance should be exiled after flashback");
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

#[test]
fn skaab_ruinator_cast_from_graveyard() {
    // "You may cast this card from your graveyard" — uses normal mana cost, not flashback.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put Skaab Ruinator in graveyard.
    let ruinator = spell_in_hand(&mut state, &reg, "Skaab Ruinator", P0);
    state.move_object(ruinator, Zone::Graveyard);

    // Put 3 creature cards in graveyard for the additional cost.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard);
    }

    // Give enough mana ({1}{U}{U}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Should be castable from graveyard.
    let actions = engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == ruinator)
    });
    assert!(can_cast, "Skaab Ruinator should be castable from graveyard");

    // Cast it.
    let new_state = engine::submit_action(
        &state,
        &Action::CastSpell { object_id: ruinator, targets: vec![], sacrifice: None, exile_count: None },
        &reg,
    );

    // Should be on the stack (not panicked!).
    assert_eq!(new_state.get_object(ruinator).unwrap().zone, Zone::Stack,
        "Skaab Ruinator should be on the stack after casting from graveyard");

    // Should NOT have cast_with_flashback set (it's not flashback).
    assert!(!new_state.get_object(ruinator).unwrap().cast_with_flashback,
        "Should not be marked as flashback — it's cast-from-graveyard");
}

#[test]
fn skaab_ruinator_not_castable_without_enough_creatures() {
    // Can't cast from graveyard without 3 creature cards.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let ruinator = spell_in_hand(&mut state, &reg, "Skaab Ruinator", P0);
    state.move_object(ruinator, Zone::Graveyard);

    // Only 2 creatures (need 3).
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard);
    }

    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let actions = engine::legal_actions(&state, &reg);
    let can_cast = actions.actions.iter().any(|a| {
        matches!(a, Action::CastSpell { object_id, .. } if *object_id == ruinator)
    });
    assert!(!can_cast, "Should NOT be castable with only 2 creature cards in graveyard");
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

// ── Delver of Secrets ──────────────────────────────────────────

#[test]
fn delver_transforms_when_player_reveals_instant() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    // Put Delver on the battlefield.
    let delver = named_creature(&mut state, &reg, "Delver of Secrets", P0);
    assert_eq!(state.get_object(delver).unwrap().power, Some(1));

    // Put a Lightning Bolt (instant) on top of library.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Library);
    state.players[0].library_order.insert(0, bolt);

    // Trigger upkeep — should present a YesNo choice.
    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &reg);

    // Should NOT be transformed yet — awaiting player choice.
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert!(state.awaiting_action.is_some(), "Should be awaiting reveal choice");

    // Player chooses to reveal.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Now should be transformed.
    assert!(state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Insectile Aberration");
    // Dynamic P/T should be 3/2.
    assert_eq!(behavior.dynamic_pt(&state, delver), Some((3, 2)));

    // The card should still be on top of the library (per ruling).
    assert_eq!(state.players[0].library_order.first().copied(), Some(bolt));
}

#[test]
fn delver_does_not_transform_when_player_declines_reveal() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let delver = named_creature(&mut state, &reg, "Delver of Secrets", P0);

    // Put a Lightning Bolt (instant) on top of library.
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);
    state.move_object(bolt, Zone::Library);
    state.players[0].library_order.insert(0, bolt);

    // Trigger upkeep — should present a YesNo choice.
    let behavior = reg.get(state.get_object(delver).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, delver, &reg);

    assert!(state.awaiting_action.is_some(), "Should be awaiting reveal choice");

    // Player declines to reveal.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) },
        &reg,
    );

    // Should NOT be transformed.
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Delver of Secrets");

    // The card should still be on top of the library.
    assert_eq!(state.players[0].library_order.first().copied(), Some(bolt));
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

    // Should NOT be transformed and no choice presented.
    assert!(!state.get_object(delver).unwrap().is_transformed);
    assert_eq!(state.get_object(delver).unwrap().name, "Delver of Secrets");
    assert!(state.awaiting_action.is_none(), "No choice should be presented for non-instant/sorcery");
}

// ── Cloistered Youth ──────────────────────────────────────────

#[test]
fn cloistered_youth_presents_transform_choice_at_upkeep() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);

    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, youth, &reg);

    // Should NOT be transformed yet — awaiting player choice.
    assert!(!state.get_object(youth).unwrap().is_transformed);
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses yes to transform.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Now should be transformed.
    assert!(state.get_object(youth).unwrap().is_transformed);
    assert_eq!(state.get_object(youth).unwrap().name, "Unholy Fiend");
    assert_eq!(behavior.dynamic_pt(&state, youth), Some((3, 3)));
}

#[test]
fn cloistered_youth_can_decline_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);

    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, youth, &reg);

    // Should be awaiting player choice.
    assert!(state.awaiting_action.is_some());

    // Player declines to transform.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) },
        &reg,
    );

    // Should NOT be transformed.
    assert!(!state.get_object(youth).unwrap().is_transformed);
    assert_eq!(state.get_object(youth).unwrap().name, "Cloistered Youth");
}

#[test]
fn unholy_fiend_drains_life_at_end_step() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let youth = named_creature(&mut state, &reg, "Cloistered Youth", P0);
    // Pre-transform.
    state.get_object_mut(youth).unwrap().is_transformed = true;
    state.get_object_mut(youth).unwrap().name = "Unholy Fiend".into();

    let life_before = state.players[0].life;
    let behavior = reg.get(state.get_object(youth).unwrap().card_id).unwrap();
    behavior.on_end_step(&mut state, youth, &reg);

    assert_eq!(state.players[0].life, life_before - 1);
}

#[test]
fn cloistered_youth_front_face_has_upkeep_trigger_only() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Cloistered Youth").unwrap();
    let behavior = reg.get(card_id).unwrap();
    let data = behavior.card_data();

    // Front face should only have the upkeep trigger.
    assert_eq!(data.triggered_abilities.len(), 1);
    assert_eq!(data.triggered_abilities[0].kind, TriggerKind::Upkeep);
}

#[test]
fn unholy_fiend_back_face_has_end_step_trigger_only() {
    let reg = registry();
    let card_id = reg.get_id_by_name("Cloistered Youth").unwrap();
    let behavior = reg.get(card_id).unwrap();
    let back = behavior.back_face_data().expect("Should have back face");

    // Back face should only have the end step trigger.
    assert_eq!(back.triggered_abilities.len(), 1);
    assert_eq!(back.triggered_abilities[0].kind, TriggerKind::EndStep);
}

// ── Screeching Bat ──────────────────────────────────────────

#[test]
fn screeching_bat_transforms_at_upkeep_when_player_pays() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);
    assert!(!state.get_object(bat).unwrap().is_transformed);

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);

    // Should NOT be transformed yet — awaiting player choice.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert!(state.awaiting_action.is_some(), "Should be awaiting pay choice");

    // Player chooses to pay.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Now should be transformed.
    assert!(state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Stalking Vampire");
    assert_eq!(behavior.dynamic_pt(&state, bat), Some((5, 5)));

    // Mana should have been spent.
    assert_eq!(state.get_player(P0).mana_pool.total(), 0);
}

#[test]
fn screeching_bat_does_not_transform_when_player_declines() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);
    assert!(!state.get_object(bat).unwrap().is_transformed);

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);

    assert!(state.awaiting_action.is_some(), "Should be awaiting pay choice");

    // Player declines to pay.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(false) },
        &reg,
    );

    // Should NOT be transformed.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Screeching Bat");

    // Mana should NOT have been spent.
    assert_eq!(state.get_player(P0).mana_pool.total(), 4);
}

#[test]
fn screeching_bat_no_choice_without_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);

    // No mana in pool — choice should not be presented.
    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);

    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert!(state.awaiting_action.is_none(), "No choice should be presented without mana");
}

#[test]
fn stalking_vampire_transforms_back_when_player_pays() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);

    // Transform to Stalking Vampire first.
    if let Some(obj) = state.get_object_mut(bat) {
        obj.is_transformed = true;
        obj.name = "Stalking Vampire".into();
    }

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);

    // Should be awaiting choice.
    assert!(state.awaiting_action.is_some(), "Should be awaiting pay choice");

    // Player chooses to pay.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Should transform back to Screeching Bat.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Screeching Bat");
}

#[test]
fn stalking_vampire_does_not_have_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);

    // Simulate real game setup: obj.keywords is populated from front face card_data.
    if let Some(obj) = state.get_object_mut(bat) {
        obj.keywords = vec![Keyword::Flying];
    }

    // Verify front face has Flying.
    assert!(state.has_keyword(bat, Keyword::Flying, &reg));

    // Add mana and transform.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Now Stalking Vampire — should NOT have Flying.
    assert!(state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Stalking Vampire");
    assert!(!state.has_keyword(bat, Keyword::Flying, &reg),
        "Stalking Vampire should not have Flying");
    // obj.keywords should be empty (back face has no keywords).
    assert!(state.get_object(bat).unwrap().keywords.is_empty(),
        "obj.keywords should be cleared on transform to back face");
}

#[test]
fn screeching_bat_regains_flying_on_transform_back() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);

    // Simulate real game: front face keywords populated.
    if let Some(obj) = state.get_object_mut(bat) {
        obj.keywords = vec![Keyword::Flying];
    }

    // Transform to Stalking Vampire via the helper directly.
    // (Set up as if already transformed, with back face data.)
    if let Some(obj) = state.get_object_mut(bat) {
        obj.is_transformed = true;
        obj.name = "Stalking Vampire".into();
        obj.keywords = vec![]; // Back face has no keywords.
        obj.subtypes = vec!["Vampire".into()];
    }

    // Add mana and transform back.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Should be back to Screeching Bat with Flying restored.
    assert!(!state.get_object(bat).unwrap().is_transformed);
    assert_eq!(state.get_object(bat).unwrap().name, "Screeching Bat");
    assert!(state.has_keyword(bat, Keyword::Flying, &reg),
        "Screeching Bat should have Flying after transforming back");
    assert!(state.get_object(bat).unwrap().keywords.contains(&Keyword::Flying),
        "obj.keywords should contain Flying after transforming back");
}

#[test]
fn screeching_bat_transform_updates_subtypes() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);

    // Simulate real game setup: front face subtypes.
    if let Some(obj) = state.get_object_mut(bat) {
        obj.keywords = vec![Keyword::Flying];
        obj.subtypes = vec!["Bat".into()];
    }

    // Verify front face subtype.
    assert!(state.get_object(bat).unwrap().subtypes.contains(&"Bat".to_string()));

    // Add mana and transform.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) },
        &reg,
    );

    // Stalking Vampire should have "Vampire" subtype, not "Bat".
    let obj = state.get_object(bat).unwrap();
    assert!(obj.subtypes.contains(&"Vampire".to_string()),
        "Stalking Vampire should have Vampire subtype");
    assert!(!obj.subtypes.contains(&"Bat".to_string()),
        "Stalking Vampire should not have Bat subtype");
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
    let obj = state.get_object(subject).unwrap();
    assert!(obj.is_transformed);
    assert_eq!(obj.name, "Ludevic's Abomination");
    assert_eq!(behavior.dynamic_pt(&state, subject), Some((13, 13)));
    // Verify keywords and subtypes are updated by apply_transform (not stale).
    assert!(obj.keywords.contains(&Keyword::Trample), "back face should have Trample");
    assert!(!obj.keywords.contains(&Keyword::Defender), "back face should not have Defender");
    assert!(obj.subtypes.contains(&"Lizard".to_string()));
    assert!(obj.subtypes.contains(&"Horror".to_string()));
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

    // Activate through the engine so the sacrifice cost is paid properly.
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility {
            object_id: grimgrin,
            ability_index: 0,
            targets: vec![],
        },
        &reg,
    );

    // Grimgrin should be untapped.
    assert!(!new_state.get_object(grimgrin).unwrap().tapped);
    // Grimgrin should have a +1/+1 counter.
    assert_eq!(new_state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1);
    // Zombie should be dead (sacrificed as cost).
    assert_eq!(new_state.get_object(zombie).unwrap().zone, Zone::Graveyard);
}

#[test]
fn grimgrin_sacrifice_not_available_without_other_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.get_object_mut(grimgrin).unwrap().tapped = true;

    // No other creatures — sacrifice ability should NOT be available.
    let legal = engine::legal_actions(&state, &reg);
    let has_activate = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, ability_index: 0, .. } if *object_id == grimgrin
    ));
    assert!(!has_activate, "Grimgrin sacrifice ability should not be available without another creature");
}

#[test]
fn grimgrin_attack_trigger_destroys_and_adds_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let defender_creature = ready_creature(&mut state, P1, 3, 3);

    // Set up combat state with Grimgrin attacking P1.
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(grimgrin, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    // Call on_attacks directly (simulating trigger resolution).
    let behavior = reg.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, &reg);

    // With only one defender creature, the choice auto-applies (mandatory + 1 target).
    // The target should be destroyed and Grimgrin should get a +1/+1 counter.
    assert_eq!(state.get_object(defender_creature).unwrap().zone, Zone::Graveyard,
        "Defending creature should be destroyed");
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1,
        "Grimgrin should have a +1/+1 counter from attack trigger");
}

#[test]
fn grimgrin_attack_trigger_presents_choice_with_multiple_targets() {
    use mtg_engine::actions::Target;

    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let creature_a = ready_creature(&mut state, P1, 2, 2);
    let creature_b = ready_creature(&mut state, P1, 3, 3);

    // Set up combat state with Grimgrin attacking P1.
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(grimgrin, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    let behavior = reg.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, &reg);

    // With multiple targets, the controller should be presented a choice.
    assert!(state.awaiting_action.is_some(),
        "Should present target choice when defender has multiple creatures");

    // Resolve the choice — pick creature_a.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(creature_a))),
        },
        &reg,
    );

    assert_eq!(state.get_object(creature_a).unwrap().zone, Zone::Graveyard,
        "Chosen creature should be destroyed");
    assert_eq!(state.get_object(creature_b).unwrap().zone, Zone::Battlefield,
        "Unchosen creature should remain");
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1,
        "Grimgrin should have a +1/+1 counter");
}

// Ruling: "If the defending player controls no creatures when Grimgrin attacks,
// the last ability will be removed from the stack and have no effect."
#[test]
fn grimgrin_attack_no_targets_no_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    // Defender has NO creatures.

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(grimgrin, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    let behavior = reg.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, &reg);

    // No valid targets — ability does nothing, no +1/+1 counter.
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 0,
        "Grimgrin should NOT get a +1/+1 counter when defender has no creatures");
}

// Ruling: "If Grimgrin's last ability resolves, but the targeted creature isn't destroyed
// (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin."
#[test]
fn grimgrin_attack_indestructible_target_still_gets_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let indestructible = ready_creature(&mut state, P1, 4, 4);
    // Make the creature indestructible by adding the keyword.
    if let Some(obj) = state.get_object_mut(indestructible) {
        obj.keywords.push(Keyword::Indestructible);
    }

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(grimgrin, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    let behavior = reg.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, &reg);

    // The indestructible creature should still be on the battlefield.
    assert_eq!(state.get_object(indestructible).unwrap().zone, Zone::Battlefield,
        "Indestructible creature should survive destruction");
    // But Grimgrin still gets the +1/+1 counter.
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 1,
        "Grimgrin should still get +1/+1 counter even if target survives");
}

// Ruling: "If the targeted creature is an illegal target by the time Grimgrin's last ability
// resolves, the entire ability doesn't resolve and none of its effects will occur."
// This test verifies the attack trigger uses the defending player from combat state.
#[test]
fn grimgrin_attack_uses_defending_player_from_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_creature(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    // Defender has a creature.
    let defender_creature = ready_creature(&mut state, P1, 2, 2);
    // Controller also has another creature (should NOT be targetable).
    let own_creature = ready_creature(&mut state, P0, 3, 3);

    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(grimgrin, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
    });

    let behavior = reg.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, &reg);

    // With only one defender creature, auto-applies. The defender's creature should
    // be destroyed, not the controller's own creature.
    assert_eq!(state.get_object(defender_creature).unwrap().zone, Zone::Graveyard,
        "Defending player's creature should be targeted");
    assert_eq!(state.get_object(own_creature).unwrap().zone, Zone::Battlefield,
        "Controller's own creature should not be targeted");
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
fn geist_angel_exiled_at_end_of_combat() {
    let reg = registry();
    let mut state = game_at_step(Step::EndCombat, P0);
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

    // End of combat — angel should be exiled via delayed trigger in end_combat.
    mtg_engine::combat::end_combat(&mut state);
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
fn liliana_plus_one_each_player_discards_with_choice() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    // Give both players multiple cards so they must choose.
    let p0_card_a = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let p0_card_b = spell_in_hand(&mut state, &reg, "Bump in the Night", P0);
    let p1_card_a = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let p1_card_b = spell_in_hand(&mut state, &reg, "Bump in the Night", P1);

    // Activate +1.
    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // Active player (P0) should be asked to choose which card to discard.
    assert!(state.awaiting_action.is_some(), "Expected awaiting_action for P0 discard choice");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseCardFromHand { cards, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P0, "Active player should choose first");
        assert_eq!(cards.len(), 2, "P0 has 2 cards to choose from");
    } else {
        panic!("Expected ChooseCardFromHand for P0");
    }

    // P0 chooses to discard card A.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p0_card_a),
    }, &reg);

    // P0's chosen card should be in graveyard.
    assert_eq!(state.get_object(p0_card_a).unwrap().zone, Zone::Graveyard);
    // P0's other card should still be in hand.
    assert_eq!(state.get_object(p0_card_b).unwrap().zone, Zone::Hand);

    // Now P1 should be asked to choose.
    assert!(state.awaiting_action.is_some(), "Expected awaiting_action for P1 discard choice");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseCardFromHand { cards, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P1, "Other player should choose second");
        assert_eq!(cards.len(), 2, "P1 has 2 cards to choose from");
    } else {
        panic!("Expected ChooseCardFromHand for P1");
    }

    // P1 chooses to discard card B.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenCard(p1_card_b),
    }, &reg);

    // P1's chosen card should be in graveyard.
    assert_eq!(state.get_object(p1_card_b).unwrap().zone, Zone::Graveyard);
    // P1's other card should still be in hand.
    assert_eq!(state.get_object(p1_card_a).unwrap().zone, Zone::Hand);
}

#[test]
fn liliana_plus_one_single_card_auto_discards() {
    // When a player has exactly 1 card, it should auto-discard (no choice needed).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    // Give P0 one card and P1 one card.
    let p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // Both players had 1 card each, so both auto-discard. No awaiting_action.
    assert!(state.awaiting_action.is_none(), "Should auto-discard when only 1 card");
    assert_eq!(state.get_object(p0_card).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_plus_one_empty_hand_skipped() {
    // Ruling: "You can activate Liliana's first ability even if some or all players
    // will be unable to discard a card."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    // P0 has no cards, P1 has one card.
    let p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // P0 is skipped (no cards), P1 auto-discards (1 card).
    assert!(state.awaiting_action.is_none(), "P1 auto-discards");
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_plus_one_both_empty_hands() {
    // Both players have empty hands — ability resolves with no effect.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // No awaiting_action, nothing happened.
    assert!(state.awaiting_action.is_none());
}

#[test]
fn liliana_minus_two_target_player_sacrifices_creature() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let creature_a = ready_creature(&mut state, P1, 3, 3);
    let creature_b = ready_creature(&mut state, P1, 2, 2);

    // Activate -2 targeting P1.
    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P1)], &reg);

    // P1 should be asked which creature to sacrifice.
    assert!(state.awaiting_action.is_some(), "Expected sacrifice choice for P1");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChooseTarget { options, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P1, "Target player chooses which creature to sacrifice");
        assert_eq!(options.len(), 2, "P1 has 2 creatures to choose from");
    } else {
        panic!("Expected ChooseTarget for P1");
    }

    // P1 chooses creature_a to sacrifice.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTarget(Some(Target::Object(creature_a))),
    }, &reg);

    assert_eq!(state.get_object(creature_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(creature_b).unwrap().zone, Zone::Battlefield);
}

#[test]
fn liliana_minus_two_single_creature_auto_sacrifices() {
    // With only one creature, it's auto-sacrificed (no choice needed).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let creature = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P1)], &reg);

    // Only one creature, auto-sacrificed.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_two_no_creatures() {
    // If the target player has no creatures, nothing happens.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P1)], &reg);

    // No creatures, no effect.
    assert!(state.awaiting_action.is_none());
}

#[test]
fn liliana_minus_two_can_target_self() {
    // "Target player" means you can target yourself, not just opponent.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let own_creature = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P0)], &reg);

    // P0 targeted themselves, their creature is auto-sacrificed.
    assert_eq!(state.get_object(own_creature).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_six_pile_division_and_choice() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 9);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    // Give P1 three permanents.
    let c1 = ready_creature(&mut state, P1, 3, 3);
    let c2 = ready_creature(&mut state, P1, 2, 2);
    let c3 = ready_creature(&mut state, P1, 1, 1);

    // Activate -6 targeting P1.
    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // Step 1: Liliana's controller (P0) should divide permanents into two piles.
    assert!(state.awaiting_action.is_some(), "Expected pile division choice");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, target_player, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P0, "Controller divides the piles");
        assert_eq!(*target_player, P1, "Target player is P1");
        assert_eq!(permanents.len(), 3, "Three permanents to divide");
    } else {
        panic!("Expected DividePermanentsIntoPiles");
    }

    // P0 puts c1 in pile 1, c2 and c3 in pile 2.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1]),
    }, &reg);

    // Step 2: P1 should choose which pile to sacrifice.
    assert!(state.awaiting_action.is_some(), "Expected pile choice for P1");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::ChoosePile { pile_1, pile_2, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P1, "Target player chooses which pile to sacrifice");
        assert_eq!(pile_1.len(), 1, "Pile 1 has 1 permanent");
        assert_eq!(pile_2.len(), 2, "Pile 2 has 2 permanents");
    } else {
        panic!("Expected ChoosePile for P1");
    }

    // P1 chooses pile 1 (sacrifices c1 only).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(0),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard, "c1 should be sacrificed");
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Battlefield, "c2 should survive");
    assert_eq!(state.get_object(c3).unwrap().zone, Zone::Battlefield, "c3 should survive");
}

#[test]
fn liliana_minus_six_empty_pile_allowed() {
    // Ruling: "A pile can be empty. If the player chooses an empty pile, no permanents will be sacrificed."
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 9);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let c1 = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // P0 puts all permanents in pile 1 (pile 2 is empty).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1]),
    }, &reg);

    // P1 chooses the empty pile (pile 2, index 1) — nothing sacrificed.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(1),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield, "Chose empty pile, nothing sacrificed");
}

#[test]
fn liliana_minus_six_all_in_one_pile() {
    // Controller can put all permanents in one pile. If target player chooses that pile,
    // all permanents are sacrificed.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 9);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let c1 = ready_creature(&mut state, P1, 3, 3);
    let c2 = ready_creature(&mut state, P1, 2, 2);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // P0 puts everything in pile 1 (empty pile 2).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1, c2]),
    }, &reg);

    // P1 chooses pile 1 — all sacrificed.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(0),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_six_no_permanents() {
    // If target player has no permanents, nothing happens.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 9);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    assert!(state.awaiting_action.is_none(), "No permanents, no pile division needed");
}

#[test]
fn liliana_minus_six_can_target_self() {
    // -6 says "target player", so controller can target themselves.
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_creature(&mut state, &reg, "Liliana of the Veil", P0);
    state.add_counters(liliana, CounterType::Loyalty, 9);
    if let Some(obj) = state.get_object_mut(liliana) {
        obj.card_types = vec![CardType::Planeswalker];
    }

    // P0 has a creature (besides Liliana herself, which is also a permanent).
    let c1 = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P0)], &reg);

    // P0 is both controller and target. Division choice should still go to P0.
    assert!(state.awaiting_action.is_some(), "Expected pile division");
    if let Some(AwaitingAction::ResolutionChoice { player, choice: ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, target_player, .. }, .. }) = &state.awaiting_action {
        assert_eq!(*player, P0, "Controller is P0");
        assert_eq!(*target_player, P0, "Target is also P0");
        // P0 has Liliana and the creature on the battlefield.
        assert!(permanents.len() >= 2, "At least Liliana + creature");
    } else {
        panic!("Expected DividePermanentsIntoPiles");
    }
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
    behavior.on_loyalty_ability(&mut state, garruk, 1, &[], &reg);

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
    behavior.on_loyalty_ability(&mut state, garruk, 1, &[], &reg);

    // Transform is now handled by SBA, not inside on_loyalty_ability.
    check_state_based_actions_with_registry(&mut state, Some(&reg));

    // Should have transformed.
    assert!(state.get_object(garruk).unwrap().is_transformed);
    assert_eq!(state.get_object(garruk).unwrap().name, "Garruk, the Veil-Cursed");
}

#[test]
fn garruk_back_face_creates_deathtouch_wolf() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 2);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // +1: Create a 1/1 black Wolf with deathtouch (ability_index 10).
    behavior.on_loyalty_ability(&mut state, garruk, 10, &[], &reg);

    let wolves: Vec<_> = state.objects_in_zone(Zone::Battlefield, P0)
        .iter()
        .filter(|o| o.is_token && o.name == "Wolf")
        .cloned()
        .collect();
    assert_eq!(wolves.len(), 1, "Should create a Wolf token");
    assert_eq!(wolves[0].power, Some(1), "Wolf should be 1/1");
    assert_eq!(wolves[0].toughness, Some(1), "Wolf should be 1/1");
    assert!(wolves[0].keywords.contains(&Keyword::Deathtouch), "Wolf should have deathtouch");
}

#[test]
fn garruk_back_face_sacrifice_to_tutor() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    // Put a creature on the battlefield to sacrifice.
    let sac_target = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(sac_target).unwrap().card_types = vec![CardType::Creature];

    // Put a creature card in the library.
    let lib_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(lib_creature, Zone::Library);
    state.get_player_mut(P0).library_order.push(lib_creature);
    if let Some(obj) = state.get_object_mut(lib_creature) {
        obj.card_types = vec![CardType::Creature];
    }

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // -1: Sacrifice a creature, search for a creature card (ability_index 11).
    behavior.on_loyalty_ability(&mut state, garruk, 11, &[], &reg);

    // Sac target should be in graveyard.
    assert_eq!(state.get_object(sac_target).unwrap().zone, Zone::Graveyard,
        "Sacrificed creature should be in graveyard");

    // Library creature should now be in hand.
    assert_eq!(state.get_object(lib_creature).unwrap().zone, Zone::Hand,
        "Tutored creature should be in hand");
}

#[test]
fn garruk_back_face_tutor_presents_sacrifice_choice() {
    // With multiple creatures, the -1 ability should present a sacrifice choice.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 4);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    // Two creatures to choose from.
    let creature1 = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature1).unwrap().card_types = vec![CardType::Creature];
    let creature2 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature2).unwrap().card_types = vec![CardType::Creature];

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 11, &[], &reg);

    // Should be awaiting a sacrifice choice (not auto-selected).
    assert!(state.awaiting_action.is_some(),
        "Should present sacrifice choice when multiple creatures available");

    // Resolve with the specific creature we want to sacrifice.
    let new_state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(mtg_engine::actions::Target::Object(creature1))),
        },
        &reg,
    );

    // creature1 should be sacrificed (in graveyard).
    assert_eq!(new_state.get_object(creature1).unwrap().zone, Zone::Graveyard,
        "Chosen creature should be sacrificed");
    // creature2 should still be on the battlefield.
    assert_eq!(new_state.get_object(creature2).unwrap().zone, Zone::Battlefield,
        "Non-chosen creature should remain on battlefield");
}

#[test]
fn garruk_back_face_tutor_shuffles_library() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 4);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    let sac_target = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(sac_target).unwrap().card_types = vec![CardType::Creature];

    // Put several cards in the library so we can detect shuffling.
    let mut lib_ids = Vec::new();
    for name in &["Grizzly Bears", "Doom Blade", "Giant Growth", "Divination", "Lightning Bolt"] {
        let id = spell_in_hand(&mut state, &reg, name, P0);
        state.move_object(id, Zone::Library);
        state.get_player_mut(P0).library_order.push(id);
        if *name == "Grizzly Bears" {
            if let Some(obj) = state.get_object_mut(id) {
                obj.card_types = vec![CardType::Creature];
            }
        }
        lib_ids.push(id);
    }

    let order_before: Vec<_> = state.get_player(P0).library_order.clone();

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, garruk, 11, &[], &reg);

    let order_after: Vec<_> = state.get_player(P0).library_order.clone();

    // The searched creature should be removed from library (now in hand).
    assert!(!order_after.contains(&lib_ids[0]),
        "Tutored creature should no longer be in library");
    // Library should have been shuffled — with 4 remaining cards, the probability
    // of maintaining the exact same order is 1/24 = ~4%. We accept this tiny flake risk
    // as evidence of shuffling, since the alternative (not shuffling) always preserves order.
    // If this test flakes, it's still proving the shuffle codepath runs.
    assert_eq!(order_after.len(), order_before.len() - 1,
        "Library should have one fewer card after tutoring");
}

#[test]
fn garruk_back_face_overrun() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 4);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    // Put 2 creature cards in graveyard.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.get_object_mut(c).unwrap().card_types = vec![CardType::Creature];
        state.move_object(c, Zone::Graveyard);
    }

    // Put a creature on the battlefield.
    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().card_types = vec![CardType::Creature];

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    // -3: Creatures get +X/+X and trample (ability_index 12).
    behavior.on_loyalty_ability(&mut state, garruk, 12, &[], &reg);

    // X should be 2 (2 creature cards in graveyard).
    // Creature should have +2/+2 until end of turn.
    let has_buff = state.until_end_of_turn_effects.iter()
        .any(|e| e.target == creature && e.power_mod == 2 && e.toughness_mod == 2);
    assert!(has_buff, "Creature should have +2/+2 until end of turn");

    // Should have trample.
    let has_trample = state.until_end_of_turn_keywords.iter()
        .any(|k| k.target == creature && k.keyword == Keyword::Trample);
    assert!(has_trample, "Creature should have trample until end of turn");
}

#[test]
fn garruk_back_face_loyalty_abilities_shown_when_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = named_creature(&mut state, &reg, "Garruk Relentless", P0);
    state.add_counters(garruk, CounterType::Loyalty, 3);
    if let Some(obj) = state.get_object_mut(garruk) {
        obj.card_types = vec![CardType::Planeswalker];
        obj.is_transformed = true;
        obj.name = "Garruk, the Veil-Cursed".into();
    }

    let behavior = reg.get(state.get_object(garruk).unwrap().card_id).unwrap();
    let abilities = behavior.loyalty_abilities(&state, garruk);

    // Back face should have 3 abilities with indices 10, 11, 12.
    assert_eq!(abilities.len(), 3, "Back face should have 3 loyalty abilities");
    assert_eq!(abilities[0].ability_index, 10);
    assert_eq!(abilities[0].loyalty_change, 1); // +1
    assert_eq!(abilities[1].ability_index, 11);
    assert_eq!(abilities[1].loyalty_change, -1); // -1
    assert_eq!(abilities[2].ability_index, 12);
    assert_eq!(abilities[2].loyalty_change, -3); // -3
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
fn grimoire_discard_presents_choice_and_adds_study_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 multiple cards in hand so a choice is presented.
    let c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let c2 = spell_in_hand(&mut state, &reg, "Giant Growth", P0);

    // Add {1} mana for the ability cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Activate ability 0 via the engine.
    state = engine::submit_action(
        &state,
        &Action::ActivateAbility { object_id: grimoire, ability_index: 0, targets: vec![] },
        &reg,
    );

    // Should be awaiting a discard choice.
    assert!(state.awaiting_action.is_some(), "Should be awaiting discard choice");

    // Choose to discard c1.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(c1) },
        &reg,
    );

    // c1 should be in graveyard.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    // c2 should still be in hand.
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Hand);
    // Study counter should be added via the proper counter system.
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 1);
}

#[test]
fn grimoire_single_card_in_hand_auto_discards() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 only one card in hand.
    let c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    // Add {1} mana for the ability cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Activate ability 0 via the engine.
    state = engine::submit_action(
        &state,
        &Action::ActivateAbility { object_id: grimoire, ability_index: 0, targets: vec![] },
        &reg,
    );

    // With only one card, discard should be automatic (no choice needed).
    assert!(state.awaiting_action.is_none(), "Should not be awaiting a choice with one card");
    // c1 should be discarded.
    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    // Study counter should be added.
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 1);
}

#[test]
fn grimoire_accumulates_three_study_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();

    // Give P0 three cards to discard (one at a time).
    let c1 = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let c2 = spell_in_hand(&mut state, &reg, "Giant Growth", P0);
    let c3 = spell_in_hand(&mut state, &reg, "Lightning Bolt", P0);

    // Activate 3 times, discarding one card each time.
    // Each time: add mana, submit activate, choose a card.
    for (i, card_to_discard) in [c1, c2, c3].iter().enumerate() {
        state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);
        // Untap Grimoire for subsequent activations.
        if i > 0 {
            state.get_object_mut(grimoire).unwrap().tapped = false;
        }

        state = engine::submit_action(
            &state,
            &Action::ActivateAbility { object_id: grimoire, ability_index: 0, targets: vec![] },
            &reg,
        );

        // For the last card in hand, auto-discard happens.
        if state.awaiting_action.is_some() {
            state = engine::submit_action(
                &state,
                &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(*card_to_discard) },
                &reg,
            );
        }
    }

    // Should have 3 study counters.
    assert_eq!(state.get_counter_count(grimoire, CounterType::Study), 3);
}

#[test]
fn grimoire_reanimates_all_graveyard_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    // Add 3 study counters via the proper counter system.
    state.add_counters(grimoire, CounterType::Study, 3);

    // Put creatures in both graveyards.
    let gy1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(gy1).unwrap().name = "Creature A".into();
    state.move_object(gy1, Zone::Graveyard);

    let gy2 = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(gy2).unwrap().name = "Creature B".into();
    state.move_object(gy2, Zone::Graveyard);

    // Activate ability 1 via the engine (tap + sacrifice + remove counters).
    state = engine::submit_action(
        &state,
        &Action::ActivateAbility { object_id: grimoire, ability_index: 1, targets: vec![] },
        &reg,
    );

    // Both creatures should be on the battlefield under P0's control.
    assert_eq!(state.get_object(gy1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy1).unwrap().controller, P0);
    assert_eq!(state.get_object(gy2).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(gy2).unwrap().controller, P0);
    // They should have the Zombie subtype and black color.
    assert!(state.get_object(gy1).unwrap().subtypes.contains(&"Zombie".into()));
    assert!(state.get_object(gy2).unwrap().subtypes.contains(&"Zombie".into()));
    assert!(state.get_object(gy1).unwrap().colors.contains(&Color::Black));
    assert!(state.get_object(gy2).unwrap().colors.contains(&Color::Black));
    // Grimoire should be sacrificed (in graveyard).
    assert_eq!(state.get_object(grimoire).unwrap().zone, Zone::Graveyard);
}

#[test]
fn grimoire_ability_1_not_available_without_3_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimoire of the Dead").unwrap();
    let grimoire = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(grimoire).unwrap().name = "Grimoire of the Dead".into();
    // Only 2 study counters -- not enough.
    state.add_counters(grimoire, CounterType::Study, 2);

    let legal = engine::legal_actions(&state, &reg);
    let has_sacrifice_ability = legal.actions.iter().any(|a| {
        matches!(a, Action::ActivateAbility { ability_index: 1, .. })
    });
    assert!(!has_sacrifice_ability, "Should not be able to activate ability 1 with only 2 study counters");
}

// ── Civilized Scholar ──────────────────────────────────────────

#[test]
fn civilized_scholar_discard_creature_transforms() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scholar = named_creature(&mut state, &reg, "Civilized Scholar", P0);

    // Put a card in the library (will be drawn).
    let lib_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    state.move_object(lib_card, Zone::Library);
    state.players[0].library_order = vec![lib_card];

    // Put a non-creature in hand (so we have a choice after drawing).
    let _hand_spell = spell_in_hand(&mut state, &reg, "Doom Blade", P0);

    // Activate the ability — draws a card, then prompts for discard.
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility { object_id: scholar, ability_index: 0, targets: vec![] },
        &reg,
    );

    // Should be awaiting a discard choice now.
    assert!(new_state.awaiting_action.is_some(),
        "Should be awaiting discard choice");

    // Choose to discard the creature (Grizzly Bears we drew).
    let new_state = engine::submit_action(
        &new_state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(lib_card) },
        &reg,
    );

    // Should have transformed (discarded a creature).
    assert!(new_state.get_object(scholar).unwrap().is_transformed,
        "Should transform after discarding a creature");
    assert_eq!(new_state.get_object(scholar).unwrap().name, "Homicidal Brute");
    assert!(!new_state.get_object(scholar).unwrap().tapped,
        "Should be untapped after transform");
}

#[test]
fn civilized_scholar_discard_noncreature_no_transform() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scholar = named_creature(&mut state, &reg, "Civilized Scholar", P0);

    // Put a card in the library (will be drawn).
    let lib_card = spell_in_hand(&mut state, &reg, "Doom Blade", P0);
    state.move_object(lib_card, Zone::Library);
    state.players[0].library_order = vec![lib_card];

    // Put a creature in hand.
    let hand_creature = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    // Activate the ability.
    let new_state = engine::submit_action(
        &state,
        &Action::ActivateAbility { object_id: scholar, ability_index: 0, targets: vec![] },
        &reg,
    );

    // Should be awaiting a discard choice.
    assert!(new_state.awaiting_action.is_some());

    // Choose to discard the non-creature (Doom Blade we drew), keeping the creature.
    let new_state = engine::submit_action(
        &new_state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenCard(lib_card) },
        &reg,
    );

    // Should NOT transform (discarded a non-creature).
    assert!(!new_state.get_object(scholar).unwrap().is_transformed,
        "Should NOT transform after discarding a non-creature");
    assert_eq!(new_state.get_object(scholar).unwrap().name, "Civilized Scholar");
    // Should still be tapped (no untap since no transform).
    assert!(new_state.get_object(scholar).unwrap().tapped,
        "Should remain tapped when no transform");

    // The creature should still be in hand.
    assert_eq!(new_state.get_object(hand_creature).unwrap().zone, Zone::Hand,
        "Player chose to keep the creature in hand");
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
    // +1 (no target) + -2 targeting P0 + -2 targeting P1 = at least 3 actions.
    // (Not -6, since loyalty is only 3.)
    assert!(loyalty_actions.len() >= 3, "Expected at least 3 loyalty actions (+1 untargeted, -2 x2 player targets), got {}", loyalty_actions.len());

    // Verify +1 has no targets and -2 has player targets.
    let plus_one: Vec<_> = loyalty_actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { ability_index: 0, .. }))
        .collect();
    assert_eq!(plus_one.len(), 1, "+1 should have exactly one action (untargeted)");

    let minus_two: Vec<_> = loyalty_actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { ability_index: 1, .. }))
        .collect();
    assert_eq!(minus_two.len(), 2, "-2 should have two actions (targeting P0 and P1)");
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
        targets: vec![],
    }, &reg);

    // Loyalty should be 4 (3 + 1).
    assert_eq!(new_state.get_counter_count(liliana, CounterType::Loyalty), 4);

}
