//! Tests for edge cases and coverage gaps identified in the code review.
//!
//! These tests cover scenarios that were previously untested:
//! - Indestructible interactions with zero toughness and sacrifice
//! - Simultaneous creature death in combat
//! - Aura falling off when enchanted creature leaves the battlefield

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::combat;
use mtg_engine::destruction;
use mtg_engine::ids::CardId;
use mtg_engine::sba::{check_state_based_actions, check_state_based_actions_with_registry};
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

// ── Indestructible + zero toughness ────────────────────────────────

/// Rule 704.5f: A creature with 0 toughness dies even if indestructible.
/// Indestructible only prevents destruction (rule 702.12b), and the
/// 0-toughness SBA is not destruction.
#[test]
fn indestructible_creature_with_zero_toughness_still_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = state.create_object(CardId(99), P0, Zone::Battlefield, Some(5), Some(0));
    state.get_object_mut(creature).unwrap().summoning_sick = false;
    // Grant indestructible via keyword.
    state.until_end_of_turn_keywords.push(
        mtg_engine::state::UntilEndOfTurnKeyword {
            target: creature,
            keyword: Keyword::Indestructible,
        },
    );

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(
        state.get_object(creature).unwrap().zone,
        Zone::Graveyard,
        "Indestructible creature with 0 toughness should still die (rule 704.5f)"
    );
}

/// Indestructible creature survives lethal damage — the SBA for lethal
/// damage uses try_destroy, which is blocked by indestructible.
#[test]
fn indestructible_creature_survives_lethal_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 3, 3);
    state.get_object_mut(creature).unwrap().damage_marked = 10;
    state.until_end_of_turn_keywords.push(
        mtg_engine::state::UntilEndOfTurnKeyword {
            target: creature,
            keyword: Keyword::Indestructible,
        },
    );

    check_state_based_actions_with_registry(&mut state, Some(&reg));

    assert_eq!(
        state.get_object(creature).unwrap().zone,
        Zone::Battlefield,
        "Indestructible creature should survive lethal damage"
    );
}

/// Sacrifice bypasses indestructible — sacrifice is not destruction.
#[test]
fn sacrifice_bypasses_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 5, 5);
    state.until_end_of_turn_keywords.push(
        mtg_engine::state::UntilEndOfTurnKeyword {
            target: creature,
            keyword: Keyword::Indestructible,
        },
    );
    assert!(state.has_keyword(creature, Keyword::Indestructible, &reg));

    let result = destruction::sacrifice(&mut state, creature);
    assert!(result, "Sacrifice should succeed even on indestructible creature");
    assert_eq!(
        state.get_object(creature).unwrap().zone,
        Zone::Graveyard,
        "Sacrificed indestructible creature should be in graveyard"
    );
}

/// try_destroy does NOT destroy an indestructible creature.
#[test]
fn try_destroy_blocked_by_indestructible() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 5, 5);
    state.until_end_of_turn_keywords.push(
        mtg_engine::state::UntilEndOfTurnKeyword {
            target: creature,
            keyword: Keyword::Indestructible,
        },
    );

    let result = destruction::try_destroy(&mut state, creature, &reg);
    assert_eq!(result, destruction::DestroyResult::Indestructible);
    assert_eq!(
        state.get_object(creature).unwrap().zone,
        Zone::Battlefield,
        "try_destroy should not move indestructible creature"
    );
}

// ── Simultaneous death in combat ───────────────────────────────────

/// When two creatures deal lethal damage to each other in combat,
/// both should die simultaneously when SBAs are checked.
#[test]
fn mutually_lethal_combat_both_die() {
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 3, 3);
    let blocker = ready_creature(&mut state, P1, 3, 3);

    combat::declare_attackers(&mut state, &[(attacker, P1)]);
    combat::declare_blockers(&mut state, &[(blocker, attacker)]);
    combat::deal_combat_damage(&mut state, &registry());

    assert_eq!(state.get_object(attacker).unwrap().damage_marked, 3);
    assert_eq!(state.get_object(blocker).unwrap().damage_marked, 3);

    check_state_based_actions(&mut state);

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
    state.move_object(creature, Zone::Graveyard);

    // SBA should clean up the unattached aura.
    check_state_based_actions(&mut state);

    assert_eq!(
        state.get_object(aura).unwrap().zone,
        Zone::Graveyard,
        "Aura should fall off and go to graveyard when enchanted creature dies (rule 704.5m)"
    );
}

/// Aura stays on the battlefield as long as its enchanted creature is alive.
#[test]
fn aura_stays_while_creature_alive() {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = ready_creature(&mut state, P0, 3, 3);

    let aura = state.create_object(CardId(50), P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().attached_to = Some(creature);

    check_state_based_actions(&mut state);

    assert_eq!(
        state.get_object(aura).unwrap().zone,
        Zone::Battlefield,
        "Aura should stay on battlefield while its target is alive"
    );
}

// ── Token death triggers ───────────────────────────────────────────

/// Tokens go to the graveyard when they die (triggering "dies" events),
/// then cease to exist as a separate SBA.
#[test]
fn token_dies_goes_to_graveyard_then_ceases_to_exist() {
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Create a token creature.
    let token = state.create_token("Zombie", P0, 2, 2, vec![], vec![CardType::Creature], vec![]);
    state.get_object_mut(token).unwrap().summoning_sick = false;

    // Deal lethal damage.
    state.get_object_mut(token).unwrap().damage_marked = 3;

    // SBA should: 1) kill the token (move to graveyard), 2) cease to exist (remove from objects).
    check_state_based_actions(&mut state);

    // Token should be completely gone — removed from the objects map.
    assert!(
        state.get_object(token).is_none(),
        "Token should cease to exist after dying and being cleaned up by SBAs"
    );
}

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
    check_state_based_actions(&mut state);
    assert_eq!(state.get_object(creature).unwrap().zone, Zone::Battlefield);
}
