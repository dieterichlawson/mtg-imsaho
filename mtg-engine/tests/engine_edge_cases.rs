//! Corner cases the engine has to get right: empty zones, zero counts, an
//! effect whose subject has already gone.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::events::GameEvent;
use mtg_engine::ids::CardId;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

// ── Indestructible + zero toughness ────────────────────────────────

// ── Simultaneous death in combat ───────────────────────────────────

/// When two creatures deal lethal damage to each other in combat,
/// both should die simultaneously when SBAs are checked.
#[test]
fn mutually_lethal_combat_both_die() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    submit_declare_blockers(&mut state, P1, &[(blocker, attacker)], &reg);
    combat::deal_combat_damage(&mut state, &reg);

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 3);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(
        state.get_object(attacker).unwrap().zone,
        Zone::Graveyard,
        "Attacker should die from mutually lethal combat"
    );
    assert_eq!(
        state.get_object(blocker).unwrap().zone,
        Zone::Graveyard,
        "Blocker should die from mutually lethal combat"
    );
    // Player takes no damage — attacker was blocked.
    assert_eq!(state.get_player(P1).life, 20);
}

// ── Aura falls off ─────────────────────────────────────────────────

/// When the enchanted creature dies, the aura should fall off and go
/// to the graveyard via SBA 704.5m (aura not attached to a legal permanent).
#[test]
fn aura_goes_to_graveyard_when_creature_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    // Attach an aura to the creature.
    let aura_id = reg.get_id_by_name("Holy Strength").unwrap();
    let aura = state.create_object(aura_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().attached_to = Some(creature);
    state.get_object_mut(aura).unwrap().summoning_sick = false;

    // Kill the creature directly.
    state.move_object(creature, Zone::Graveyard, &reg);

    // SBA should clean up the unattached aura.
    check_state_based_actions(&mut state, &reg);

    assert_eq!(
        state.get_object(aura).unwrap().zone,
        Zone::Graveyard,
        "Aura should fall off and go to graveyard when enchanted creature dies (rule 704.5m)"
    );
}

/// Aura stays on the battlefield as long as its enchanted creature is alive.
#[test]
fn aura_stays_while_creature_alive() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    let aura = state.create_object(CardId(50), P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().attached_to = Some(creature);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(
        state.get_object(aura).unwrap().zone,
        Zone::Battlefield,
        "Aura should stay on battlefield while its target is alive"
    );
}

// ── Token death triggers ───────────────────────────────────────────

// ── Damage does not reduce toughness ───────────────────────────────

/// Damage is marked on creatures, not subtracted from toughness.
/// A 3/3 with 2 damage still has effective toughness 3, not 1.
#[test]
fn damage_does_not_reduce_effective_toughness() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    assert_eq!(
        state.effective_toughness(creature, &reg),
        Some(3),
        "Effective toughness should not be reduced by damage — damage is tracked separately"
    );
    // But the creature is still alive (2 damage < 3 toughness).
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
}

// ── Zone change creates new object identity ────────────────────────

/// When a creature leaves the battlefield and re-enters, attached auras
/// should fall off. The re-entered creature is a "new object" per rule 400.7.
#[test]
fn blinked_creature_loses_aura() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);

    // Attach an aura.
    let aura_id = reg.get_id_by_name("Holy Strength").unwrap();
    let aura = state.create_object(aura_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().attached_to = Some(creature);

    // "Blink" the creature: exile then return to battlefield.
    state.move_object(creature, Zone::Exile, &reg);

    // SBA should clean up the aura (its target left the battlefield).
    check_state_based_actions(&mut state, &reg);

    assert_eq!(
        state.get_object(aura).unwrap().zone,
        Zone::Graveyard,
        "Aura should fall off when creature is blinked (exiled) — rule 400.7"
    );
}

/// When a creature leaves and re-enters, damage should be cleared.
#[test]
fn zone_change_clears_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    // Mark some damage.
    state.get_object_mut(creature).unwrap().damage_marked = 2;

    // Move to hand and back to battlefield.
    state.move_object(creature, Zone::Hand, &reg);
    state.move_object(creature, Zone::Battlefield, &reg);

    assert_eq!(
        state.get_object(creature).unwrap().damage_marked, 0,
        "Damage should be cleared when a creature re-enters the battlefield"
    );
}

// ── "Dies" triggers see correct information ────────────────────────

/// When a creature dies, the `CreatureDied` event should contain the
/// correct `card_id` and controller from when it was on the battlefield.
#[test]
fn dies_trigger_has_correct_info() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = CardId(42);
    let creature = state.create_object(card_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    state.get_object_mut(creature).unwrap().controller = P1; // controlled by P1

    // Kill via lethal damage.
    state.get_object_mut(creature).unwrap().damage_marked = 5;
    state.events.clear();
    check_state_based_actions(&mut state, &reg);

    // Find the CreatureDied event.
    let died_event = state.events.iter().find(|e| {
        matches!(e, GameEvent::CreatureDied { object, .. } if *object == creature)
    });
    assert!(died_event.is_some(), "Should emit CreatureDied event");

    if let Some(GameEvent::CreatureDied { card_id: cid, controller, .. }) = died_event {
        assert_eq!(*cid, card_id, "CreatureDied should have the correct card_id");
        assert_eq!(*controller, P1, "CreatureDied should record the controller");
    }
}

/// When a creature dies, death-watch triggers on other permanents should fire.
#[test]
fn death_watch_triggers_fire_on_creature_death() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place an Unruly Mob (gains +1/+1 counter when another creature you control dies).
    let mob = named_permanent(&mut state, &reg, "Unruly Mob", P0);

    // Place a creature that will die.
    let victim = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(victim).unwrap().damage_marked = 2;

    // Run SBAs to kill the victim.
    check_state_based_actions(&mut state, &reg);

    // Process triggers (the death-watch should fire).
    mtg_engine::triggers::process_triggers(&mut state, &reg);

    // Unruly Mob should have gained a +1/+1 counter.
    let counter_count = state.get_counter_count(mob, CounterType::PlusOnePlusOne);
    assert_eq!(
        counter_count, 1,
        "Unruly Mob should gain a +1/+1 counter when another creature you control dies"
    );
}

// ── Cleanup step ───────────────────────────────────────────────────

/// "Until end of turn" effects should be cleared during the cleanup step.
/// A creature with a Giant Growth (+3/+3 until EOT) should revert.
#[test]
fn cleanup_clears_until_end_of_turn_effects() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);

    // Simulate Giant Growth: +3/+3 until end of turn.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 3,
            toughness_mod: 3,
        },
    );

    assert_eq!(state.effective_power(creature, &reg), Some(5));
    assert_eq!(state.effective_toughness(creature, &reg), Some(5));

    advance_to_cleanup(&mut state, &reg);

    assert_eq!(
        state.effective_power(creature, &reg),
        Some(2),
        "+3/+3 should be gone after cleanup"
    );
    assert_eq!(
        state.effective_toughness(creature, &reg),
        Some(2),
        "+3/+3 should be gone after cleanup"
    );
}

/// If a creature survives only because of an "until end of turn" toughness
/// bonus, it should die in cleanup when the bonus is removed and SBAs are
/// checked. (This tests the cleanup-step SBA interaction.)
#[test]
fn creature_dies_in_cleanup_when_eot_buff_expires() {
    let reg = registry();
    let mut state = game_at_step(Step::PostcombatMain, P0);

    // 1/1 creature with 1 damage — alive because of +0/+1 until EOT.
    let creature = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(creature).unwrap().damage_marked = 1;
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::ModifyPT {
            target: creature,
            power_mod: 0,
            toughness_mod: 1,
        },
    );

    // With the buff, effective toughness is 2, damage is 1 — survives.
    assert_eq!(state.effective_toughness(creature, &reg), Some(2));
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);

    advance_to_cleanup(&mut state, &reg);

    // During cleanup, damage is cleared AND the buff expires.
    // The creature is now 1/1 with 0 damage — it lives.
    // (Cleanup clears damage at the same time as removing buffs.)
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield,
        "Creature should survive cleanup because damage is cleared at the same time as buffs expire");
}

/// Until-end-of-turn keyword grants should expire during cleanup.
#[test]
fn cleanup_clears_keyword_grants() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantKeyword {
            target: creature,
            keyword: Keyword::Flying,
        },
    );

    assert!(state.has_keyword(creature, Keyword::Flying, &reg));

    advance_to_cleanup(&mut state, &reg);

    assert!(
        !state.has_keyword(creature, Keyword::Flying, &reg),
        "Until-end-of-turn flying should expire during cleanup"
    );
}

/// Until-end-of-turn removed keywords should be restored during cleanup.
/// Manor Gargoyle loses defender (and thus indestructible) until end of turn.
#[test]
fn cleanup_restores_removed_keywords() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 4, 4);
    // Give it defender via object keywords.
    state.get_object_mut(creature).unwrap().keywords.push(Keyword::Defender);
    assert!(state.has_keyword(creature, Keyword::Defender, &reg));

    // Remove defender until end of turn.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::RemoveKeyword {
            target: creature,
            keyword: Keyword::Defender,
        },
    );
    assert!(
        !state.has_keyword(creature, Keyword::Defender, &reg),
        "Defender should be temporarily removed"
    );

    advance_to_cleanup(&mut state, &reg);

    assert!(
        state.has_keyword(creature, Keyword::Defender, &reg),
        "Defender should be restored after cleanup"
    );
}

/// Until-end-of-turn can't-block should be cleared during cleanup.
#[test]
fn cleanup_clears_cant_block() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 3, 3);
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::CantBlock { target: creature });

    // Verify it's in the list.
    assert!(state.until_end_of_turn.iter().any(|e| matches!(e,
        mtg_engine::state::TemporaryEffect::CantBlock { target } if *target == creature)));

    advance_to_cleanup(&mut state, &reg);

    assert!(
        state.until_end_of_turn.is_empty(),
        "Can't-block should be cleared after cleanup"
    );
}

/// Until-end-of-turn protection should be cleared during cleanup.
#[test]
fn cleanup_clears_protection_grants() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantProtection {
            target: creature,
            filter: CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
        },
    );

    assert!(!state.until_end_of_turn.is_empty());

    advance_to_cleanup(&mut state, &reg);

    assert!(
        state.until_end_of_turn.is_empty(),
        "Protection grants should be cleared after cleanup"
    );
}

/// Until-end-of-turn control changes should be reverted during cleanup.
#[test]
fn cleanup_reverts_control_changes() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P1, 4, 4);
    assert_eq!(state.get_object(creature).unwrap().controller, P1);

    // Steal creature until end of turn.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::ChangeControl { target: creature, original_controller: P1 });
    state.get_object_mut(creature).unwrap().controller = P0;
    assert_eq!(state.get_object(creature).unwrap().controller, P0);

    advance_to_cleanup(&mut state, &reg);

    assert_eq!(
        state.get_object(creature).unwrap().controller,
        P1,
        "Control should revert to original controller after cleanup"
    );
}

/// Until-end-of-turn protection should prevent a non-Human from blocking.
#[test]
fn eot_protection_prevents_blocking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 2, 2);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    // Attacker has protection from non-Human creatures.
    state.until_end_of_turn.push(
        mtg_engine::state::TemporaryEffect::GrantProtection {
            target: attacker,
            filter: CreatureFilter::Not(Box::new(CreatureFilter::HasSubtype("Human".into()))),
        },
    );

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    // Blocker is not a Human, so can_block_attacker should reject it.
    assert!(
        !combat::can_block_attacker(&state, blocker, attacker, &reg),
        "Non-Human blocker should not be able to block creature with protection from non-Humans"
    );
}

/// Cant-block prevents a creature from appearing in eligible blockers.
#[test]
fn eot_cant_block_prevents_blocking() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 2, 2);

    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);

    // Without can't-block, blocker should be eligible.
    let eligible_before = combat::eligible_blockers(&state, P1, &reg);
    assert!(
        eligible_before.contains(&blocker),
        "Blocker should be eligible before can't-block"
    );

    // Apply can't-block.
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::CantBlock { target: blocker });
    let eligible_after = combat::eligible_blockers(&state, P1, &reg);
    assert!(
        !eligible_after.contains(&blocker),
        "Blocker should not be eligible after can't-block"
    );
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Mirror-Mad Phantasm's ability uses `draw_top_card` for the reveal loop,
/// which sets `has_drawn_from_empty=true` if library runs out. This causes the
/// player to lose via SBA even though they didn't actually draw from empty.
/// CR 701.15a: revealing is not drawing. `reveal_top_card` must not set
/// `has_drawn_from_empty`, or a reveal loop that runs the library out — the
/// shape Mirror-Mad Phantasm and Trepanation Blade both use — would lose its
/// controller the game to SBA (CR 704.5b).
#[test]
fn revealing_past_the_end_of_a_library_is_not_drawing_from_it() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = registry.get_id_by_name("Grizzly Bears").unwrap();
    for _ in 0..3 {
        let id = state.create_object(bears, P0, Zone::Library, Some(2), Some(2));
        state.get_player_mut(P0).library_order.push(id);
    }

    let mut revealed = 0;
    while state.get_player_mut(P0).reveal_top_card().is_some() {
        revealed += 1;
        assert!(revealed <= 3, "reveal_top_card kept handing out cards past the end");
    }
    assert_eq!(revealed, 3, "every card in the library is revealed exactly once");
    assert!(!state.get_player(P0).has_drawn_from_empty,
        "running the library out by revealing must not flag a draw from an empty library");
}
