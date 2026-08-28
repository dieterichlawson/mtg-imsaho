//! Creatures that trigger on dealing combat damage — the "whenever this deals
//! combat damage to a player" family, and the watchers over it.
//!
//! Cards covered (10), so this is greppable by name as well as by rule:
//!
//! - Abattoir Ghoul
//! - Balefire Dragon
//! - Bloodcrazed Neonate
//! - Champion of the Parish
//! - Curiosity
//! - Falkenrath Marauders
//! - Rakish Heir
//! - Stromkirk Noble
//! - Stromkirk Patrol
//! - Sturmgeist

mod common;

use common::*;
use mtg_engine::combat;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::triggers;
use mtg_engine::types::*;
// ── Abattoir Ghoul ────────────────────────────────────────────────

/// Abattoir Ghoul gains life when a creature it damaged this turn dies.
#[test]
fn abattoir_ghoul_gains_life_from_damaged_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3); // 2/3 creature

    // Simulate the ghoul having dealt damage to the victim this turn.
    state.get_object_mut(victim).unwrap().damaged_by.push(ghoul);
    // Mark lethal damage on the victim (3 toughness, 3 damage).
    state.get_object_mut(victim).unwrap().damage_marked = 3;

    // SBA should kill the victim.
    check_state_based_actions(&mut state, &reg);
    // Process death triggers.
    triggers::process_triggers(&mut state, &reg);

    // Ghoul's controller (P0) should have gained 3 life (victim's toughness).
    assert_eq!(state.get_player(P0).life, 23, "should gain life equal to victim's toughness");
}

/// Abattoir Ghoul does NOT gain life if it didn't damage the dying creature.
#[test]
fn abattoir_ghoul_no_life_if_not_damaged_by_ghoul() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3);

    // Victim dies without being damaged by the ghoul.
    state.get_object_mut(victim).unwrap().damage_marked = 3;

    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 20, "should NOT gain life");
}

/// Abattoir Ghoul uses last-known toughness (including +1/+1 counters).
#[test]
fn abattoir_ghoul_uses_last_known_toughness_with_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3); // base 2/3

    // Give the victim a +1/+1 counter (effective toughness = 4).
    state.add_counters(victim, CounterType::PlusOnePlusOne, 1);

    // Ghoul damaged it.
    state.get_object_mut(victim).unwrap().damaged_by.push(ghoul);
    // Mark lethal damage (4 toughness with counter, 4 damage).
    state.get_object_mut(victim).unwrap().damage_marked = 4;

    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    // Should gain 4 life (last-known toughness with counter), not 3 (base).
    assert_eq!(state.get_player(P0).life, 24, "should gain life = last-known toughness including counters");
}

/// The ruling's own example: "if Abattoir Ghoul deals 3 first-strike damage to
/// a 7/7 creature and then you give the creature -5/-5 before the regular
/// combat damage step, you'll gain 2 life."
///
/// The existing counters test shrinks nothing — it adds a +1/+1 counter, so a
/// reading that took the *printed* toughness would be wrong by one in the same
/// direction as one that took the base. This is the case the ruling actually
/// describes.
#[test]
fn abattoir_ghoul_gains_the_toughness_the_creature_died_with() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 7, 7);

    mtg_engine::damage::deal_damage(&mut state, ghoul,
        mtg_engine::events::DamageTarget::Object(victim), 3,
        mtg_engine::damage::DamageKind::Combat, &reg);
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "test setup: 3 damage does not kill a 7/7");

    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ModifyPT {
        target: victim, power_mod: -5, toughness_mod: -5,
    });
    state.events.clear();
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 22,
        "2 life — the toughness it had when it died, not the 7 it was printed with");
}

/// "a creature **dealt damage by this creature this turn**" is a fact about
/// the turn, not about damage still marked on the creature.
///
/// Regenerating removes the damage (CR 701.15a) — it does not un-deal it. A
/// creature the Ghoul damaged, that regenerated and then died later the same
/// turn, still feeds the Ghoul. `regenerate` used to clear `damaged_by`
/// alongside the marked damage, so it did not.
#[test]
fn abattoir_ghoul_still_gains_life_after_the_victim_regenerated() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3);
    state.add_regeneration_shield(victim);

    // Through the damage pipeline, so `damaged_by` is recorded the way the
    // game records it rather than pushed by hand.
    mtg_engine::damage::deal_damage(&mut state, ghoul,
        mtg_engine::events::DamageTarget::Object(victim), 3,
        mtg_engine::damage::DamageKind::Combat, &reg);

    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
        "test setup: the shield saved it");
    assert_eq!(state.get_object(victim).unwrap().damage_marked, 0,
        "and regenerating removed the damage");

    // It dies later the same turn, to something else.
    state.get_object_mut(victim).unwrap().damage_marked = 3;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 23,
        "the Ghoul dealt it damage this turn, so its death still gains 3 life");
}

/// The other half of "this turn": a creature the Ghoul damaged on an earlier
/// turn is not one it damaged *this* turn. Cleanup clears the record along
/// with the damage (CR 514.2).
#[test]
fn abattoir_ghoul_gains_nothing_from_a_creature_damaged_on_an_earlier_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3);

    mtg_engine::damage::deal_damage(&mut state, ghoul,
        mtg_engine::events::DamageTarget::Object(victim), 1,
        mtg_engine::damage::DamageKind::Combat, &reg);
    assert!(state.get_object(victim).unwrap().damaged_by.contains(&ghoul),
        "test setup: the damage was recorded");

    stock_library(&mut state, &reg, P0, 5);
    stock_library(&mut state, &reg, P1, 5);
    advance_to_next_turn(&mut state, &reg);
    let life_before = state.get_player(P0).life;

    state.get_object_mut(victim).unwrap().damage_marked = 3;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, life_before,
        "the damage was dealt on a previous turn, so this death gains nothing");
}

/// The two halves together: damaged, regenerated, and dead on the *next* turn.
///
/// Regeneration leaves no marked damage, so a cleanup that only visited
/// creatures with `damage_marked > 0` never reached this one — and once
/// `regenerate` stopped clearing `damaged_by` itself, the record would have
/// survived into the next turn and paid out there. Cleanup visits every
/// permanent now.
#[test]
fn abattoir_ghouls_record_of_a_regenerated_creature_still_ends_with_the_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ghoul = named_permanent(&mut state, &reg, "Abattoir Ghoul", P0);
    let victim = ready_creature(&mut state, P1, 2, 3);
    state.add_regeneration_shield(victim);

    mtg_engine::damage::deal_damage(&mut state, ghoul,
        mtg_engine::events::DamageTarget::Object(victim), 3,
        mtg_engine::damage::DamageKind::Combat, &reg);
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(victim).unwrap().damage_marked, 0,
        "test setup: it regenerated, so nothing is marked on it");

    stock_library(&mut state, &reg, P0, 5);
    stock_library(&mut state, &reg, P1, 5);
    advance_to_next_turn(&mut state, &reg);
    let life_before = state.get_player(P0).life;

    state.get_object_mut(victim).unwrap().damage_marked = 3;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, life_before,
        "a new turn, so the Ghoul did not damage it this turn");
}

// ── Champion of the Parish ────────────────────────────────────────

/// Champion of the Parish gets a +1/+1 counter when another Human enters.
#[test]
fn champion_of_the_parish_counter_on_human_etb() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // Another Human enters the battlefield under our control.
    let human = named_permanent(&mut state, &reg, "Unruly Mob", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P0,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, champion, CounterType::PlusOnePlusOne), 1,
        "Champion should get a +1/+1 counter");
}

/// Champion does NOT trigger on non-Human creatures entering.
#[test]
fn champion_of_the_parish_no_counter_on_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // A non-Human creature enters.
    let vampire = named_permanent(&mut state, &reg, "Rakish Heir", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: vampire,
        controller: P0,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, champion, CounterType::PlusOnePlusOne), 0,
        "Champion should NOT trigger on non-Human");
}

/// Champion does NOT trigger on opponent's Human.
#[test]
fn champion_of_the_parish_no_counter_on_opponent_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // Opponent's Human enters.
    let human = named_permanent(&mut state, &reg, "Unruly Mob", P1);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: human,
        controller: P1,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, champion, CounterType::PlusOnePlusOne), 0,
        "Champion should NOT trigger on opponent's Human");
}

/// "Whenever **another** Human you control enters." A Champion is a Human, and
/// its own arrival is not another's.
#[test]
fn champion_of_the_parish_does_not_count_its_own_arrival() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_permanent(&mut state, &reg, "Champion of the Parish", P0);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: champion,
        controller: P0,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, champion, CounterType::PlusOnePlusOne), 0);
}

/// CR 603.2: the condition is read as the creature enters, so a creature that
/// fails it never triggers at all — no stack entry, and no priority window
/// around one.
///
/// The three tests above count counters, and counting counters cannot tell a
/// trigger that never happened from one that resolved and did nothing.
#[test]
fn champion_of_the_parish_puts_nothing_on_the_stack_for_a_creature_it_does_not_care_about() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _champion = named_permanent(&mut state, &reg, "Champion of the Parish", P0);
    let vampire = named_permanent(&mut state, &reg, "Rakish Heir", P1);
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: vampire,
        controller: P1,
    });
    triggers::collect_triggers(&mut state, &reg);

    assert!(state.stack.is_empty(),
        "an opponent's non-Human is not this trigger's event: {:?}", state.stack);
}

/// The other side of reading the condition as the creature enters: a Human
/// that stops being one before the trigger resolves still earned the counter.
///
/// Village Ironsmith is a Human Werewolf, and Moonmist — an instant — reads
/// "transform all Human creatures", so transforming the entered creature with
/// the Champion's trigger on the stack is a real line of play.
#[test]
fn champion_of_the_parish_keeps_its_counter_when_the_human_stops_being_one() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let champion = named_permanent(&mut state, &reg, "Champion of the Parish", P0);
    let smith = named_permanent(&mut state, &reg, "Village Ironsmith", P0);
    assert!(state.has_subtype(smith, "Human", &reg), "test setup: it enters as a Human");

    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object: smith,
        controller: P0,
    });
    triggers::collect_triggers(&mut state, &reg);
    assert!(!state.stack.is_empty(), "test setup: the trigger is waiting");

    // Transformed in response — now a Werewolf, not a Human.
    mtg_engine::cards::helpers::apply_transform(&mut state, smith, &reg);
    assert!(!state.has_subtype(smith, "Human", &reg));

    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, champion, CounterType::PlusOnePlusOne), 1,
        "it was a Human when it entered, which is when the ability triggered");
}

// ── Stromkirk Noble ───────────────────────────────────────────────

/// Stromkirk Noble gets +1/+1 counter on combat damage to player.
/// Stromkirk Noble can't be blocked by Humans.
#[test]
fn stromkirk_noble_cant_be_blocked_by_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let noble = named_permanent(&mut state, &reg, "Stromkirk Noble", P0);

    // Create a Human blocker.
    let human = named_permanent(&mut state, &reg, "Unruly Mob", P1);

    // Set up combat.
    attacks_unblocked(&mut state, noble, P1);

    // Human should not be able to block the Noble.
    let can_block = combat::can_block_attacker(&state, human, noble, &reg);
    assert!(!can_block, "Humans should not be able to block Stromkirk Noble");

    // Non-Human should be able to block.
    let non_human = ready_creature(&mut state, P1, 2, 2);
    let can_block_nh = combat::can_block_attacker(&state, non_human, noble, &reg);
    assert!(can_block_nh, "Non-Humans should be able to block Stromkirk Noble");
}

/// "Humans" is read off the blocker's ACTIVE face, and half this set's Humans
/// are the front face of a werewolf. A transformed Village Ironsmith is a
/// Werewolf, not a Human, and can block the Noble — Moonmist on the defending
/// side is the line of play that gets there.
///
/// The test above uses Unruly Mob, which is a Human on both sides of nothing,
/// so it cannot tell an active-face read from a printed-front-face one.
#[test]
fn stromkirk_noble_can_be_blocked_by_a_human_that_has_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let noble = named_permanent(&mut state, &reg, "Stromkirk Noble", P0);
    let smith = named_permanent(&mut state, &reg, "Village Ironsmith", P1);
    attacks_unblocked(&mut state, noble, P1);

    assert!(state.has_subtype(smith, "Human", &reg), "test setup: it starts as a Human");
    assert!(!combat::can_block_attacker(&state, smith, noble, &reg),
        "and as a Human it cannot block");

    mtg_engine::cards::helpers::apply_transform(&mut state, smith, &reg);

    assert!(!state.has_subtype(smith, "Human", &reg),
        "its back face is a Werewolf");
    assert!(combat::can_block_attacker(&state, smith, noble, &reg),
        "so the restriction no longer reaches it");
}

// ── Rakish Heir ───────────────────────────────────────────────────

/// "Whenever a Vampire you control deals combat damage to a player, put a
/// +1/+1 counter on it." The counter goes on the Vampire that dealt the
/// damage, not on the Heir.
#[test]
fn rakish_heir_counter_on_other_vampire_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let heir = named_permanent(&mut state, &reg, "Rakish Heir", P0);
    let other_vamp = named_permanent(&mut state, &reg, "Stromkirk Noble", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: other_vamp,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 1,
    });
    triggers::process_triggers(&mut state, &reg);

    // Stromkirk Noble gets +1/+1 from its own trigger AND +1/+1 from Rakish Heir.
    assert_eq!(counters_of(&state, other_vamp, CounterType::PlusOnePlusOne), 2,
        "one counter from the Noble's own trigger, one from the Heir");
    assert_eq!(counters_of(&state, heir, CounterType::PlusOnePlusOne), 0,
        "and none on the Heir — \"it\" is the Vampire that dealt the damage");
}

/// "A Vampire **you control**". An opponent's Vampire connecting is the same
/// event and not this trigger's, and the Heir is the one who has to know the
/// difference: the condition is answered as the damage is dealt (CR 603.2),
/// in `should_trigger_on_damage_to_player`.
///
/// Vampire Interloper has no trigger of its own, so any counter on it came
/// from the Heir.
#[test]
fn rakish_heir_ignores_a_vampire_an_opponent_controls() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _heir = named_permanent(&mut state, &reg, "Rakish Heir", P0);
    let theirs = named_permanent(&mut state, &reg, "Vampire Interloper", P1);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: theirs,
        target: mtg_engine::events::DamageTarget::Player(P0),
        amount: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, theirs, CounterType::PlusOnePlusOne), 0,
        "a Vampire the Heir's controller does not control gets nothing");
}

/// The Heir is itself a Vampire its controller controls, so it counters
/// itself — for that reason, not because its trigger says "this creature".
#[test]
fn rakish_heir_counters_itself_as_one_of_the_vampires_you_control() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let heir = named_permanent(&mut state, &reg, "Rakish Heir", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: heir,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, heir, CounterType::PlusOnePlusOne), 1);
}

/// CR 121.1: a counter can only be put on a permanent that is there to take
/// it. A Vampire that traded with a blocker in the same combat damage step is
/// in the graveyard by the time the trigger resolves, and gets nothing — which
/// is a different thing from the *Heir* dying, where CR 113.7a means the
/// trigger resolves anyway.
#[test]
fn rakish_heir_gives_nothing_to_a_vampire_that_died_dealing_the_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _heir = named_permanent(&mut state, &reg, "Rakish Heir", P0);
    let doomed = named_permanent(&mut state, &reg, "Vampire Interloper", P0);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: doomed,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });
    // It dealt its damage and died to the blocker in the same step.
    state.move_object(doomed, Zone::Graveyard, &reg);
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, doomed, CounterType::PlusOnePlusOne), 0,
        "there is no permanent left to put a counter on");
}

/// Rakish Heir does NOT give +1/+1 to non-Vampire creatures.
#[test]
fn rakish_heir_no_counter_on_non_vampire() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let _heir = named_permanent(&mut state, &reg, "Rakish Heir", P0); // watcher
    let non_vamp = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(non_vamp).unwrap().summoning_sick = false;

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: non_vamp,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 3,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(counters_of(&state, non_vamp, CounterType::PlusOnePlusOne), 0,
        "Non-Vampire should NOT get a counter from Rakish Heir");
}

// ── "Whenever this deals combat damage to a player, put a +1/+1 counter on it" ──

/// The cards in the set that share this ability verbatim apart from how many
/// counters they get. They had a test each that differed only in a name and a
/// number; the shape is the point, so it is one table.
///
/// Rakish Heir is deliberately not here. It counters itself too, but for a
/// different reason — "a Vampire you control", which happens to include
/// itself — and its own tests are the ones that say so.
///
/// The table is checked against the registry: a new card with a
/// combat-damage-to-player trigger that counters itself has to be added here,
/// or the coverage assertion below fails.
const SELF_COUNTER_ON_COMBAT_DAMAGE: &[(&str, u32)] = &[
    ("Stromkirk Noble", 1),
    ("Stromkirk Patrol", 1),
    ("Falkenrath Marauders", 2),
    // Found by the coverage check below — the four hand-written tests this
    // table replaced never covered the Neonate's counter at all.
    ("Bloodcrazed Neonate", 1),
];

#[test]
fn a_self_countering_creature_gets_its_counters_on_combat_damage() {
    let reg = registry();
    for (name, expected) in SELF_COUNTER_ON_COMBAT_DAMAGE {
        let mut state = game_at_step(Step::CombatDamage, P0);
        let creature = named_permanent(&mut state, &reg, name, P0);

        state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
            source: creature,
            target: mtg_engine::events::DamageTarget::Player(P1),
            amount: 2,
        });
        triggers::process_triggers(&mut state, &reg);

        assert_eq!(counters_of(&state, creature, CounterType::PlusOnePlusOne), *expected,
            "{name} should get {expected} +1/+1 counter(s) for damaging a player");
    }
}

/// The table above must not drift out of date. Every card whose
/// combat-damage-to-player trigger says it counters itself has to be in it.
#[test]
fn the_self_counter_table_covers_every_such_card() {
    let reg = registry();
    let listed: std::collections::BTreeSet<&str> =
        SELF_COUNTER_ON_COMBAT_DAMAGE.iter().map(|(n, _)| *n).collect();

    let mut missing = Vec::new();
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        let counters_itself = data.triggered_abilities.iter().any(|a| {
            matches!(a.kind, mtg_engine::cards::TriggerKind::CombatDamageToPlayer)
                && a.description.contains("+1/+1 counter")
                && !a.description.contains("that creature")
        });
        if counters_itself && !listed.contains(name) {
            missing.push((*name).to_string());
        }
    }
    assert!(missing.is_empty(),
        "these cards counter themselves on combat damage but are not in the table: {missing:?}");
}

// ── Bloodcrazed Neonate ───────────────────────────────────────────

/// Bloodcrazed Neonate must attack each combat (`ForceAttack`).
#[test]
fn bloodcrazed_neonate_forced_to_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let neonate = named_permanent(&mut state, &reg, "Bloodcrazed Neonate", P0);

    // Check that the neonate has ForceAttack via continuous effects.
    let has_force_attack = state.has_effect(neonate, &|e| matches!(e, ContinuousEffect::ForceAttack { .. }), &reg);
    assert!(has_force_attack, "Bloodcrazed Neonate should have ForceAttack");
}

// ── Sturmgeist ────────────────────────────────────────────────────

/// "Sturmgeist's power and toughness are each equal to the number of cards in
/// **your** hand." The opponent's hand is not it — reading theirs instead fails
/// this, which is why P1 holds a different number.
#[test]
fn sturmgeist_pt_equals_hand_size() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sturmgeist = named_permanent(&mut state, &reg, "Sturmgeist", P0);

    for _ in 0..4 {
        state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Hand, None, None);
    }
    for _ in 0..2 {
        state.create_object(mtg_engine::ids::CardId(9999), P1, Zone::Hand, None, None);
    }

    assert_eq!(state.effective_power(sturmgeist, &reg), Some(4),
        "power equals the controller's hand size");
    assert_eq!(state.effective_toughness(sturmgeist, &reg), Some(4),
        "and so does toughness");
}

/// Ruling (2011-09-22): "The ability that defines Sturmgeist's power and
/// toughness works in all zones, not just the battlefield."
///
/// That is CR 604.3 — a characteristic-defining ability functions everywhere.
/// Gating the card's `dynamic_pt` on `zone == Battlefield` passed the whole
/// workspace.
///
/// CR 109.5: a card outside the battlefield has no controller, and its owner
/// acts as one — which `move_object` arranges, so "your hand" keeps meaning the
/// owner's once the card is in a graveyard.
#[test]
fn sturmgeists_defining_ability_works_outside_the_battlefield() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let sturmgeist = named_card_in_graveyard(&mut state, &reg, "Sturmgeist", P0);
    assert_eq!(state.get_object(sturmgeist).unwrap().zone, Zone::Graveyard,
        "test precondition");
    assert_eq!(state.effective_power(sturmgeist, &reg), Some(0),
        "an empty hand, and it is not in hand to count itself");

    for _ in 0..3 {
        state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Hand, None, None);
    }
    assert_eq!(state.effective_power(sturmgeist, &reg), Some(3),
        "the defining ability still runs while the card sits in a graveyard");
    assert_eq!(state.effective_toughness(sturmgeist, &reg), Some(3));
}

// ── Balefire Dragon ───────────────────────────────────────────────

/// "it deals **that much** damage to each creature **that player** controls."
///
/// Both quantities come from the damage that was actually dealt, not from the
/// Dragon: the amount is 4 here rather than the Dragon's printed 6, so an
/// implementation reading `effective_power` would be caught, and only the
/// damaged player's creatures are hit.
#[test]
fn balefire_dragon_sweeps_opponent_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let dragon = named_permanent(&mut state, &reg, "Balefire Dragon", P0);
    let opp_creature1 = ready_creature(&mut state, P1, 3, 9);
    let opp_creature2 = ready_creature(&mut state, P1, 2, 9);
    let own_creature = ready_creature(&mut state, P0, 1, 9); // should NOT be damaged

    // Four, not the Dragon's six: what got through is what the ability deals.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: dragon,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 4,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(opp_creature1).unwrap().damage_marked, 4);
    assert_eq!(state.get_object(opp_creature2).unwrap().damage_marked, 4);
    assert_eq!(state.get_object(own_creature).unwrap().damage_marked, 0,
        "\"each creature that player controls\" is the damaged player's, not everyone's");
}

/// Ruling (2018-12-07): "The damage dealt by Balefire Dragon's triggered
/// ability isn't combat damage."
///
/// Inquisitor's Flail says the difference out loud — "If another creature
/// would deal **combat damage** to equipped creature, it deals double that
/// damage instead" — so a defending creature wearing one takes 6 from the
/// trigger, not 12. Anything that treated the sweep as combat damage would
/// double it, and the sweep is the Dragon's whole card.
#[test]
fn balefire_dragons_sweep_is_not_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    let dragon = named_permanent(&mut state, &reg, "Balefire Dragon", P0);
    let victim = ready_creature(&mut state, P1, 3, 20);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P1);
    state.get_object_mut(flail).unwrap().attached_to = Some(victim);

    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: dragon,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 6,
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().damage_marked, 6,
        "the Flail doubles combat damage, and this is not combat damage");
}

// ── Curiosity ─────────────────────────────────────────────────────

/// Curiosity draws a card when enchanted creature deals combat damage to opponent (player says yes).
#[test]
fn curiosity_draw_on_enchanted_creature_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Create a creature and attach Curiosity to it.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let curiosity_card_id = reg.get_id_by_name("Curiosity").unwrap();
    let curiosity = state.create_object(curiosity_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(curiosity).unwrap().name = "Curiosity".into();
    state.get_object_mut(curiosity).unwrap().attached_to = Some(creature);

    // Give P0 a card in library to draw.
    let lib_card = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    let hand_before = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();

    // Enchanted creature deals combat damage to opponent.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: creature,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a yes/no choice for "you may draw a card".
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses yes.
    state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(true),
        },
        &reg,
    );

    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, hand_before + 1, "Should have drawn 1 card from Curiosity");
}

/// Curiosity does NOT draw a card when the player declines.
#[test]
fn curiosity_decline_draw() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);

    // Create a creature and attach Curiosity to it.
    let creature = ready_creature(&mut state, P0, 2, 2);
    let curiosity_card_id = reg.get_id_by_name("Curiosity").unwrap();
    let curiosity = state.create_object(curiosity_card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(curiosity).unwrap().name = "Curiosity".into();
    state.get_object_mut(curiosity).unwrap().attached_to = Some(creature);

    // Give P0 a card in library to draw.
    let lib_card = state.create_object(mtg_engine::ids::CardId(9999), P0, Zone::Library, None, None);
    state.get_player_mut(P0).library_order.push(lib_card);

    let hand_before = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();

    // Enchanted creature deals combat damage to opponent.
    state.events.push(mtg_engine::events::GameEvent::CombatDamageDealt {
        source: creature,
        target: mtg_engine::events::DamageTarget::Player(P1),
        amount: 2,
    });

    triggers::process_triggers(&mut state, &reg);

    // Should be awaiting a yes/no choice.
    assert!(state.awaiting_action.is_some(), "Should be awaiting yes/no choice");

    // Player chooses no.
    state = mtg_engine::engine::submit_action(
        &state,
        &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::YesNoDecision(false),
        },
        &reg,
    );

    let hand_after = state.objects.values()
        .filter(|o| o.zone == Zone::Hand && o.owner == P0)
        .count();
    assert_eq!(hand_after, hand_before, "Should NOT have drawn a card when declining");
}
