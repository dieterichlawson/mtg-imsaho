//! Tests for Tier 15 medium-complexity Innistrad cards.
//! Tests for Tier 15 hard/complex Innistrad cards.


mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::CardId;
use mtg_engine::engine;
use mtg_engine::actions::Action;
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
        &Action::CastSpell { object_id: spell, targets: vec![], sacrifice: None },
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
        &Action::CastSpell { object_id: spell, targets: vec![], sacrifice: None },
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

// ── Screeching Bat ──────────────────────────────────────────

#[test]
fn screeching_bat_transforms_at_upkeep_with_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let bat = named_creature(&mut state, &reg, "Screeching Bat", P0);
    assert!(!state.get_object(bat).unwrap().is_transformed);

    // Add mana for the upkeep transform cost: {2}{B}{B}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Black, 2);

    let behavior = reg.get(state.get_object(bat).unwrap().card_id).unwrap();
    behavior.on_upkeep(&mut state, bat, &reg);

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

    // End of combat — angel should be exiled.
    behavior.on_end_combat(&mut state, geist, &reg);
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
