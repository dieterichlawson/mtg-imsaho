//! Creatures with several interacting abilities — transform, a trigger and an
//! activated ability at once. The largest of the per-card files.
//!
//! Cards covered (20), so this is greppable by name as well as by rule:
//!
//! - Back from the Brink
//! - Bitterheart Witch
//! - Cellar Door
//! - Creeping Renaissance
//! - Creepy Doll
//! - Curse of Stalked Prey
//! - Dearly Departed
//! - Evil Twin
//! - Galvanic Juggernaut
//! - Gutter Grime
//! - Heretic's Punishment
//! - Kessig Cagebreakers
//! - Manor Gargoyle
//! - Mentor of the Meek
//! - Mirror-Mad Phantasm
//! - Moldgraf Monstrosity
//! - Skaab Ruinator
//! - Tree of Redemption
//! - Unbreathing Horde
//! - Undead Alchemist

mod common;

use common::*;
use mtg_engine::cards::AttackInfo;
use mtg_engine::ids::CardId;
use mtg_engine::engine;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;

use mtg_engine::types::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::events::{DamageTarget, GameEvent};
use mtg_engine::state::StackEntry;
// ── Curse of Stalked Prey ────────────────────────────────────────

#[test]
fn curse_of_stalked_prey_gives_counter_on_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Place the curse on the battlefield attached to P1.
    let curse = attach_curse_to_player(&mut state, &reg, "Curse of Stalked Prey", P0, P1);

    // Place an attacking creature.
    let attacker = ready_creature(&mut state, P0, 2, 2);

    // Simulate combat damage to P1.
    let behavior = reg.get(state.get_object(curse).unwrap().card_id).unwrap();
    behavior.on_any_combat_damage_to_player(&mut state, curse, attacker, P1, 2, &reg);

    // The attacker should have a +1/+1 counter.
    assert_eq!(counters_of(&state, attacker, CounterType::PlusOnePlusOne), 1,
        "Attacker should get a +1/+1 counter");
}

// ── Dearly Departed ──────────────────────────────────────────────

#[test]
fn dearly_departed_gives_counter_to_entering_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Dearly Departed in the graveyard.
    let _dd = named_card_in_graveyard(&mut state, &reg, "Dearly Departed", P0);

    // Create a Human in hand, then move to battlefield so the
    // entering-with-counters replacement effect fires.
    let champ_id = reg.get_id_by_name("Champion of the Parish").unwrap();
    let human = state.create_object(champ_id, P0, Zone::Hand, Some(1), Some(1));
    state.get_object_mut(human).unwrap().name = "Champion of the Parish".into();
    state.move_object(human, Zone::Battlefield, &reg);

    assert_eq!(counters_of(&state, human, CounterType::PlusOnePlusOne), 1,
        "Human should enter with a +1/+1 counter from Dearly Departed");
}

// ── Mentor of the Meek ───────────────────────────────────────────

#[test]
fn mentor_of_the_meek_draws_when_small_creature_enters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mentor = named_permanent(&mut state, &reg, "Mentor of the Meek", P0);

    // Give P0 mana to pay {1}.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Put a card in library to draw.
    let lib_card = state.create_object(CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    // A 1/1 creature enters under our control.
    let small_creature = ready_creature(&mut state, P0, 1, 1);

    let behavior = reg.get(state.get_object(mentor).unwrap().card_id).unwrap();
    behavior.on_any_creature_enters(&mut state, mentor, small_creature, P0, &reg);

    // Oracle: "you may pay {1}" — should present a choice.
    assert!(state.awaiting_action.is_some(), "Should present 'you may pay' choice");

    // Player chooses yes (pay {1} and draw).
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );

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

    let cage = named_permanent(&mut state, &reg, "Kessig Cagebreakers", P0);

    // Set up combat: Kessig Cagebreakers is attacking P1.
    attacks_unblocked(&mut state, cage, P1);

    // Put 3 creatures in graveyard.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 2, 2);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let behavior = reg.get(state.get_object(cage).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, cage, AttackInfo::new(cage, P1), &[], &reg);

    // Should have 3 Wolf tokens on the battlefield.
    assert_eq!(count_tokens_named(&state, "Wolf"), 3, "Should have created 3 Wolf tokens");

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

    let jug = named_permanent(&mut state, &reg, "Galvanic Juggernaut", P0);
    // Tap it.
    state.get_object_mut(jug).unwrap().tapped = true;

    // A creature dies.
    let dead = ready_creature(&mut state, P1, 1, 1);
    let behavior = reg.get(state.get_object(jug).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, jug, dead, P1, &[], 1, false, &[], &reg);

    assert!(!state.get_object(jug).unwrap().tapped, "Galvanic Juggernaut should untap when a creature dies");
}

// ── Bitterheart Witch ────────────────────────────────────────────

#[test]
fn bitterheart_witch_finds_curse_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // CR 603.3d: "attached to target player" is targeted, so the player was
    // chosen when the trigger went on the stack — before the search.
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses yes to search...
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    // ...then picks the Curse. CR 701.19b: searching never forces a find, so
    // the choice is offered even with a single Curse in the library.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(curse_obj))) },
        &reg,
    );

    // The curse should be on the battlefield attached to opponent.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Battlefield, "Curse should be on battlefield");
    assert_eq!(curse.attached_to_player, Some(P1), "Curse should be attached to opponent");
}

#[test]
fn bitterheart_witch_can_attach_curse_to_self() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // Targeting yourself is legal — the card says "target player", not
    // "target opponent".
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P0)], &reg);
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice {
            choice: ResolvedChoice::ChosenTarget(Some(Target::Object(curse_obj))) },
        &reg,
    );

    // The curse should be on the battlefield attached to self.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Battlefield, "Curse should be on battlefield");
    assert_eq!(curse.attached_to_player, Some(P0), "Curse should be attached to self");
}

#[test]
fn bitterheart_witch_decline_search() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);

    // Put a curse in the library.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse_obj).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse_obj);

    // Trigger death.
    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player declines to search.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) },
        &reg,
    );

    // Curse should still be in the library.
    let curse = state.get_object(curse_obj).unwrap();
    assert_eq!(curse.zone, Zone::Library, "Curse should remain in library when search declined");
}

// ── Gutter Grime ─────────────────────────────────────────────────

#[test]
fn gutter_grime_creates_ooze_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    // A nontoken creature we control dies.
    let dead = ready_creature(&mut state, P0, 2, 2);

    let behavior = reg.get(state.get_object(grime).unwrap().card_id).unwrap();
    behavior.on_any_creature_dies(&mut state, grime, dead, P0, &[], 2, false, &[], &reg);

    // Should have a slime counter on Gutter Grime.
    assert_eq!(counters_of(&state, grime, CounterType::Slime), 1,
        "Gutter Grime should have 1 slime counter");

    // Should have created an Ooze token.
    assert_eq!(count_tokens_named(&state, "Ooze"), 1, "Should have created 1 Ooze token");
}

// ── Heretic's Punishment ─────────────────────────────────────────

#[test]
fn heretics_punishment_mills_then_deals_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);

    // Put cards in library. Use a known card so mana value is deterministic.
    // Kalonian Tusker costs {G}{G} = MV 2.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    let initial_life = state.get_player(P1).life;
    activate_via_hooks(&mut state, &reg, hp, 0, &[mtg_engine::actions::Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);

    // Put cards in library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    let target_creature = ready_creature(&mut state, P1, 5, 5);

    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Object(target_creature)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    let obj = state.get_object(target_creature).unwrap();
    assert_eq!(obj.damage_marked, 2, "Creature should have 2 damage marked");
    assert!(obj.damaged_by.contains(&hp), "damaged_by should track the source");
}

#[test]
fn heretics_punishment_fizzles_when_target_illegal() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let hp = named_permanent(&mut state, &reg, "Heretic's Punishment", P0);

    // Put cards in library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..3 {
        let card = state.create_object(tusker_id, P0, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P0).library_order.insert(0, card);
    }

    // Create a creature target then move it off the battlefield (illegal target).
    let target_creature = ready_creature(&mut state, P1, 3, 3);
    state.move_object(target_creature, Zone::Graveyard, &reg);

    activate_via_hooks(&mut state, &reg, hp, 0, &[Target::Object(target_creature)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Entire ability should fizzle: no cards milled.
    let lib_count = state.get_player(P0).library_order.len();
    assert_eq!(lib_count, 3, "Library should be unchanged when ability fizzles");
}

// ── Undead Alchemist ─────────────────────────────────────────────

#[test]
fn undead_alchemist_mills_instead_of_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _alchemist = named_permanent(&mut state, &reg, "Undead Alchemist", P0);

    // Create a Zombie that dealt damage.
    let zombie = state.create_token_with_subtypes(
        "Zombie", P0, 2, 2,
        vec![Color::Black], vec![CardType::Creature], vec![],
        vec!["Zombie".into()],
        &reg,
    )[0];
    state.get_object_mut(zombie).unwrap().summoning_sick = false;

    // Put creature cards in P1's library so milling creates tokens.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    for _ in 0..2 {
        let card = state.create_object(tusker_id, P1, Zone::Library, Some(3), Some(3));
        state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
        state.get_player_mut(P1).library_order.insert(0, card);
    }

    let initial_life = state.get_player(P1).life;

    // The replacement effect intercepts damage before it's applied.
    let replaced = mtg_engine::replacement::apply(
        &mut state,
        mtg_engine::replacement::ReplaceableEvent::DealsDamage {
            source: zombie,
            target: mtg_engine::events::DamageTarget::Player(P1),
            amount: 2,
            combat: true,
        },
        &reg,
    )
    .is_none();
    assert!(replaced, "Undead Alchemist should replace Zombie combat damage");

    // Process triggers so the CreatureCardMilled events fire the exile+token ability.
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Life should be unchanged (damage was replaced, never applied).
    assert_eq!(state.get_player(P1).life, initial_life, "Life should be unchanged — damage was replaced");

    // The creature cards should have been exiled (not in graveyard).
    let p1_exile = state.objects.values()
        .filter(|o| o.zone == Zone::Exile && o.owner == P1)
        .count();
    assert_eq!(p1_exile, 2, "Milled creature cards should be exiled");

    // Should have created Zombie tokens.
    // 2 creature cards milled = 2 Zombie tokens + the original zombie we created.
    assert!(count_tokens_named_by(&state, "Zombie", P0) >= 2,
        "Should create Zombie tokens for each milled creature");
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
        state.move_object(c, Zone::Graveyard, &reg);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    // Cast the spell and put it on the stack.
    state = cast_onto_stack(&state, &reg, spell, vec![]);
    // Resolve: this triggers a ChooseCardType choice.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(state.awaiting_action.is_some(), "Should be awaiting card type choice");

    // Choose "Creature" (index 0).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()) },
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
        state.move_object(c, Zone::Graveyard, &reg);
    }
    for _ in 0..2 {
        let e = state.create_object(CardId(9999), P0, Zone::Battlefield, None, None);
        state.get_object_mut(e).unwrap().card_types = vec![CardType::Enchantment];
        state.move_object(e, Zone::Graveyard, &reg);
    }

    let spell = castable_spell(&mut state, &reg, "Creeping Renaissance", P0);
    state = cast_onto_stack(&state, &reg, spell, vec![]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Choose "Enchantment" (index 2).
    state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(2, "Option 2".into()) },
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
    state.move_object(c, Zone::Graveyard, &reg);

    // Put Creeping Renaissance itself in graveyard for flashback.
    let card_id = reg.get_id_by_name("Creeping Renaissance").unwrap();
    let spell = state.create_object(card_id, P0, Zone::Graveyard, None, None);
    state.get_object_mut(spell).unwrap().name = "Creeping Renaissance".into();

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
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()) },
        &reg,
    );

    // Creature in hand.
    let hand = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0 && o.card_types.contains(&CardType::Creature))
        .count();
    assert_eq!(hand, 1, "Creature should be in hand");

    // Creeping Renaissance should be exiled (flashback). A card is never
    // removed from the game outright, so "gone" is not an acceptable outcome.
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Exile,
        "Creeping Renaissance should be exiled after flashback");
}

// ── Cellar Door ──────────────────────────────────────────────────

#[test]
fn cellar_door_creates_zombie_when_milling_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let door = named_permanent(&mut state, &reg, "Cellar Door", P0);

    // Put a creature card on top of P1's library.
    let tusker_id = reg.get_id_by_name("Kalonian Tusker").unwrap();
    let card = state.create_object(tusker_id, P1, Zone::Library, Some(3), Some(3));
    state.get_object_mut(card).unwrap().name = "Kalonian Tusker".into();
    state.get_player_mut(P1).library_order.insert(0, card);

    activate_via_hooks(&mut state, &reg, door, 0, &[mtg_engine::actions::Target::Player(P1)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have created a Zombie token (since a creature was milled).
    assert_eq!(count_tokens_named(&state, "Zombie"), 1,
        "Should create a Zombie token when milling a creature");
}

// ── Skaab Ruinator ───────────────────────────────────────────────

#[test]
fn skaab_ruinator_exiles_creatures_from_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 3 creature cards in graveyard for the additional cost.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
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

/// "Skaab Ruinator is on the stack when you pay its costs. It can't be exiled
/// to pay for itself." Casting it from your graveyard with only two *other*
/// creature cards there is not a legal cast — the Ruinator does not count
/// towards the three it has to exile.
#[test]
fn skaab_ruinator_cannot_be_exiled_to_pay_for_itself() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Two other creature cards in the graveyard: three creature cards in the
    // zone, but only two the cost can reach.
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }
    assert!(!can_cast(&state, &reg, ruinator),
        "two other creature cards is not three — it cannot exile itself");

    // A third makes it castable.
    let c = ready_creature(&mut state, P0, 1, 1);
    state.move_object(c, Zone::Graveyard, &reg);
    assert!(can_cast(&state, &reg, ruinator),
        "three other creature cards pays the cost");
}

#[test]
fn skaab_ruinator_cast_from_graveyard() {
    // "You may cast this card from your graveyard" — uses normal mana cost, not flashback.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.priority_player = Some(P0);

    // Put Skaab Ruinator in graveyard.
    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);

    // Put 3 creature cards in graveyard for the additional cost.
    for _ in 0..3 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    // Give enough mana ({1}{U}{U}).
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    // Should be castable from graveyard.
    let can_cast = can_cast(&state, &reg, ruinator);
    assert!(can_cast, "Skaab Ruinator should be castable from graveyard");

    // Cast it — the engine sets up a ChooseExileFromGraveyard prompt
    // (exile 3 creatures) and leaves the spell in the graveyard until
    // the cost is paid. Use the test helper to pick the max-power
    // subset (all three 1/1s).
    let mut new_state = cast_onto_stack(&state, &reg, ruinator, vec![]);
    new_state = resolve_exile_choice_max_power(&new_state, &reg);

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

    let ruinator = named_card_in_graveyard(&mut state, &reg, "Skaab Ruinator", P0);

    // Only 2 creatures (need 3).
    for _ in 0..2 {
        let c = ready_creature(&mut state, P0, 1, 1);
        state.move_object(c, Zone::Graveyard, &reg);
    }

    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let can_cast = can_cast(&state, &reg, ruinator);
    assert!(!can_cast, "Should NOT be castable with only 2 creature cards in graveyard");
}

// ── Manor Gargoyle ───────────────────────────────────────────────

#[test]
fn manor_gargoyle_loses_defender_and_gains_flying() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gargoyle = named_permanent(&mut state, &reg, "Manor Gargoyle", P0);

    // Should start with Defender.
    assert!(state.has_keyword(gargoyle, Keyword::Defender, &reg),
        "Manor Gargoyle should start with Defender");

    // Activate ability.
    activate_via_hooks(&mut state, &reg, gargoyle, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Should have lost Defender. Asked through the accessor: `obj.keywords`
    // holds only runtime grants and is empty for every registry card, so
    // reading it directly would report "no Defender" whatever happened.
    assert!(!state.has_keyword(gargoyle, Keyword::Defender, &reg),
        "Manor Gargoyle should lose Defender after activation");

    // Should have gained Flying (as until-end-of-turn keyword).
    let has_flying = state.until_end_of_turn.iter()
        .any(|e| matches!(e, mtg_engine::state::TemporaryEffect::GrantKeyword { target, keyword } if *target == gargoyle && *keyword == Keyword::Flying));
    assert!(has_flying, "Manor Gargoyle should gain Flying until end of turn");
}

// ── Tree of Redemption ───────────────────────────────────────────

#[test]
fn tree_of_redemption_swaps_life_and_toughness() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    // P0 starts at 20 life, Tree base toughness is 13.

    activate_via_hooks(&mut state, &reg, tree, 0, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

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
            &reg,
        );
    }

    // Put 1 Zombie card in graveyard.
    let _gy_zombie = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);

    // Cast Unbreathing Horde — on_resolve counts graveyard BEFORE moving to battlefield.
    let horde = castable_spell(&mut state, &reg, "Unbreathing Horde", P0);
    let new_state = cast_and_resolve(&state, &reg, horde, vec![]);

    // Should have 3 counters (2 battlefield Zombies + 1 graveyard Zombie).
    assert_eq!(counters_of(&new_state, horde, CounterType::PlusOnePlusOne), 3,
        "Unbreathing Horde should enter with 3 +1/+1 counters");
}

// ── Back from the Brink ──────────────────────────────────────────

#[test]
fn back_from_the_brink_creates_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    // Put a creature in graveyard.
    let dead = named_card_in_graveyard(&mut state, &reg, "Kalonian Tusker", P0);


    // The ability_index encodes the creature's ObjectId.
    let ability_index = usize::try_from(dead.0).unwrap();
    activate_via_hooks(&mut state, &reg, enchant, ability_index, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // The creature should be exiled.
    assert_eq!(state.get_object(dead).unwrap().zone, Zone::Exile,
        "Original creature should be exiled");

    // A token copy should be on the battlefield.
    assert_eq!(count_tokens_named(&state, "Kalonian Tusker"), 1,
        "Should have created a token copy");
}

#[test]
fn back_from_the_brink_ability_per_creature_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    // Put two different creatures in the graveyard.
    let _tusker = named_card_in_graveyard(&mut state, &reg, "Kalonian Tusker", P0);
    let _piker = named_card_in_graveyard(&mut state, &reg, "Goblin Piker", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);

    // Should have one ability per creature in the graveyard.
    assert_eq!(abilities.len(), 2, "Should have one ability per creature in graveyard");

    // Each ability should reference a different creature and have its mana cost.
    let tusker_ability = abilities.iter().find(|a| a.description.contains("Kalonian Tusker"));
    let piker_ability = abilities.iter().find(|a| a.description.contains("Goblin Piker"));
    assert!(tusker_ability.is_some(), "Should have ability for Kalonian Tusker");
    assert!(piker_ability.is_some(), "Should have ability for Goblin Piker");

    // Kalonian Tusker costs {G}{G} — 2 colored symbols.
    let tusker_cost = &tusker_ability.unwrap().cost;
    assert_eq!(tusker_cost.symbols.len(), 2, "Kalonian Tusker costs {{G}}{{G}}");

    // Goblin Piker costs {1}{R} — 2 symbols.
    let piker_cost = &piker_ability.unwrap().cost;
    assert_eq!(piker_cost.symbols.len(), 2, "Goblin Piker costs {{1}}{{R}}");
}

#[test]
fn back_from_the_brink_no_abilities_without_creatures_in_graveyard() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);

    assert_eq!(abilities.len(), 0, "No abilities when graveyard has no creatures");
}

#[test]
fn back_from_the_brink_uses_creature_mana_cost() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let enchant = named_permanent(&mut state, &reg, "Back from the Brink", P0);

    // Savannah Lions costs {W}.
    let lions = named_card_in_graveyard(&mut state, &reg, "Savannah Lions", P0);

    let behavior = reg.get(state.get_object(enchant).unwrap().card_id).unwrap();
    let abilities = behavior.activated_abilities(&state, enchant, &reg);
    assert_eq!(abilities.len(), 1);

    let ability = &abilities[0];
    // Savannah Lions costs {W} — 1 white mana symbol.
    assert_eq!(ability.cost.symbols.len(), 1, "Savannah Lions costs {{W}}");
    assert!(ability.sorcery_speed_only, "Activate only as a sorcery");

    // Activate the ability — use the creature's ObjectId as the ability index.
    let ability_index = usize::try_from(lions.0).unwrap();
    activate_via_hooks(&mut state, &reg, enchant, ability_index, &[]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(lions).unwrap().zone, Zone::Exile,
        "Lions should be exiled");
    assert_eq!(count_tokens_named(&state, "Savannah Lions"), 1,
        "Should have created a token copy of Savannah Lions");
}

// ── Grimgrin, Corpse-Born ──────────────────────────────────────────

/// "Grimgrin enters tapped" is a replacement effect (CR 614.1c), so it is
/// tapped *as* it arrives — not moved onto the battlefield untapped and tapped
/// afterwards, which is what an `on_resolve` override used to do. `move_object`
/// emits `EnteredBattlefield` as part of the move, so under the old shape every
/// ETB watcher saw an untapped Grimgrin.
#[test]
fn grimgrin_enters_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Grimgrin, Corpse-Born").unwrap();
    let id = state.create_object(card_id, P0, Zone::Stack, Some(5), Some(5));
    state.get_object_mut(id).unwrap().name = "Grimgrin, Corpse-Born".into();

    assert!(plan_entering(&mut state, &reg, id, Some(Zone::Stack)).tapped,
        "the replacement effect taps it as part of the entry event");

    state.move_object(id, Zone::Battlefield, &reg);
    assert!(state.get_object(id).unwrap().tapped);
    assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
}

/// "Grimgrin enters tapped **and doesn't untap during your untap step**." The
/// second half is the whole reason the sacrifice ability exists — without it
/// Grimgrin would simply untap for free every turn — and it had no test.
///
/// Another tapped creature untaps in the same step, so this shows the untap
/// step really ran rather than being skipped.
#[test]
fn grimgrin_does_not_untap_during_his_controllers_untap_step() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let other = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(grimgrin).unwrap().tapped = true;
    state.get_object_mut(other).unwrap().tapped = true;

    // Round the table back to P0's untap step.
    advance_to_next_turn(&mut state, &reg);
    advance_to_next_turn(&mut state, &reg);
    assert_eq!(state.active_player, P0, "back to Grimgrin's controller's turn");

    assert!(!state.get_object(other).unwrap().tapped,
        "an ordinary creature untapped, so the untap step ran");
    assert!(state.get_object(grimgrin).unwrap().tapped,
        "Grimgrin does not untap during his controller's untap step");
}

#[test]
fn grimgrin_sacrifice_untaps_and_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    state.get_object_mut(grimgrin).unwrap().tapped = true;

    let zombie = ready_creature(&mut state, P0, 2, 2);

    // Activate through the engine, sacrificing the zombie (not Grimgrin itself).
    let new_state = activate_sacrificing(&state, &reg, grimgrin, 0, vec![], zombie);

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

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
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

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let defender_creature = ready_creature(&mut state, P1, 3, 3);

    // Set up combat state with Grimgrin attacking P1.
    attacks_unblocked(&mut state, grimgrin, P1);

    // Fire the AttackersDeclared event and run the trigger pipeline.
    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // With only one defender creature, the target is auto-chosen and the
    // trigger goes directly on the stack and resolves.
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

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let creature_a = ready_creature(&mut state, P1, 2, 2);
    let creature_b = ready_creature(&mut state, P1, 3, 3);

    // Set up combat state with Grimgrin attacking P1.
    attacks_unblocked(&mut state, grimgrin, P1);

    // Fire the AttackersDeclared event and collect triggers (which enters
    // the stack-time target-choice prompt because there are two valid defenders).
    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::collect_triggers(&mut state, &reg);

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
    // Resolve the trigger on the stack.
    triggers::process_triggers(&mut state, &reg);

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

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    // Defender has NO creatures.

    attacks_unblocked(&mut state, grimgrin, P1);

    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // No valid targets — trigger removed from stack (CR 603.3c), no +1/+1 counter.
    assert_eq!(state.get_counter_count(grimgrin, CounterType::PlusOnePlusOne), 0,
        "Grimgrin should NOT get a +1/+1 counter when defender has no creatures");
}

// Ruling: "If Grimgrin's last ability resolves, but the targeted creature isn't destroyed
// (perhaps because it regenerated or has indestructible), you'll still put a +1/+1 on Grimgrin."
#[test]
fn grimgrin_attack_indestructible_target_still_gets_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    let indestructible = ready_creature(&mut state, P1, 4, 4);
    // Make the creature indestructible by adding the keyword.
    if let Some(obj) = state.get_object_mut(indestructible) {
        obj.keywords.push(Keyword::Indestructible);
    }

    attacks_unblocked(&mut state, grimgrin, P1);

    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

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

    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P0);
    // Defender has a creature.
    let defender_creature = ready_creature(&mut state, P1, 2, 2);
    // Controller also has another creature (should NOT be targetable).
    let own_creature = ready_creature(&mut state, P0, 3, 3);

    attacks_unblocked(&mut state, grimgrin, P1);

    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(grimgrin, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // With only one defender creature (own_creature is filtered out by
    // is_valid_target), the target auto-resolves. The defender's creature
    // should be destroyed, not the controller's own creature.
    assert_eq!(state.get_object(defender_creature).unwrap().zone, Zone::Graveyard,
        "Defending player's creature should be targeted");
    assert_eq!(state.get_object(own_creature).unwrap().zone, Zone::Battlefield,
        "Controller's own creature should not be targeted");
}

// ── Evil Twin ──────────────────────────────────────────

#[test]
fn evil_twin_copies_creature_on_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let opponent_creature = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);

    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &reg);

    // ETB now presents an optional choice instead of auto-copying.
    assert!(state.awaiting_action.is_some(), "Should present a copy choice");

    // Resolve the choice by selecting the opponent's creature.
    let target = mtg_engine::actions::Target::Object(opponent_creature);
    let effect = mtg_engine::state::PendingEffect::CopyCreature { source_id: twin };
    state.awaiting_action = None;
    mtg_engine::engine::apply_pending_effect(&mut state, &target, &effect, &reg);

    // Evil Twin should have copied Grizzly Bears stats.
    assert_eq!(state.get_object(twin).unwrap().name, "Grizzly Bears");
    assert_eq!(state.get_object(twin).unwrap().power, Some(2));
    assert_eq!(state.get_object(twin).unwrap().toughness, Some(2));
    // Should still have the Evil Twin marker.
    assert!(state.get_object(twin).unwrap().copy_grantor.is_some(),
        "a permanent that entered as a copy records the card whose copy \
         effect made it, so the granted ability can be found (CR 706.2)");
}

/// A copy of a legendary creature is itself legendary (CR 707.2), so an Evil
/// Twin copying your own legend triggers the legend rule.
#[test]
fn evil_twin_copying_legendary_triggers_legend_rule() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0 controls a legendary creature with fixed P/T.
    let original = named_permanent(&mut state, &reg, "Geist of Saint Traft", P0);
    assert!(state.get_object(original).unwrap().is_legendary);

    // Evil Twin (also P0) enters and copies it.
    let twin = named_permanent(&mut state, &reg, "Evil Twin", P0);
    let behavior = reg.get(state.get_object(twin).unwrap().card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, twin, &[], &reg);
    state.awaiting_action = None;
    engine::apply_pending_effect(
        &mut state,
        &Target::Object(original),
        &mtg_engine::state::PendingEffect::CopyCreature { source_id: twin },
        &reg,
    );

    // The copy is legendary and shares the original's name.
    assert!(state.get_object(twin).unwrap().is_legendary,
        "a copy of a legendary creature is itself legendary (CR 707.2)");
    assert_eq!(state.get_object(twin).unwrap().name, "Geist of Saint Traft");

    // Legend rule: P0 now controls two same-named legendaries — SBA must
    // require choosing which to keep.
    check_state_based_actions(&mut state, &reg);
    assert!(state.awaiting_action.is_some(),
        "legend rule should force P0 to choose which legendary to keep");
}

// ── Moldgraf Monstrosity ──────────────────────────────────────────

#[test]
fn moldgraf_monstrosity_returns_creatures_on_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monstrosity = named_permanent(&mut state, &reg, "Moldgraf Monstrosity", P0);

    // Put two creatures in P0's graveyard.
    let gy1 = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(gy1).unwrap().name = "Creature 1".into();
    state.move_object(gy1, Zone::Graveyard, &reg);
    let gy2 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(gy2).unwrap().name = "Creature 2".into();
    state.move_object(gy2, Zone::Graveyard, &reg);

    // Die for real — the trigger resolves with the card already in the
    // graveyard, which is the only place "exile it" can apply.
    mtg_engine::destruction::try_destroy(&mut state, monstrosity, &reg);
    assert_eq!(state.get_object(monstrosity).unwrap().zone, Zone::Graveyard,
        "test precondition: the Monstrosity died");

    let behavior = reg.get(state.get_object(monstrosity).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, monstrosity, &[], &reg);

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

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
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

    // CR 101.4: the choice is recorded, but nothing has left a hand yet —
    // every player chooses first and the cards are discarded together.
    assert_eq!(state.get_object(p0_card_a).unwrap().zone, Zone::Hand,
        "P0's card must stay in hand until P1 has also chosen");
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

    // Now that the last player has chosen, both discards happen at once.
    assert_eq!(state.get_object(p0_card_a).unwrap().zone, Zone::Graveyard,
        "P0's chosen card is discarded when the last player has chosen");
    assert_eq!(state.get_object(p1_card_b).unwrap().zone, Zone::Graveyard);
    // Each player's other card should still be in hand.
    assert_eq!(state.get_object(p0_card_b).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(p1_card_a).unwrap().zone, Zone::Hand);
}

#[test]
fn liliana_plus_one_single_card_auto_discards() {
    // When a player has exactly 1 card, it should auto-discard (no choice needed).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
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

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);
    let p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 0, &[], &reg);

    // P0 is skipped (no cards), P1 auto-discards (1 card).
    assert!(state.awaiting_action.is_none(), "P1 auto-discards");
    assert_eq!(state.get_object(p1_card).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_two_target_player_sacrifices_creature() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
    use mtg_engine::actions::ResolvedChoice;

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

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

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

    let creature = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 1, &[Target::Player(P1)], &reg);

    // Only one creature, auto-sacrificed.
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Graveyard);
}

/// Each of Liliana's three abilities has a "nothing to work with" case, and
/// all three used to be their own test asserting only `awaiting_action.is_none()`.
/// A test whose one assertion is "no prompt appeared" passes with the ability's
/// implementation deleted, so each row here is run twice — once with nothing to
/// act on, once with something — and the second half is what makes the first
/// mean anything.
#[test]
fn each_liliana_ability_with_nothing_to_act_on_asks_nothing_and_does_nothing() {
    let reg = registry();
    // (ability index, targets, what the ability would need)
    let cases: [(usize, &[Target], &str); 3] = [
        (0, &[], "a card in someone's hand to discard"),
        (1, &[Target::Player(P1)], "a creature the target player controls"),
        (2, &[Target::Player(P1)], "a permanent the target player controls"),
    ];

    for (index, targets, needs) in cases {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
        set_loyalty(&mut state, liliana, 9);
        let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
        behavior.on_loyalty_ability(&mut state, liliana, index, targets, &reg);

        assert!(state.awaiting_action.is_none(),
            "ability {index} with no {needs} must not stop for a choice");

        // The control: give it something, and the same call does ask.
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
        set_loyalty(&mut state, liliana, 9);
        if index == 0 {
            // Two cards each, so neither player's discard is auto-picked.
            for p in [P0, P1] {
                spell_in_hand(&mut state, &reg, "Grizzly Bears", p);
                spell_in_hand(&mut state, &reg, "Lightning Bolt", p);
            }
        } else {
            ready_creature(&mut state, P1, 2, 2);
            ready_creature(&mut state, P1, 3, 3);
        }
        let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
        behavior.on_loyalty_ability(&mut state, liliana, index, targets, &reg);

        assert!(state.awaiting_action.is_some(),
            "control: ability {index} does ask once there is {needs}");
    }
}

#[test]
fn liliana_minus_two_can_target_self() {
    // "Target player" means you can target yourself, not just opponent.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 3);

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

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);
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
        choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()),
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

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);

    let c1 = ready_creature(&mut state, P1, 3, 3);

    let behavior = reg.get(state.get_object(liliana).unwrap().card_id).unwrap();
    behavior.on_loyalty_ability(&mut state, liliana, 2, &[Target::Player(P1)], &reg);

    // P0 puts all permanents in pile 1 (pile 2 is empty).
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenSubset(vec![c1]),
    }, &reg);

    // P1 chooses the empty pile (pile 2, index 1) — nothing sacrificed.
    state = engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenIndex(1, "Option 1".into()),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Battlefield, "Chose empty pile, nothing sacrificed");
}

#[test]
fn liliana_minus_six_all_in_one_pile() {
    // Controller can put all permanents in one pile. If target player chooses that pile,
    // all permanents are sacrificed.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);

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
        choice: ResolvedChoice::ChosenIndex(0, "Option 0".into()),
    }, &reg);

    assert_eq!(state.get_object(c1).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(c2).unwrap().zone, Zone::Graveyard);
}

#[test]
fn liliana_minus_six_can_target_self() {
    // -6 says "target player", so controller can target themselves.
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let liliana = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    set_loyalty(&mut state, liliana, 9);
    let _c1 = ready_creature(&mut state, P0, 2, 2);

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

// ── Essence of the Wild ──────────────────────────────────────────

#[test]
fn essence_overrides_entering_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Essence of the Wild on the battlefield via cast_and_resolve,
    // which calls on_resolve and sets entering_copy_source.
    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    state = cast_and_resolve(&state, &reg, essence, vec![]);

    // Verify Essence itself is on the battlefield.
    assert_eq!(state.get_object(essence).unwrap().zone, Zone::Battlefield);

    // Now cast a creature — it should enter as a 6/6 copy of Essence
    // via the replacement effect (before ETB triggers fire).
    let bear = castable_spell(&mut state, &reg, "Grizzly Bears", P0);
    state = cast_and_resolve(&state, &reg, bear, vec![]);

    // Bear should now be a 6/6 Essence copy (replacement effect, not trigger).
    assert_eq!(state.get_object(bear).unwrap().power, Some(6));
    assert_eq!(state.get_object(bear).unwrap().toughness, Some(6));
    assert_eq!(state.get_object(bear).unwrap().name, "Essence of the Wild");
    assert_eq!(state.get_object(bear).unwrap().subtypes, vec!["Avatar".to_string()]);
}

#[test]
fn essence_does_not_override_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put Essence on the battlefield for P0.
    let essence = castable_spell(&mut state, &reg, "Essence of the Wild", P0);
    state = cast_and_resolve(&state, &reg, essence, vec![]);

    // Opponent's creature enters via move_object — should NOT be affected
    // because Essence only applies to its controller's creatures.
    let opp_bear = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    state.move_object(opp_bear, Zone::Battlefield, &reg);

    // Opponent's creature should be unchanged.
    assert_eq!(state.get_object(opp_bear).unwrap().power, Some(2));
    assert_eq!(state.get_object(opp_bear).unwrap().toughness, Some(2));
    assert_eq!(state.get_object(opp_bear).unwrap().name, "Grizzly Bears");
}

// ── Mirror-Mad Phantasm ──────────────────────────────────────────

/// Mirror-Mad Phantasm: "{1}{U}: its owner shuffles it into their library, then
/// reveals cards until a card named Mirror-Mad Phantasm is revealed, puts that
/// card onto the battlefield and all other cards revealed this way into their
/// graveyard."
///
/// The shuffle makes the mill count random, so assert what is true of every
/// shuffle: the Phantasm comes back to the battlefield, everything revealed
/// above it is in the graveyard, everything below it is still in the library,
/// and no card ends up anywhere else.
#[test]
fn mirror_mad_phantasm_mills_to_find_itself() {
    let reg = registry();

    // Several runs, because a single shuffle could put the Phantasm on top and
    // mill nothing at all.
    let mut saw_a_mill = false;
    for _ in 0..20 {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let phantasm = named_permanent(&mut state, &reg, "Mirror-Mad Phantasm", P0);

        let library: Vec<_> = ["Grizzly Bears", "Lightning Bolt", "Doom Blade", "Divination"]
            .iter()
            .map(|n| {
                let c = spell_in_hand(&mut state, &reg, n, P0);
                state.move_object(c, Zone::Library, &reg);
                c
            })
            .collect();
        state.players[0].library_order = library.clone();

        let behavior = reg.get(state.get_object(phantasm).unwrap().card_id).unwrap();
        behavior.resolve_activated_ability(&mut state, phantasm, 0, &[], &reg);

        assert_eq!(state.get_object(phantasm).unwrap().zone, Zone::Battlefield,
            "the Phantasm is always found — it was shuffled into the library it is milling");

        let still_in_library = &state.players[0].library_order;
        for card in &library {
            let zone = state.get_object(*card).unwrap().zone;
            if still_in_library.contains(card) {
                assert_eq!(zone, Zone::Library,
                    "a card left below the Phantasm stays in the library");
            } else {
                assert_eq!(zone, Zone::Graveyard,
                    "a card revealed above the Phantasm goes to the graveyard, not {zone:?}");
                saw_a_mill = true;
            }
        }
        assert!(!still_in_library.contains(&phantasm),
            "the Phantasm left the library for the battlefield");
    }
    assert!(saw_a_mill, "20 shuffles never once put a card above the Phantasm");
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
    state = activate(&state, &reg, grimoire, 0, vec![]);

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
    state = activate(&state, &reg, grimoire, 0, vec![]);

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

        state = activate(&state, &reg, grimoire, 0, vec![]);

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
    state.move_object(gy1, Zone::Graveyard, &reg);

    let gy2 = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(gy2).unwrap().name = "Creature B".into();
    state.move_object(gy2, Zone::Graveyard, &reg);

    // Activate ability 1 via the engine (tap + sacrifice + remove counters).
    state = activate(&state, &reg, grimoire, 1, vec![]);

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

// -------------------------------------------------------------------------
// Creepy Doll
// -------------------------------------------------------------------------

/// The trigger should fire when `CombatDamageDealt` event targets a creature.
#[test]
fn trigger_fires_on_combat_damage_to_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, doll, P1, &[target]);

    // Emit combat damage event (doll deals 1 damage to target creature).
    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Object(target),
        amount: 1,
    });

    // Collect triggers.
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // Should have a trigger on the stack for Creepy Doll's ability.
    let has_trigger = state.stack.iter().any(|entry| matches!(entry, StackEntry::Trigger(_)));
    assert!(has_trigger,
        "Should have a trigger on the stack for Creepy Doll's combat damage to creature");
}

/// The trigger should NOT fire when `CombatDamageDealt` targets a player.
#[test]
fn trigger_does_not_fire_on_combat_damage_to_player() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    attacks_unblocked(&mut state, doll, P1);

    // Emit combat damage event (doll deals 1 damage to player).
    state.events.push(GameEvent::CombatDamageDealt {
        source: doll,
        target: DamageTarget::Player(P1),
        amount: 1,
    });

    // Collect triggers.
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // Should NOT have a trigger on the stack (Creepy Doll doesn't have CombatDamageToPlayer).
    let has_trigger = state.stack.iter().any(|entry| matches!(entry, StackEntry::Trigger(_)));
    assert!(!has_trigger,
        "Should NOT trigger on combat damage to player");
}

/// The `on_deals_combat_damage_to_creature` hook calls `try_destroy` on win.
#[test]
fn on_deals_combat_damage_to_creature_calls_destroy() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let doll = named_permanent(&mut state, &reg, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 3, 3);

    // Call the hook directly many times to verify it can destroy.
    // (Due to randomness, we call it many times and check that at least one destroys.)
    let card_id = state.get_object(doll).unwrap().card_id;
    let behavior = reg.get(card_id).unwrap();

    let mut any_destroyed = false;
    for _ in 0..50 {
        let mut test_state = state.clone();
        behavior.on_deals_combat_damage_to_creature(&mut test_state, doll, target, 1, &reg);
        if test_state.get_object(target).is_some_and(|o| o.zone != Zone::Battlefield) {
            any_destroyed = true;
            break;
        }
    }
    assert!(any_destroyed, "Creepy Doll should eventually destroy the target creature");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: When Creepy Doll deals lethal combat damage to a creature
/// AND wins the coin flip, the creature should be destroyed by the
/// triggered ability even if it could regenerate from the lethal damage.
/// The ruling says these are separate events.
/// Note: This is hard to test deterministically due to the coin flip.
/// We test the simpler case: Creepy Doll's trigger fires even when
/// the creature already has lethal damage.
#[test]
fn bug_creepy_doll_trigger_with_lethal_damage() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let doll = named_permanent(&mut state, &registry, "Creepy Doll", P0);
    let target = ready_creature(&mut state, P1, 2, 1); // 1 toughness, will take lethal from 1 dmg

    // Simulate combat damage: Doll deals 1 to target (lethal for 1 toughness)
    if let Some(obj) = state.get_object_mut(target) {
        obj.damage_marked = 1;
        obj.damaged_by.push(doll);
    }

    // Give target a regeneration shield (to survive lethal damage)
    if let Some(obj) = state.get_object_mut(target) {
        obj.regeneration_shields = 1;
    }

    // The trigger should still fire (it's a separate "destroy" effect)
    let behavior = registry.get(state.get_object(doll).unwrap().card_id).unwrap();
    behavior.on_deals_combat_damage_to_creature(&mut state, doll, target, 1, &registry);

    // After the trigger (which calls try_destroy on a coin flip win),
    // the creature may survive (regeneration absorbs the destroy) or die.
    // The key question is whether the trigger FIRES at all — it should.
    // We can't control the coin flip, but we can verify the trigger ran
    // by checking if try_destroy was called (regeneration shield consumed).
    let _shields_after = state.get_object(target).unwrap().regeneration_shields;

    // If the coin flip was won AND try_destroy was called, the shield is consumed.
    // If the coin flip was lost, shields remain at 1.
    // Either way, the trigger should have fired. We verify by running SBAs
    // and checking the creature survived via regeneration.
    // Run the trigger multiple times to get at least one coin flip win.
    // If try_destroy is called on a win, the regeneration shield is consumed.
    // We reset and retry until we get a win (statistically guaranteed in ~10 tries).
    let mut won_at_least_once = false;
    for _ in 0..20 {
        // Reset target state
        if let Some(obj) = state.get_object_mut(target) {
            obj.regeneration_shields = 1;
            obj.damage_marked = 1;
            obj.zone = Zone::Battlefield;
        }

        behavior.on_deals_combat_damage_to_creature(&mut state, doll, target, 1, &registry);

        let shields = state.get_object(target).unwrap().regeneration_shields;
        if shields == 0 {
            // Coin flip was won, try_destroy was called, regeneration was consumed
            won_at_least_once = true;
            break;
        }
    }

    assert!(won_at_least_once,
        "After 20 attempts, Creepy Doll should have won at least one coin flip and called try_destroy");
}

// -------------------------------------------------------------------------
// Gutter Grime — the rest
// -------------------------------------------------------------------------

/// A slime counter and an Ooze per nontoken creature death, and the Oozes are
/// sized by the *current* counter count — so an Ooze made when the count was 1
/// is a 2/2 once the count reaches 2. That is what "power and toughness are
/// each equal to the number of slime counters on Gutter Grime" means: a
/// characteristic-defining ability, recomputed, not a size fixed at creation.
#[test]
fn every_ooze_is_sized_by_the_current_slime_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

    for expected in 1..=2 {
        let creature = ready_creature(&mut state, P0, 2, 2);
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, grime, CounterType::Slime), expected,
            "one slime counter per nontoken creature death");
        assert_eq!(count_tokens_named(&state, "Ooze"), expected as usize,
            "and one Ooze per death");

        // Every Ooze, including the ones made earlier, is the current size.
        let oozes: Vec<_> = state.objects.values()
            .filter(|o| o.is_token && o.zone == Zone::Battlefield && o.name == "Ooze")
            .map(|o| o.id)
            .collect();
        for ooze in oozes {
            assert_eq!(state.effective_power(ooze, &reg), Some(expected as i32),
                "with {expected} slime counter(s), every Ooze is {expected}/{expected}");
            assert_eq!(state.effective_toughness(ooze, &reg), Some(expected as i32));
        }
    }
}

/// "Whenever a **nontoken** creature **you control** dies" — two conditions,
/// and both need a row, since a Gutter Grime that ignored the condition
/// entirely would satisfy either one alone.
#[test]
fn gutter_grime_counts_only_your_own_nontoken_creatures() {
    // (whose creature, is it a token, does the slime counter arrive?)
    const CASES: &[(PlayerId, bool, bool)] = &[
        (P0, false, true),
        (P0, true, false),
        (P1, false, false),
    ];

    for &(controller, is_token, counts) in CASES {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);

        let creature = if is_token {
            let id = state.create_token("Spirit", controller, 1, 1,
                vec![Color::White], vec![CardType::Creature], vec![], &reg)[0];
            state.get_object_mut(id).unwrap().summoning_sick = false;
            id
        } else {
            ready_creature(&mut state, controller, 2, 2)
        };

        // Killed for real, so a dying token is removed from `state.objects` by
        // SBA 704.5d before the trigger resolves — the case where reading
        // `is_token` back off the dead object answers `false` and the "nontoken"
        // clause silently stops applying.
        kill_by_damage(&mut state, &reg, creature);
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, grime, CounterType::Slime), u32::from(counts),
            "controller=p{}, is_token={is_token}", controller.0);
        assert_eq!(count_tokens_named(&state, "Ooze"), usize::from(counts),
            "controller=p{}, is_token={is_token}", controller.0);
    }
}

/// The Oozes' size is read off Gutter Grime, so losing Gutter Grime makes them
/// 0/0 — and state-based actions then bury them (CR 704.5a).
#[test]
fn the_oozes_die_when_gutter_grime_leaves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let grime = named_permanent(&mut state, &reg, "Gutter Grime", P0);
    let creature = ready_creature(&mut state, P0, 2, 2);
    kill_by_damage(&mut state, &reg, creature);
    triggers::process_triggers(&mut state, &reg);

    let ooze = find_token_named(&state, "Ooze").expect("an Ooze was made");
    assert_eq!(state.effective_power(ooze, &reg), Some(1), "test precondition: a 1/1");

    state.move_object(grime, Zone::Graveyard, &reg);

    assert_eq!(state.effective_power(ooze, &reg), Some(0),
        "with no Gutter Grime there are no slime counters to count");
    assert_eq!(state.effective_toughness(ooze, &reg), Some(0));

    mtg_engine::sba::check_state_based_actions(&mut state, &reg);
    assert!(state.get_object(ooze).is_none(),
        "a 0-toughness token dies and, being a token, ceases to exist");
}

// -------------------------------------------------------------------------
// Unbreathing Horde — the rest
// -------------------------------------------------------------------------

/// Combat damage is prevented and a counter is removed.
#[test]
fn prevents_combat_damage_removes_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    // Give it 3 +1/+1 counters.
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    // Attacker attacks, Horde blocks.
    let attacker = ready_creature(&mut state, P1, 2, 2);
    attacks_blocked_by(&mut state, attacker, P0, &[horde]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The Horde should have taken no damage but lost a counter.
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Damage should be prevented");
    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    assert_eq!(counters, 2, "Should have lost one +1/+1 counter");
}

/// When Unbreathing Horde deals damage as attacker, the other creature still takes damage.
#[test]
fn still_deals_damage_to_others() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);

    let blocker = ready_creature(&mut state, P1, 2, 5);
    // Horde attacks, blocker blocks.
    attacks_blocked_by(&mut state, horde, P1, &[blocker]);

    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // The blocker should have taken damage from Horde (0 base + 3 counters = 3 power).
    assert!(state.get_object(blocker).unwrap().damage_marked > 0,
        "Blocker should take damage from Unbreathing Horde");
    // The Horde should have taken no damage (prevented).
    assert_eq!(state.get_object(horde).unwrap().damage_marked, 0,
        "Horde damage should be prevented");
}

/// ETB counter count is correct with zombies on battlefield and graveyard.
#[test]
fn enters_with_correct_counter_count() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Put 2 zombies on battlefield.
    let _z1 = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    let _z2 = named_permanent(&mut state, &reg, "Diregraf Ghoul", P0);

    // Put 1 zombie in graveyard.
    let _z3 = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    // Cast Unbreathing Horde — on_resolve counts graveyard before moving to battlefield.
    let horde = castable_spell(&mut state, &reg, "Unbreathing Horde", P0);
    state = cast_and_resolve(&state, &reg, horde, vec![]);

    let counters = state.get_object(horde).unwrap().counters
        .get(&CounterType::PlusOnePlusOne).copied().unwrap_or(0);
    // 2 battlefield zombies + 1 graveyard zombie = 3 counters.
    assert_eq!(counters, 3, "Should have 3 +1/+1 counters (2 bf + 1 gy zombies)");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug AC (`audits/AUDIT_BUGS.md)`: Unbreathing Horde under-counts when
/// reanimated from a graveyard. Per Scryfall ruling: "If Unbreathing
/// Horde enters from a graveyard, it counts itself for its enter-with-
/// counters ability."
///
/// Oracle (Unbreathing Horde): "This creature enters with a +1/+1
/// counter on it for each other Zombie you control and each Zombie
/// card in your graveyard."
///
/// "Enters with X counters" is a CR 614.1c replacement effect, so the
/// count is computed at entry timing — at which point the Horde is
/// still in the graveyard zone (it hasn't fully entered yet) and the
/// "Zombie cards in your graveyard" count includes the Horde itself.
///
/// Failure mode: `unbreathing_horde.rs` runs the
/// `add_zombie_counters` helper from the `on_enter_battlefield`
/// handler — i.e. AFTER the move to battlefield. By that point,
/// `count_zombies_in_graveyard` no longer sees the Horde (it's on
/// the battlefield), so the reanimated Horde misses one counter
/// compared to the cast path.
///
/// We put two other Zombies in P0's graveyard alongside the Horde,
/// then move the Horde to the battlefield (mirroring Unburial Rites
/// reanimation), then fire the ETB handler. The fix should give the
/// Horde three +1/+1 counters (2 other Zombies + the Horde itself);
/// the bug gives it only two.
#[test]
fn bug_ac_unbreathing_horde_counts_itself_when_reanimated() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Two other Zombie creature cards in P0's graveyard.
    let walking_corpse_id = registry.get_id_by_name("Walking Corpse").unwrap();
    let z1 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z1).unwrap().name = "Walking Corpse (a)".into();
    let z2 = state.create_object(walking_corpse_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(z2).unwrap().name = "Walking Corpse (b)".into();

    // Unbreathing Horde sitting in P0's graveyard, ready to be reanimated.
    let horde_card_id = registry.get_id_by_name("Unbreathing Horde").unwrap();
    let horde = state.create_object(horde_card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(horde).unwrap().name = "Unbreathing Horde".into();

    // Reanimate: move the Horde to the battlefield and fire its ETB
    // handler (this mirrors what Unburial Rites does).
    state.move_object(horde, Zone::Battlefield, &registry);
    let behavior = registry.get(horde_card_id).unwrap();
    behavior.on_enter_battlefield(&mut state, horde, &[], &registry);

    let counters = state
        .get_object(horde)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0);

    assert!(
        counters >= 3,
        "Reanimated Unbreathing Horde should enter with at least 3 \
         +1/+1 counters (2 other Zombies in graveyard + the Horde \
         counts itself per the Scryfall ruling). Bug AC: \
         on_enter_battlefield runs after the move, so the helper sees \
         only the 2 other Zombies in the graveyard and adds 2 counters. \
         Got: {counters}",
    );
}

/// "…put it onto the battlefield attached to **target player**" is a targeted
/// triggered ability, so the player is chosen as the trigger goes on the stack
/// (CR 603.3d) — not part-way through its resolution.
///
/// It used to be asked for at resolution, after the search: the trigger was
/// declared `target_requirement: None` and the card built its own player list.
/// An opponent responding to the trigger could not know whom it would hit, and
/// CR 608.2b never re-checked the choice. It also meant hexproof had to be
/// filtered by hand in the card rather than once, in the engine, for everything
/// that targets a player.
#[test]
fn bitterheart_witch_targets_its_player_when_the_trigger_goes_on_the_stack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    let data = reg.card_data(state.get_object(witch).unwrap().card_id).unwrap();
    let trigger = data.triggered_abilities.first().expect("the death trigger");
    assert!(trigger.target_requirement.is_some(),
        "the trigger must declare its target so the engine chooses it at \
         CR 603.3d time; got {:?}", trigger.target_requirement);

    // With no target handed to it, the trigger does nothing rather than
    // inventing one mid-resolution.
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_object_mut(curse).unwrap().name = "Curse of the Pierced Heart".into();
    state.get_player_mut(P0).library_order.push(curse);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[], &reg);
    assert!(state.awaiting_action.is_none(),
        "no target, no ability — it must not fall back to asking at resolution");
}

/// CR 701.19b: "If a player is instructed to search a hidden zone for cards
/// with a stated quality ... that player isn't required to find some or all of
/// those cards even if they're present."
///
/// So Bitterheart Witch's controller may search, decline the only Curse in the
/// library, and still have shuffled. The code used to take the Curse for them
/// whenever exactly one was there.
#[test]
fn bitterheart_witch_may_search_and_decline_the_only_curse() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let witch = named_permanent(&mut state, &reg, "Bitterheart Witch", P0);
    let curse_card_id = reg.get_id_by_name("Curse of the Pierced Heart").unwrap();
    let curse_obj = state.create_object(curse_card_id, P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(curse_obj);

    let behavior = reg.get(state.get_object(witch).unwrap().card_id).unwrap();
    behavior.on_dies(&mut state, witch, &[Target::Player(P1)], &reg);

    // Yes, search.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(true) },
        &reg,
    );
    assert!(state.awaiting_action.is_some(),
        "the Curse is offered, not taken — even though there is only one");

    // Decline the find.
    state = engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) },
        &reg,
    );

    assert_eq!(state.get_object(curse_obj).unwrap().zone, Zone::Library,
        "declining leaves the Curse in the library");
    assert_eq!(state.get_object(curse_obj).unwrap().attached_to_player, None);
}
