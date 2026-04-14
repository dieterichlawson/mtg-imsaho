//! Regression tests for the equipment "as long as Human" snapshot bug
//! (audit finding "Bug B").
//!
//! Background: Silver-Inlaid Dagger says "Equipped creature gets +2/+0. As
//! long as equipped creature is a Human, it gets an additional +1/+0." and
//! Butcher's Cleaver says "+3/+0; as long as equipped creature is a Human,
//! it has lifelink." Both bonuses are *continuous* — they should follow the
//! current type of the creature, not be frozen at equip time.
//!
//! The previous implementations took a snapshot of `is_human` in their
//! `update_effects` helper at the moment of equipping, then wrote a fixed
//! `instance_continuous_effects` vector. So if a Human Werewolf transformed
//! into its non-Human back face (via Moonmist or via the no-spells-last-turn
//! upkeep trigger), the +1/+0 / lifelink stayed put. The fix is to express
//! the bonus as a `ContinuousEffect::ConditionalModifyPT` /
//! `ContinuousEffect::ConditionalKeyword` (the same shape Bonds of Faith
//! uses), which gets re-evaluated every time we compute effective P/T or
//! check keywords.
//!
//! These tests pin the fix in both directions: bonus appears when the
//! creature is Human, drops when it isn't, and updates live when the
//! type changes.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::GameState;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

fn equipment(state: &mut GameState, reg: &CardRegistry, name: &str, owner: mtg_engine::ids::PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown card {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.is_equipment = true;
    id
}

/// Equip the equipment to the creature, paying mana from the pool.
fn equip(state: &GameState, reg: &CardRegistry, equipment_id: ObjectId, creature_id: ObjectId, equip_cost: i32) -> GameState {
    let mut s = state.clone();
    s.get_player_mut(P0).mana_pool.add(ManaType::Colorless, equip_cost as u32);
    let legal = engine::legal_actions(&s, reg);
    let action = legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == equipment_id && targets == &[Target::Object(creature_id)]))
        .cloned()
        .expect("equip action should be available");
    engine::submit_action(&s, &action, reg)
}

// ════════════════════════════════════════════════════════════════════
// Silver-Inlaid Dagger — +2/+0, +1/+0 if Human (continuous)
// ════════════════════════════════════════════════════════════════════

#[test]
fn silver_inlaid_dagger_bonus_when_attached_to_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // Avacyn's Pilgrim is a Human Monk.
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let state = equip(&state, &reg, dagger, pilgrim, 2);
    // 1/1 + 2 (base) + 1 (Human bonus) = 4/1.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(4));
    assert_eq!(state.effective_toughness(pilgrim, &reg), Some(1));
}

#[test]
fn silver_inlaid_dagger_no_bonus_when_attached_to_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // Walking Corpse is a Zombie, not a Human.
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let state = equip(&state, &reg, dagger, zombie, 2);
    // 2/2 + 2 (base) + 0 (no Human bonus) = 4/2.
    assert_eq!(state.effective_power(zombie, &reg), Some(4));
    assert_eq!(state.effective_toughness(zombie, &reg), Some(2));
}

#[test]
fn silver_inlaid_dagger_drops_human_bonus_when_creature_loses_human_subtype() {
    // The regression case: equip the dagger to a Human creature, then
    // change the creature's subtypes to remove "Human", and verify the
    // +1/+0 bonus disappears immediately.
    //
    // We simulate the type change directly by mutating obj.subtypes — this
    // is exactly what a transform effect would do, and it lets the test
    // run without depending on a specific transform card's mechanics.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let mut state = equip(&state, &reg, dagger, pilgrim, 2);

    // Sanity: Human bonus is active.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(4),
        "Human bonus should be active right after equipping a Human");

    // Now strip the Human subtype (as a transform effect would).
    {
        let obj = state.get_object_mut(pilgrim).unwrap();
        obj.subtypes = vec!["Werewolf".into()]; // non-Human
    }

    // Bonus should drop immediately because the effect is conditional, not snapshot.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(3),
        "Human bonus should drop the moment the creature stops being a Human");
    assert_eq!(state.effective_toughness(pilgrim, &reg), Some(1));
}

#[test]
fn silver_inlaid_dagger_gains_human_bonus_when_creature_becomes_human() {
    // Inverse case: equip to a non-Human, then add the Human subtype,
    // and verify the +1/+0 bonus appears.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let mut state = equip(&state, &reg, dagger, zombie, 2);

    // Sanity: no Human bonus.
    assert_eq!(state.effective_power(zombie, &reg), Some(4),
        "non-Human zombie should be 2 + 2 = 4 power");

    // Now add the Human subtype.
    {
        let obj = state.get_object_mut(zombie).unwrap();
        // Walking Corpse's base subtypes are Zombie. Add Human alongside.
        obj.subtypes = vec!["Zombie".into(), "Human".into()];
    }

    assert_eq!(state.effective_power(zombie, &reg), Some(5),
        "Human bonus should appear once the creature becomes a Human (+1 from base 4)");
}

// ════════════════════════════════════════════════════════════════════
// Butcher's Cleaver — +3/+0, lifelink if Human (continuous)
// ════════════════════════════════════════════════════════════════════

#[test]
fn butchers_cleaver_lifelink_when_attached_to_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let cleaver = equipment(&mut state, &reg, "Butcher's Cleaver", P0);
    let state = equip(&state, &reg, cleaver, pilgrim, 3);
    assert!(state.has_keyword(pilgrim, Keyword::Lifelink, &reg),
        "Human-equipped Cleaver should grant lifelink");
}

#[test]
fn butchers_cleaver_no_lifelink_when_attached_to_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let cleaver = equipment(&mut state, &reg, "Butcher's Cleaver", P0);
    let state = equip(&state, &reg, cleaver, zombie, 3);
    assert!(!state.has_keyword(zombie, Keyword::Lifelink, &reg),
        "non-Human-equipped Cleaver should NOT grant lifelink");
}

#[test]
fn butchers_cleaver_drops_lifelink_when_creature_loses_human_subtype() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let cleaver = equipment(&mut state, &reg, "Butcher's Cleaver", P0);
    let mut state = equip(&state, &reg, cleaver, pilgrim, 3);

    assert!(state.has_keyword(pilgrim, Keyword::Lifelink, &reg),
        "lifelink should be active on the Human");

    // Strip Human subtype (as a transform effect would).
    {
        let obj = state.get_object_mut(pilgrim).unwrap();
        obj.subtypes = vec!["Werewolf".into()];
    }

    assert!(!state.has_keyword(pilgrim, Keyword::Lifelink, &reg),
        "lifelink should drop the moment the creature stops being a Human");

    // The +3/+0 bonus is unconditional and should still apply.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(4),
        "unconditional +3/+0 bonus should still apply (1 + 3 = 4 power)");
}

#[test]
fn butchers_cleaver_gains_lifelink_when_creature_becomes_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let cleaver = equipment(&mut state, &reg, "Butcher's Cleaver", P0);
    let mut state = equip(&state, &reg, cleaver, zombie, 3);

    assert!(!state.has_keyword(zombie, Keyword::Lifelink, &reg));

    // Add Human subtype.
    {
        let obj = state.get_object_mut(zombie).unwrap();
        obj.subtypes = vec!["Zombie".into(), "Human".into()];
    }

    assert!(state.has_keyword(zombie, Keyword::Lifelink, &reg),
        "lifelink should appear once the creature becomes a Human");
}

// ════════════════════════════════════════════════════════════════════
// Reattachment: when the dagger/cleaver moves to a different creature,
// the conditional bonus should follow the new attachment, not the old.
// ════════════════════════════════════════════════════════════════════

#[test]
fn silver_inlaid_dagger_human_bonus_follows_reattachment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0); // Human
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);    // non-Human
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);

    // First equip → pilgrim (Human, gets +3/+0 total).
    let state = equip(&state, &reg, dagger, pilgrim, 2);
    assert_eq!(state.effective_power(pilgrim, &reg), Some(4),
        "pilgrim should be 4 power with the Human bonus");
    // Zombie should be unaffected.
    assert_eq!(state.effective_power(zombie, &reg), Some(2));

    // Reattach to zombie → only the unconditional +2/+0 should apply.
    let state = equip(&state, &reg, dagger, zombie, 2);
    // Pilgrim no longer wears the dagger, back to base.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(1),
        "pilgrim should be back to 1 power once the dagger moves off");
    // Zombie has the unconditional +2/+0 only — 2 + 2 = 4, no Human bonus.
    assert_eq!(state.effective_power(zombie, &reg), Some(4));
}

#[test]
fn butchers_cleaver_lifelink_follows_reattachment() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0); // Human
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);    // non-Human
    let cleaver = equipment(&mut state, &reg, "Butcher's Cleaver", P0);

    let state = equip(&state, &reg, cleaver, pilgrim, 3);
    assert!(state.has_keyword(pilgrim, Keyword::Lifelink, &reg));
    assert!(!state.has_keyword(zombie, Keyword::Lifelink, &reg));

    let state = equip(&state, &reg, cleaver, zombie, 3);
    assert!(!state.has_keyword(pilgrim, Keyword::Lifelink, &reg),
        "lifelink should leave pilgrim once the cleaver is reattached");
    assert!(!state.has_keyword(zombie, Keyword::Lifelink, &reg),
        "zombie should not gain lifelink (not a Human)");
}

// ════════════════════════════════════════════════════════════════════
// Sharpened Pitchfork — first strike, +1/+1 if Human (continuous)
//
// Same shape of bug as Silver-Inlaid Dagger / Butcher's Cleaver: the
// previous implementation snapshotted `is_human` at equip time via an
// `update_effects` helper. Fixed in commit `5294721`-ish by switching to
// ContinuousEffect::ConditionalModifyPT.
// ════════════════════════════════════════════════════════════════════

#[test]
fn sharpened_pitchfork_bonus_when_attached_to_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0); // Human
    let fork = equipment(&mut state, &reg, "Sharpened Pitchfork", P0);
    let state = equip(&state, &reg, fork, pilgrim, 1);
    // 1/1 + 1/+1 (Human bonus) = 2/2 with first strike.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(2));
    assert_eq!(state.effective_toughness(pilgrim, &reg), Some(2));
    assert!(state.has_keyword(pilgrim, Keyword::FirstStrike, &reg));
}

#[test]
fn sharpened_pitchfork_no_bonus_when_attached_to_non_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0); // Zombie
    let fork = equipment(&mut state, &reg, "Sharpened Pitchfork", P0);
    let state = equip(&state, &reg, fork, zombie, 1);
    // 2/2 + 0/0 (non-Human) = 2/2 with first strike.
    assert_eq!(state.effective_power(zombie, &reg), Some(2));
    assert_eq!(state.effective_toughness(zombie, &reg), Some(2));
    assert!(state.has_keyword(zombie, Keyword::FirstStrike, &reg));
}

#[test]
fn sharpened_pitchfork_drops_human_bonus_when_creature_loses_human_subtype() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let fork = equipment(&mut state, &reg, "Sharpened Pitchfork", P0);
    let mut state = equip(&state, &reg, fork, pilgrim, 1);

    assert_eq!(state.effective_power(pilgrim, &reg), Some(2));

    // Strip the Human subtype.
    {
        let obj = state.get_object_mut(pilgrim).unwrap();
        obj.subtypes = vec!["Werewolf".into()];
    }

    // Bonus should drop immediately.
    assert_eq!(state.effective_power(pilgrim, &reg), Some(1));
    assert_eq!(state.effective_toughness(pilgrim, &reg), Some(1));
    // First strike is unconditional, should still be there.
    assert!(state.has_keyword(pilgrim, Keyword::FirstStrike, &reg));
}

#[test]
fn sharpened_pitchfork_gains_human_bonus_when_creature_becomes_human() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let zombie = named_creature(&mut state, &reg, "Walking Corpse", P0);
    let fork = equipment(&mut state, &reg, "Sharpened Pitchfork", P0);
    let mut state = equip(&state, &reg, fork, zombie, 1);

    // Sanity: no Human bonus on a non-Human.
    assert_eq!(state.effective_power(zombie, &reg), Some(2));

    // Add Human subtype.
    {
        let obj = state.get_object_mut(zombie).unwrap();
        obj.subtypes = vec!["Zombie".into(), "Human".into()];
    }

    // Bonus should appear immediately.
    assert_eq!(state.effective_power(zombie, &reg), Some(3));
    assert_eq!(state.effective_toughness(zombie, &reg), Some(3));
}
