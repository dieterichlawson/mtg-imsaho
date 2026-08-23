//! Regression tests for the unified damage pipeline.
//!
//! Historically each damage path hand-rolled its own subset of checks:
//! fight damage skipped Unbreathing Horde's prevent-and-remove-counter
//! replacement, and noncombat damage skipped the deathtouch flag and
//! player-damage lifelink. All damage now flows through
//! `mtg_engine::damage::deal_damage`.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::damage::{deal_damage, DamageKind};
use mtg_engine::events::DamageTarget;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

/// Fight damage is noncombat damage and must respect Unbreathing Horde's
/// "prevent that damage, remove a +1/+1 counter" replacement (CR 614.1a).
#[test]
fn fight_damage_respects_prevent_damage_remove_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let horde = named_creature(&mut state, &reg, "Unbreathing Horde", P0);
    state.add_counters(horde, CounterType::PlusOnePlusOne, 3);
    let bear = ready_creature(&mut state, P1, 2, 2);

    mtg_engine::combat::fight(&mut state, horde, bear, &reg);

    let horde_obj = state.get_object(horde).unwrap();
    assert_eq!(horde_obj.damage_marked, 0,
        "Unbreathing Horde's replacement must prevent fight damage");
    assert_eq!(counters_of(&state, horde, CounterType::PlusOnePlusOne), 2,
        "preventing the damage must remove exactly one +1/+1 counter");
    // The bear still takes the Horde's damage.
    assert!(state.get_object(bear).unwrap().damage_marked > 0,
        "the other fighter still takes damage normally");
}

/// Noncombat damage from a deathtouch source must set the deathtouch flag
/// so SBAs destroy the damaged creature (CR 704.5h applies to ANY damage).
#[test]
fn noncombat_deathtouch_damage_sets_flag() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let source = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(source).unwrap().keywords.push(Keyword::Deathtouch);
    let victim = ready_creature(&mut state, P1, 4, 4);

    deal_damage(&mut state, source, DamageTarget::Object(victim), 1, DamageKind::NonCombat, &reg);

    let v = state.get_object(victim).unwrap();
    assert_eq!(v.damage_marked, 1);
    assert!(v.dealt_deathtouch_damage,
        "noncombat damage from a deathtouch source must set dealt_deathtouch_damage");
}

/// Noncombat damage to a player from a lifelink source must gain its
/// controller life (CR 702.15 applies to ANY damage, not just combat).
#[test]
fn noncombat_damage_to_player_triggers_lifelink() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let source = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(source).unwrap().keywords.push(Keyword::Lifelink);
    let p0_life = state.get_player(P0).life;
    let p1_life = state.get_player(P1).life;

    deal_damage(&mut state, source, DamageTarget::Player(P1), 2, DamageKind::NonCombat, &reg);

    assert_eq!(state.get_player(P1).life, p1_life - 2);
    assert_eq!(state.get_player(P0).life, p0_life + 2,
        "lifelink must apply to noncombat damage dealt to a player");
}
