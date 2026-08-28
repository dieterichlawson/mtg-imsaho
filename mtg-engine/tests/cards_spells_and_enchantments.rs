//! Instants, sorceries and enchantments whose behaviour is particular to the
//! card rather than to a rule the engine implements generally.
//!
//! Cards covered (13), so this is greppable by name as well as by rule:
//!
//! - Angelic Overseer
//! - Army of the Damned
//! - Ashmouth Hound
//! - Blasphemous Act
//! - Burning Vengeance
//! - Cackling Counterpart
//! - Elite Inquisitor
//! - Hamlet Captain
//! - Night Revelers
//! - Scourge of Geier Reach
//! - Sever the Bloodline
//! - Spare from Evil
//! - Traitorous Blood

mod common;

use common::*;
use mtg_engine::events::GameEvent;
use mtg_engine::triggers;
use mtg_engine::types::*;
use mtg_engine::actions::Target;
// ── Scourge of Geier Reach ──────────────────────────────────────

/// "Scourge of Geier Reach gets +1/+1 for each creature your opponents
/// control" — a characteristic-defining count that has to be recomputed as the
/// board changes, and has to count the right half of the board.
#[test]
fn scourge_of_geier_reach_counts_only_opponents_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let scourge = named_permanent(&mut state, &reg, "Scourge of Geier Reach", P0);
    let pt = |s: &mtg_engine::state::GameState| {
        (s.effective_power(scourge, &reg).unwrap(), s.effective_toughness(scourge, &reg).unwrap())
    };

    assert_eq!(pt(&state), (3, 3), "an empty board leaves it at its printed 3/3");

    ready_creature(&mut state, P0, 1, 1);
    ready_creature(&mut state, P0, 2, 2);
    assert_eq!(pt(&state), (3, 3), "its controller's own creatures are not counted");

    ready_creature(&mut state, P1, 1, 1);
    ready_creature(&mut state, P1, 2, 2);
    assert_eq!(pt(&state), (5, 5), "two creatures across the table make it a 5/5");

    // "each **creature**" — an opponent's land is not one.
    named_permanent(&mut state, &reg, "Forest", P1);
    assert_eq!(pt(&state), (5, 5), "an opponent's noncreature permanent is not counted");
}

// ── Army of the Damned ──────────────────────────────────────────

/// Army creates 13 tapped Zombie tokens.
#[test]
fn army_of_the_damned_creates_13_tapped_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let spell = castable_spell(&mut state, &reg, "Army of the Damned", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Count tokens on battlefield.
    assert_eq!(count_tokens_named_by(&state, "Zombie Token", P0), 13, "Should have 13 Zombie tokens");

    // The name is "Zombie Token" — CR 111.4 derives it from the subtypes. This
    // loop said "Zombie" and so ran over nothing: every assertion in it was
    // vacuous, including the one about the thirteen Zombies being tapped, which
    // is the only interesting word in the card's text.
    let zombies: Vec<_> = state.objects.values()
        .filter(|o| o.is_token && o.name == "Zombie Token" && o.controller == P0)
        .collect();
    assert_eq!(zombies.len(), 13, "the loop below has to run over something");
    for z in zombies {
        assert!(z.tapped, "\"thirteen **tapped** ... tokens\"");
        assert_eq!((z.power, z.toughness), (Some(2), Some(2)));
        assert_eq!(z.colors, vec![Color::Black], "black Zombies");
        assert!(z.subtypes.iter().any(|s| s == "Zombie"));
    }
}

// ── Night Revelers ──────────────────────────────────────────────

/// Night Revelers has haste when opponent controls a Human.
#[test]
fn night_revelers_has_haste_with_opponent_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let revelers = named_permanent(&mut state, &reg, "Night Revelers", P0);

    // No opponent Humans: no haste.
    assert!(!state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should not have haste without opponent Human");

    // Add a Human creature to the opponent.
    let human = named_permanent(&mut state, &reg, "Champion of the Parish", P1);

    // Now should have haste.
    assert!(state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should have haste when opponent controls a Human");

    // Remove the Human.
    state.move_object(human, Zone::Graveyard, &reg);
    assert!(!state.has_keyword(revelers, Keyword::Haste, &reg),
        "Night Revelers should lose haste when opponent no longer controls a Human");
}

// ── Elite Inquisitor ────────────────────────────────────────────

/// Elite Inquisitor has protection from Vampires, Werewolves, Zombies.
/// Combat damage from those subtypes is prevented.
#[test]
fn elite_inquisitor_protection_prevents_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P1);

    let inquisitor = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);

    // Create a Vampire attacker.
    let vampire = named_permanent(&mut state, &reg, "Markov Patrician", P1);

    // Set up combat: vampire attacks, inquisitor blocks.
    attacks_blocked_by(&mut state, vampire, P0, &[inquisitor]);

    // Deal combat damage.
    mtg_engine::combat::deal_combat_damage(&mut state, &reg);

    // Elite Inquisitor should take no damage from the Vampire.
    let inq_obj = state.get_object(inquisitor).unwrap();
    assert_eq!(inq_obj.damage_marked, 0, "Elite Inquisitor should not take damage from Vampires (protection)");
}

/// Elite Inquisitor's protection prevents Zombies from blocking it.
#[test]
fn elite_inquisitor_cant_be_blocked_by_zombies() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let inquisitor = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    let zombie = named_permanent(&mut state, &reg, "Diregraf Ghoul", P1);

    // Zombie should not be able to block Elite Inquisitor (protection from Zombies).
    assert!(!mtg_engine::combat::can_block_attacker(&state, zombie, inquisitor, &reg),
        "Zombie should not be able to block Elite Inquisitor (protection from Zombies)");
}

// ── Ashmouth Hound ──────────────────────────────────────────────

/// Ashmouth Hound deals 1 damage when it blocks.
#[test]
fn ashmouth_hound_deals_damage_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);

    let hound = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
    let attacker = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(attacker).unwrap().name = "Enemy".into();

    // Set up combat.
    attacks_blocked_by(&mut state, attacker, P0, &[hound]);

    // Fire blockers declared event.
    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(hound, attacker)],
    });
    triggers::process_triggers(&mut state, &reg);

    // The attacker should have 1 damage from Ashmouth Hound's trigger.
    let att = state.get_object(attacker).unwrap();
    assert_eq!(att.damage_marked, 1, "Ashmouth Hound should deal 1 damage to the creature it blocks");
}

/// The other half of the same ability: "or becomes **blocked by** a creature".
/// Only the blocking half was tested, so `on_becomes_blocked` — a separate
/// hook, reached by a separate trigger kind — was never exercised.
#[test]
fn ashmouth_hound_deals_damage_when_it_becomes_blocked() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let hound = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
    let blocker = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, hound, P1, &[blocker]);

    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(blocker, hound)],
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 1,
        "the Hound damages the creature that blocked it");
    assert_eq!(state.get_object(hound).unwrap().damage_marked, 0,
        "and takes nothing from its own ability");
}

/// Scryfall ruling (2011-09-22): "Ashmouth Hound's ability triggers once for
/// each creature it blocks or becomes blocked by."
///
/// Two blockers is two triggers, so 1 damage to each — not one trigger that
/// picks a blocker, and not 1 damage split between them.
#[test]
fn ashmouth_hound_triggers_once_per_creature_blocking_it() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let hound = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
    let first = ready_creature(&mut state, P1, 3, 3);
    let second = ready_creature(&mut state, P1, 3, 3);
    attacks_blocked_by(&mut state, hound, P1, &[first, second]);

    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(first, hound), (second, hound)],
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(first).unwrap().damage_marked, 1,
        "each blocker is dealt 1 — this is the first");
    assert_eq!(state.get_object(second).unwrap().damage_marked, 1,
        "and the second, rather than the two sharing one trigger");
}

/// "This creature **deals 1 damage** to that creature" is a triggered
/// ability's damage, not combat damage. Inquisitor's Flail doubles "combat
/// damage" only, so it must leave this alone — the Hound's own combat damage
/// in the same combat is a different event and is doubled.
#[test]
fn ashmouth_hounds_trigger_damage_is_not_combat_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);

    let hound = named_permanent(&mut state, &reg, "Ashmouth Hound", P0);
    let flail = named_permanent(&mut state, &reg, "Inquisitor's Flail", P0);
    state.get_object_mut(flail).unwrap().attached_to = Some(hound);

    let blocker = ready_creature(&mut state, P1, 9, 9);
    attacks_blocked_by(&mut state, hound, P1, &[blocker]);

    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(blocker, hound)],
    });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 1,
        "the trigger deals 1, undoubled — the Flail says 'combat damage'");
}

// ── Hamlet Captain ──────────────────────────────────────────────

/// Hamlet Captain gives other Humans +1/+1 when it attacks.
#[test]
fn hamlet_captain_buffs_humans_on_attack() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let captain = named_permanent(&mut state, &reg, "Hamlet Captain", P0);
    let human = named_permanent(&mut state, &reg, "Champion of the Parish", P0);
    let non_human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(non_human).unwrap().name = "Bear".into();

    // Declare attackers event with Hamlet Captain attacking.
    state.events.push(GameEvent::AttackersDeclared {
        attackers: vec![(captain, P1)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Champion of the Parish should have +1/+1 buff.
    let champion_power = state.effective_power(human, &reg).unwrap();
    assert_eq!(champion_power, 2, "Champion should be 2 power (1 base + 1 from Hamlet Captain)");

    // Non-Human should not be affected.
    let bear_power = state.effective_power(non_human, &reg).unwrap();
    assert_eq!(bear_power, 2, "Non-human should still be 2 power");

    // Hamlet Captain itself should not get the buff (it says "other").
    let captain_power = state.effective_power(captain, &reg).unwrap();
    assert_eq!(captain_power, 2, "Hamlet Captain should not buff itself");
}

/// Hamlet Captain gives other Humans +1/+1 when it blocks.
#[test]
fn hamlet_captain_buffs_humans_on_block() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P1);

    let captain = named_permanent(&mut state, &reg, "Hamlet Captain", P0);
    let human = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    let attacker = ready_creature(&mut state, P1, 3, 3);

    // Declare blockers event.
    state.events.push(GameEvent::BlockersDeclared {
        assignments: vec![(captain, attacker)],
    });
    triggers::process_triggers(&mut state, &reg);

    // Elite Inquisitor should have +1/+1 buff.
    let inq_power = state.effective_power(human, &reg).unwrap();
    assert_eq!(inq_power, 3, "Elite Inquisitor should be 3 power (2 base + 1 from Hamlet Captain)");
}

/// "other Humans you control" — an opponent's Humans are not yours, whichever
/// player the Captain's own `controller` field happens to say after it has
/// left the battlefield.
#[test]
fn hamlet_captain_does_not_pump_an_opponents_humans() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let captain = named_permanent(&mut state, &reg, "Hamlet Captain", P0);
    let mine = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    let theirs = named_permanent(&mut state, &reg, "Elite Inquisitor", P1);
    let theirs_before = state.effective_power(theirs, &reg).unwrap();

    state.events.push(GameEvent::AttackersDeclared { attackers: vec![(captain, P1)] });
    triggers::process_triggers(&mut state, &reg);

    assert_eq!(state.effective_power(mine, &reg).unwrap(), 3);
    assert_eq!(state.effective_power(theirs, &reg).unwrap(), theirs_before,
        "an opponent's Human is not a Human you control");
}

/// CR 611.2c: which Humans get the bonus is settled when the ability resolves.
/// A Human that arrives afterwards does not get it, and "until end of turn"
/// means the bonus is gone by the next turn.
#[test]
fn hamlet_captains_pump_covers_who_was_there_and_lasts_one_turn() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let captain = named_permanent(&mut state, &reg, "Hamlet Captain", P0);
    let early = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);

    state.events.push(GameEvent::AttackersDeclared { attackers: vec![(captain, P1)] });
    triggers::process_triggers(&mut state, &reg);
    assert_eq!(state.effective_power(early, &reg).unwrap(), 3);

    // A Human arriving after the ability resolved is not in the affected set.
    let late = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    assert_eq!(state.effective_power(late, &reg).unwrap(), 2,
        "the set of affected creatures was fixed at resolution (CR 611.2c)");

    advance_to_next_turn(&mut state, &reg);
    assert_eq!(state.effective_power(early, &reg).unwrap(), 2,
        "\"until end of turn\" ends at the cleanup step (CR 514.2)");
}

// ── Spare from Evil ─────────────────────────────────────────────

/// Spare from Evil gives protection from non-Human creatures.
#[test]
fn spare_from_evil_grants_protection() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];
    state.get_object_mut(human).unwrap().name = "Human Warrior".into();

    let spell = castable_spell(&mut state, &reg, "Spare from Evil", P0);
    let mut state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Create a non-Human attacker (Zombie).
    let zombie = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(zombie).unwrap().subtypes = vec!["Zombie".into()];
    state.get_object_mut(zombie).unwrap().name = "Zombie".into();

    // The Zombie should not be able to block our Human (protection from non-Humans).
    assert!(!mtg_engine::combat::can_block_attacker(&state, zombie, human, &reg),
        "Non-Human creature should not be able to block a creature with protection from non-Humans");

    // A Human attacker should still be able to block.
    let human_opp = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(human_opp).unwrap().subtypes = vec!["Human".into()];
    assert!(mtg_engine::combat::can_block_attacker(&state, human_opp, human, &reg),
        "Human creature should still be able to block (protection only from non-Humans)");

    // Ruling: "A creature that is a Human in addition to other creature types
    // is not a non-Human creature." Village Ironsmith is a Human Werewolf, so
    // the protection says nothing about it either.
    let ironsmith = named_permanent(&mut state, &reg, "Village Ironsmith", P1);
    assert!(mtg_engine::combat::can_block_attacker(&state, ironsmith, human, &reg),
        "a Human-plus-other-types creature is not a non-Human creature");
    // Transformed, it is an Ironfang — a Werewolf and no longer a Human — and
    // the protection reaches it.
    mtg_engine::cards::helpers::apply_transform(&mut state, ironsmith, &reg);
    assert!(!mtg_engine::combat::can_block_attacker(&state, ironsmith, human, &reg),
        "the same card on its back face is a non-Human creature");
}

/// "protection from non-Human **creatures**" — the creature half of that is not
/// decoration. Written as a bare "isn't a Human" filter it also matched every
/// instant, sorcery, artifact and land, so a burn spell could not touch the
/// protected creature at all.
#[test]
fn spare_from_evil_does_not_protect_against_a_noncreature_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let human = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(human).unwrap().subtypes = vec!["Human".into()];

    let spell = castable_spell(&mut state, &reg, "Spare from Evil", P0);
    let mut state = cast_and_resolve(&state, &reg, spell, vec![]);

    // Brimstone Volley is a red instant. It is not a Human — and it is also
    // not a creature, so the protection says nothing about it.
    let volley = state.create_object(
        reg.get_id_by_name("Brimstone Volley").unwrap(), P1, Zone::Stack, None, None);
    assert!(!state.has_protection_from(human, volley, &reg),
        "an instant is not a non-Human creature");

    mtg_engine::damage::deal_damage(&mut state, volley,
        mtg_engine::events::DamageTarget::Object(human), 3,
        mtg_engine::damage::DamageKind::NonCombat, &reg);
    assert_eq!(state.get_object(human).unwrap().damage_marked, 3,
        "the burn spell's damage lands");

    // A non-Human creature source is still stopped.
    let zombie = ready_creature(&mut state, P1, 3, 3);
    state.get_object_mut(zombie).unwrap().subtypes = vec!["Zombie".into()];
    assert!(state.has_protection_from(human, zombie, &reg),
        "a non-Human creature is exactly what it protects from");
}

// ── Burning Vengeance ───────────────────────────────────────────

/// Burning Vengeance deals 2 damage when you cast a flashback spell.
#[test]
fn burning_vengeance_triggers_on_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_permanent(&mut state, &reg, "Burning Vengeance", P0);

    // Create a flashback spell on the stack, marked as cast_with_flashback.
    let spell = state.create_object(
        reg.get_id_by_name("Think Twice").unwrap(),
        P0,
        Zone::Stack,
        None,
        None,
    );
    state.get_object_mut(spell).unwrap().cast_with_flashback = true;
    state.get_object_mut(spell).unwrap().name = "Think Twice".into();

    // Fire SpellCast event. CR 603.3d: "deals 2 damage to any target" needs a
    // target chosen as the trigger goes on the stack, so processing runs
    // through the helper that answers that prompt via `submit_action`, the way
    // a player would.
    state.events.push(GameEvent::SpellCast { player: P0, object: spell });
    process_triggers_auto_target_opponent(&mut state, &reg);
    // Opponent should have lost 2 life.
    assert_eq!(state.get_player(P1).life, 18,
        "Burning Vengeance should deal 2 damage to opponent on flashback cast");
}

/// Burning Vengeance does not trigger on normal spell casts.
#[test]
fn burning_vengeance_ignores_non_flashback() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_permanent(&mut state, &reg, "Burning Vengeance", P0);

    // Create a normal spell on the stack (not flashback).
    let spell = state.create_object(
        reg.get_id_by_name("Think Twice").unwrap(),
        P0,
        Zone::Stack,
        None,
        None,
    );
    state.get_object_mut(spell).unwrap().name = "Think Twice".into();
    // cast_with_flashback defaults to false.

    state.events.push(GameEvent::SpellCast { player: P0, object: spell });
    // Auto-answering the target prompt matters here: this ability targets, so
    // `process_triggers` alone leaves a trigger that DID fire sitting on
    // `awaiting_action` with no target chosen and no damage dealt — which
    // looks exactly like not triggering. The helper answers the prompt, so a
    // trigger that fired would resolve and be visible.
    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P1).life, 20,
        "Burning Vengeance should NOT trigger on normal spell casts");
    assert!(state.awaiting_action.is_none(),
        "and no trigger is waiting to be pointed at anything");
}

/// "Whenever **you** cast a spell from your graveyard." An opponent flashing
/// something back is not you, and neither existing test varies who casts — so
/// an implementation that ignored the caster passed both.
#[test]
fn burning_vengeance_ignores_an_opponents_flashback_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_permanent(&mut state, &reg, "Burning Vengeance", P0);

    let spell = state.create_object(
        reg.get_id_by_name("Think Twice").unwrap(), P1, Zone::Stack, None, None);
    state.get_object_mut(spell).unwrap().cast_with_flashback = true;
    state.get_object_mut(spell).unwrap().name = "Think Twice".into();

    state.events.push(GameEvent::SpellCast { player: P1, object: spell });
    process_triggers_auto_target_opponent(&mut state, &reg);

    assert_eq!(state.get_player(P0).life, 20,
        "P1's flashback cast is not \"you cast\", so nothing triggers");
    assert_eq!(state.get_player(P1).life, 20);
    assert!(state.awaiting_action.is_none(),
        "and nothing is waiting on a target — a trigger that fired and stalled \
         for want of one would leave both life totals untouched too");
}

/// Scryfall ruling (2025-01-24): "Burning Vengeance's triggered ability will
/// resolve before the spell you cast from your graveyard."
///
/// That is CR 603.3b — the trigger goes on the stack on top of the spell that
/// caused it — rather than anything the card does, but nothing asserted it,
/// and the card's whole tempo rests on it.
#[test]
fn burning_vengeances_trigger_sits_above_the_spell_that_caused_it() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _bv = named_permanent(&mut state, &reg, "Burning Vengeance", P0);

    let spell = state.create_object(
        reg.get_id_by_name("Think Twice").unwrap(), P0, Zone::Stack, None, None);
    state.get_object_mut(spell).unwrap().cast_with_flashback = true;
    state.get_object_mut(spell).unwrap().name = "Think Twice".into();
    state.stack.push(mtg_engine::state::StackEntry::Spell(spell));

    state.events.push(GameEvent::SpellCast { player: P0, object: spell });
    process_triggers_auto_target_opponent(&mut state, &reg);

    // The damage has already happened while the spell that caused it is still
    // sitting on the stack, unresolved. That is the ruling: the trigger goes
    // on top of the spell (CR 603.3b) and so resolves first.
    assert_eq!(state.get_player(P1).life, 18, "the 2 damage has been dealt");
    assert!(state.stack.iter().any(|e| matches!(e, mtg_engine::state::StackEntry::Spell(s) if *s == spell)),
        "and the flashback spell has not resolved yet; stack is {:?}", state.stack);
}

// ── Traitorous Blood ───────────────────────────────────────────

/// Traitorous Blood steals a creature, untaps it, and grants haste + trample.
#[test]
fn traitorous_blood_steals_untaps_and_grants_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a tapped creature controlled by opponent.
    let enemy = ready_creature(&mut state, P1, 4, 4);
    state.get_object_mut(enemy).unwrap().tapped = true;
    state.get_object_mut(enemy).unwrap().name = "Enemy Beast".into();

    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(enemy)]);

    // Creature should now be controlled by P0.
    let obj = state.get_object(enemy).unwrap();
    assert_eq!(obj.controller, P0, "Traitorous Blood should change controller to caster");
    assert!(!obj.tapped, "Traitorous Blood should untap the creature");

    // Should have haste and trample.
    assert!(state.has_keyword(enemy, Keyword::Haste, &reg),
        "Traitorous Blood should grant haste");
    assert!(state.has_keyword(enemy, Keyword::Trample, &reg),
        "Traitorous Blood should grant trample");
}

/// Ruling: "Traitorous Blood can target any creature, even one that's tapped
/// or one you already control."
///
/// "Target creature" with no restriction, so the engine must offer your own —
/// the untap and the two keywords are worth having on a creature you already
/// have.
#[test]
fn traitorous_blood_can_target_a_creature_you_already_control() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let mine = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(mine).unwrap().tapped = true;
    let theirs = ready_creature(&mut state, P1, 2, 2);

    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    let offered = offered_targets(&state, &reg, spell);
    assert!(offered.contains(&Target::Object(mine)),
        "your own creature is a legal target");
    assert!(offered.contains(&Target::Object(theirs)),
        "and so is the opponent's");

    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(mine)]);
    let obj = state.get_object(mine).unwrap();
    assert_eq!(obj.controller, P0, "it stays yours");
    assert!(!obj.tapped, "and is untapped");
    assert!(state.has_keyword(mine, Keyword::Haste, &reg));
    assert!(state.has_keyword(mine, Keyword::Trample, &reg));
}

/// Ruling: "Gaining control of a creature doesn't cause you gain control of
/// any Auras or Equipment attached to it."
#[test]
fn traitorous_blood_leaves_the_equipment_with_its_owner() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let victim = ready_creature(&mut state, P1, 2, 2);
    // Equipped on P1's own turn, before this one — equip is sorcery-speed
    // (CR 702.6b), so it cannot be driven through the engine during P0's main
    // phase. The attachment is the setup, not the claim.
    let cleaver = named_permanent(&mut state, &reg, "Butcher's Cleaver", P1);
    state.get_object_mut(cleaver).unwrap().attached_to = Some(victim);

    let spell = castable_spell(&mut state, &reg, "Traitorous Blood", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(victim)]);

    assert_eq!(state.get_object(victim).unwrap().controller, P0,
        "the creature changes hands");
    assert_eq!(state.get_object(cleaver).unwrap().controller, P1,
        "the Equipment does not");
    assert_eq!(state.get_object(cleaver).unwrap().attached_to, Some(victim),
        "and stays attached to it");
}

// ── Blasphemous Act ────────────────────────────────────────────

/// Blasphemous Act deals 13 damage to each creature.
#[test]
fn blasphemous_act_deals_13_damage_to_all_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let c1 = ready_creature(&mut state, P0, 2, 14);
    let c2 = ready_creature(&mut state, P1, 3, 3);
    // "13 damage to each **creature**" — not to each permanent.
    let land = named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    let equipment = named_permanent(&mut state, &reg, "Butcher's Cleaver", P1);

    // Add tons of mana to afford it even with no cost reduction.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 8);

    let spell = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![]);

    // c1 has 14 toughness, should have 13 damage.
    assert_eq!(state.get_object(c1).unwrap().damage_marked, 13,
        "Blasphemous Act should deal 13 damage to creature");

    // The 3-toughness creature takes the same 13; nothing here runs SBAs, so it
    // is still on the battlefield holding lethal damage.
    assert_eq!(state.get_object(c2).unwrap().damage_marked, 13,
        "Blasphemous Act should deal 13 damage to opponent's creature too");

    assert_eq!(state.get_object(land).unwrap().damage_marked, 0,
        "a land is not a creature and takes none of it");
    assert_eq!(state.get_object(equipment).unwrap().damage_marked, 0,
        "nor is an unattached Equipment");
}

/// "This spell costs {1} less to cast for each creature on the battlefield",
/// asked the way the rest of the engine asks — through `cost_to_cast`, not by
/// calling the card's `modified_cost` hook directly. A hook that returns the
/// right number is worth nothing if the cost pipeline never consults it.
#[test]
fn blasphemous_act_cost_reduction() {
    use mtg_engine::engine::{CastMethod, cost_to_cast};
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let card_id = reg.get_id_by_name("Blasphemous Act").unwrap();
    let cost = |s: &mtg_engine::state::GameState| {
        cost_to_cast(s, &reg, card_id, P0, &CastMethod::Normal).mana
    };

    // No creatures: costs {8}{R} = 9 mana.
    assert_eq!(cost(&state).mana_value(), 9, "With 0 creatures it costs its printed {{8}}{{R}}");

    // Non-creature permanents are not creatures and do not reduce it.
    named_permanent(&mut state, &reg, "Kessig Wolf Run", P0);
    named_permanent(&mut state, &reg, "Butcher's Cleaver", P1);
    named_permanent(&mut state, &reg, "Gutter Grime", P0);
    assert_eq!(cost(&state).mana_value(), 9,
        "a land, an Equipment and an enchantment are not creatures");

    // Add 5 creatures: should cost {3}{R} = 4 mana.
    for _ in 0..5 {
        ready_creature(&mut state, P0, 1, 1);
    }
    assert_eq!(cost(&state).mana_value(), 4, "With 5 creatures, Blasphemous Act should cost {{3}}{{R}}");

    // Add 8+ creatures: should cost {R} = 1 mana. Creatures anyone controls
    // count — "each creature on the battlefield".
    for _ in 0..5 {
        ready_creature(&mut state, P1, 1, 1);
    }
    let cost = cost(&state);
    assert_eq!(cost.mana_value(), 1, "With 10 creatures, Blasphemous Act should cost just {{R}}");
    // Ruling: "Blasphemous Act's ability can't reduce the total cost to cast
    // the spell below {R}." The reduction comes off the generic portion only.
    assert_eq!(cost.generic_amount(), 0);
    assert_eq!(cost.colored_requirements().get(&Color::Red).copied().unwrap_or(0), 1,
        "the {{R}} survives any number of creatures");
}

/// Blasphemous Act can be cast cheaply with many creatures.
#[test]
fn blasphemous_act_castable_with_cost_reduction() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Add 8 creatures so cost is just {R}.
    for _ in 0..4 {
        ready_creature(&mut state, P0, 1, 1);
    }
    for _ in 0..4 {
        ready_creature(&mut state, P1, 1, 1);
    }

    // Give P0 just 1 red mana.
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let spell = spell_in_hand(&mut state, &reg, "Blasphemous Act", P0);

    // Should be able to cast with just {R}.
    let has_cast = can_cast(&state, &reg, spell);
    assert!(has_cast, "Blasphemous Act should be castable for {{R}} with 8 creatures on the battlefield");
}

// ── Cackling Counterpart ───────────────────────────────────────

/// Cackling Counterpart creates a token copy of target creature you control.
#[test]
fn cackling_counterpart_creates_token_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let original = named_permanent(&mut state, &reg, "Chapel Geist", P0);

    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(original)]);

    // Should now have 2 Chapel Geists on the battlefield.
    let geists: Vec<_> = state.objects.values()
        .filter(|o| o.zone == Zone::Battlefield && o.name == "Chapel Geist" && o.controller == P0)
        .collect();
    assert_eq!(geists.len(), 2, "Should have original + token copy of Chapel Geist");

    // The token should be a token.
    let token = geists.iter().find(|o| o.is_token).expect("One should be a token");
    assert_eq!(token.power, Some(2));
    assert_eq!(token.toughness, Some(3));
}

/// Ruling: "The token copies exactly what was printed on the original creature
/// and nothing else... It doesn't copy whether that creature is tapped or
/// untapped, whether it has any counters on it or Auras and Equipment attached
/// to it, or any non-copy effects that have changed its power, toughness,
/// types, color, or so on."
///
/// Tree of Redemption is the one card in this set that *writes* a permanent's
/// toughness — "exchange your life total with this creature's toughness" — so
/// it is the only place where the printed value and the object's field come
/// apart. Copying it used to produce the exchanged number.
#[test]
fn cackling_counterpart_copies_the_printed_creature_and_nothing_else() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let tree = named_permanent(&mut state, &reg, "Tree of Redemption", P0);
    state.get_player_mut(P0).life = 4;
    let mut state = activate_only_offered_ability(&state, &reg);
    assert_eq!(state.effective_toughness(tree, &reg), Some(4),
        "test precondition: the exchange made it a 0/4");
    assert_eq!(state.get_player(P0).life, 13, "and gave its controller its printed 13");

    // Counters and a tapped state, none of which are copiable either.
    state.add_counters(tree, CounterType::PlusOnePlusOne, 2);
    state.get_object_mut(tree).unwrap().tapped = true;

    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(tree)]);

    let token = state.objects.values()
        .find(|o| o.is_token && o.zone == Zone::Battlefield)
        .expect("a token copy");
    assert_eq!((token.power, token.toughness), (Some(0), Some(13)),
        "the copy is the printed 0/13, not the exchanged 0/4");
    assert_eq!(state.effective_toughness(token.id, &reg), Some(13),
        "and no +1/+1 counters came across");
    assert!(!token.tapped, "nor did the tapped state");
}

/// Ruling: "If the copied creature is copying something else, then the token
/// enters the battlefield as whatever that creature copied" — and the same
/// logic makes a transformed permanent copiable as the face it is showing
/// (CR 712.8a), not as the front face its card id names.
#[test]
fn cackling_counterpart_copies_the_face_that_is_up() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let ironsmith = named_permanent(&mut state, &reg, "Village Ironsmith", P0);
    mtg_engine::cards::helpers::apply_transform(&mut state, ironsmith, &reg);
    assert_eq!(state.name_of(ironsmith, &reg), "Ironfang", "test precondition: it flipped");

    let spell = castable_spell(&mut state, &reg, "Cackling Counterpart", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(ironsmith)]);

    let token = state.objects.values()
        .find(|o| o.is_token && o.zone == Zone::Battlefield)
        .expect("a token copy");
    assert_eq!(token.name, "Ironfang", "the copy is of the face that is up");
    assert_eq!((token.power, token.toughness), (Some(3), Some(1)),
        "with the back face's printed 3/1, not Village Ironsmith's 1/1");
}

// ── Sever the Bloodline ────────────────────────────────────────

/// Sever the Bloodline exiles target creature and all others with the same name.
#[test]
fn sever_the_bloodline_exiles_all_with_same_name() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create 3 creatures with the same name.
    let z1 = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(z1).unwrap().name = "Zombie Token".into();
    let z2 = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(z2).unwrap().name = "Zombie Token".into();
    let z3 = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(z3).unwrap().name = "Zombie Token".into();
    // And one with a different name.
    let bear = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(bear).unwrap().name = "Bear".into();

    let spell = castable_spell(&mut state, &reg, "Sever the Bloodline", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![mtg_engine::actions::Target::Object(z1)]);

    // All 3 Zombie Tokens should be exiled.
    assert_eq!(state.get_object(z1).unwrap().zone, Zone::Exile, "Target should be exiled");
    assert_eq!(state.get_object(z2).unwrap().zone, Zone::Exile, "Same-name creature should be exiled");
    assert_eq!(state.get_object(z3).unwrap().zone, Zone::Exile, "Own creature with same name should be exiled too");

    // Bear should be unaffected.
    assert_eq!(state.get_object(bear).unwrap().zone, Zone::Battlefield, "Differently-named creature should be unaffected");
}

/// Ruling: "A double-faced creature only has the name of the face that's up.
/// For example, if Village Ironsmith is targeted by Sever the Bloodline,
/// Ironfang wouldn't be exiled."
///
/// The name has to come from the active face. `obj.name` mirrors it today, but
/// the module doc calls that field a display cache; a rules decision reads
/// `name_of`.
#[test]
fn sever_the_bloodline_reads_the_face_that_is_up() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let front = named_permanent(&mut state, &reg, "Village Ironsmith", P1);
    let flipped = named_permanent(&mut state, &reg, "Village Ironsmith", P1);
    mtg_engine::cards::helpers::apply_transform(&mut state, flipped, &reg);
    assert_eq!(state.name_of(flipped, &reg), "Ironfang", "test precondition: it flipped");
    assert_eq!(state.name_of(front, &reg), "Village Ironsmith");

    let spell = castable_spell(&mut state, &reg, "Sever the Bloodline", P0);
    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(front)]);

    assert_eq!(state.get_object(front).unwrap().zone, Zone::Exile,
        "the targeted Village Ironsmith is exiled");
    assert_eq!(state.get_object(flipped).unwrap().zone, Zone::Battlefield,
        "the one showing Ironfang has a different name, so it stays");
}

/// Ruling: "Sever the Bloodline has only one target. Other creatures with the
/// same name will be exiled even if they have hexproof or protection."
#[test]
fn sever_the_bloodline_exiles_same_named_creatures_that_could_not_be_targeted() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    let hexproof = named_permanent(&mut state, &reg, "Walking Corpse", P1);
    grant_keyword(&mut state, hexproof, Keyword::Hexproof);
    assert!(state.has_keyword(hexproof, Keyword::Hexproof, &reg), "test precondition");

    let spell = castable_spell(&mut state, &reg, "Sever the Bloodline", P0);
    let offered = offered_targets(&state, &reg, spell);
    assert!(!offered.contains(&Target::Object(hexproof)),
        "test precondition: the hexproof one cannot be targeted");

    let state = cast_and_resolve(&state, &reg, spell, vec![Target::Object(target)]);
    assert_eq!(state.get_object(hexproof).unwrap().zone, Zone::Exile,
        "it is not a target, so hexproof does not save it");
    assert_eq!(state.get_object(target).unwrap().zone, Zone::Exile);
}

// ── Angelic Overseer ───────────────────────────────────────────

/// "Flying. As long as you control a Human, Angelic Overseer has hexproof and
/// indestructible." Two of its three keywords come and go with the board; the
/// third must not.
#[test]
fn angelic_overseer_hexproof_indestructible_with_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_permanent(&mut state, &reg, "Angelic Overseer", P0);

    // Without a Human: no hexproof or indestructible.
    assert!(!state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should not have hexproof without a Human");
    assert!(!state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should not be indestructible without a Human");

    // Add a Human.
    let human = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // Now should have hexproof and indestructible.
    assert!(state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should have hexproof when you control a Human");
    assert!(state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should be indestructible when you control a Human");

    // Remove the Human.
    state.move_object(human, Zone::Graveyard, &reg);
    assert!(!state.has_keyword(angel, Keyword::Hexproof, &reg),
        "Angelic Overseer should lose hexproof when Human leaves");
    assert!(!state.has_keyword(angel, Keyword::Indestructible, &reg),
        "Angelic Overseer should lose indestructible when Human leaves");

    // Flying is printed, not conditional, so it survives all of that.
    assert!(state.has_keyword(angel, Keyword::Flying, &reg),
        "flying is unconditional — losing the Human must not take it too");
}

/// Angelic Overseer survives destroy effects when indestructible.
#[test]
fn angelic_overseer_survives_destroy_with_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let angel = named_permanent(&mut state, &reg, "Angelic Overseer", P0);
    let _human = named_permanent(&mut state, &reg, "Champion of the Parish", P0);

    // Try to destroy the angel.
    let result = mtg_engine::destruction::try_destroy(&mut state, angel, &reg);
    assert_eq!(result, mtg_engine::destruction::DestroyResult::Indestructible,
        "Angelic Overseer should be indestructible when you control a Human");

    // Angel should still be on the battlefield.
    assert_eq!(state.get_object(angel).unwrap().zone, Zone::Battlefield,
        "Angelic Overseer should survive destruction");
}

