//! Regression tests for the equipment auto-tap bug (audit finding "Bug A").
//!
//! Background: in earlier versions, `legal_actions` checked activated-ability mana
//! costs only against the player's *current floating mana pool* via `mana::can_pay`.
//! It did not call `compute_autotap` against untapped lands the way `CastSpell` did.
//! As a consequence, equipment with `Equip {N}` and any other activated ability with
//! a pure mana cost was *never* offered as a legal action in main phase 1, because
//! the mana pool is normally empty there. In a 120k-line tournament log we observed
//! the LLM player attempting to "equip the dagger before attacking" — but the engine
//! had silently auto-passed through main phase without ever surfacing the equip option.
//!
//! These tests pin down the fix: when a player controls equipment, a target creature,
//! and enough untapped lands of the right colors, the equip ability MUST appear in
//! `legal_actions` AND submitting it MUST tap the lands and attach the equipment.
//!
//! We test every mana-cost equipment in the ISD set, plus a few non-equipment
//! activated abilities (Elder of Laurels) to confirm the fix is general.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

/// Place an unattached equipment of the given name on the battlefield.
fn equipment(state: &mut GameState, reg: &CardRegistry, name: &str, owner: PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown card {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.is_equipment = true;
    id
}

/// Place an untapped basic land of the given name on the battlefield.
fn untapped_land(state: &mut GameState, reg: &CardRegistry, name: &str, owner: PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown land {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.tapped = false;
    obj.summoning_sick = false;
    id
}

/// Place N untapped basic lands of the given name. Returns the IDs.
fn untapped_lands(state: &mut GameState, reg: &CardRegistry, name: &str, owner: PlayerId, n: usize) -> Vec<ObjectId> {
    (0..n).map(|_| untapped_land(state, reg, name, owner)).collect()
}

/// Find the (single) "Activate <equipment>" action that targets the given creature.
fn find_equip_action(state: &GameState, reg: &CardRegistry, equipment_id: ObjectId, target: ObjectId) -> Action {
    let legal = engine::legal_actions(state, reg);
    legal.actions.iter()
        .find(|a| matches!(a, Action::ActivateAbility { object_id, targets, .. }
            if *object_id == equipment_id && targets == &[Target::Object(target)]))
        .cloned()
        .unwrap_or_else(|| {
            // Build a friendly diagnostic.
            let actions: Vec<String> = legal.actions.iter().map(|a| format!("{a:?}")).collect();
            panic!(
                "no Activate action found for equipment {} -> creature {}\nlegal actions:\n  {}",
                equipment_id.0,
                target.0,
                actions.join("\n  "),
            );
        })
}

/// Set up a baseline: empty mana pool, equipment + creature + N untapped Forests.
/// Returns (state, `equipment_id`, `creature_id`, lands).
fn setup_with_lands(
    reg: &CardRegistry,
    equipment_name: &str,
    creature_name: &str,
    n_forests: usize,
) -> (GameState, ObjectId, ObjectId, Vec<ObjectId>) {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let creature = named_creature(&mut state, reg, creature_name, P0);
    let eq = equipment(&mut state, reg, equipment_name, P0);
    let lands = untapped_lands(&mut state, reg, "Forest", P0, n_forests);
    // Mana pool starts empty — that is the entire point of this test.
    assert!(state.get_player(P0).mana_pool.is_empty(), "mana pool must be empty for autotap test");
    (state, eq, creature, lands)
}

// ════════════════════════════════════════════════════════════════════
// Silver-Inlaid Dagger — Equip {2}
// ════════════════════════════════════════════════════════════════════

#[test]
fn silver_inlaid_dagger_equip_offered_when_only_lands_untapped() {
    // The exact scenario from the audit log: player has the dagger, a creature,
    // and 2 untapped Forests. Mana pool empty. We use Grizzly Bears (a vanilla
    // creature with no mana ability) so the auto-tap planner has to use lands.
    // Equip {2} should appear in legal actions and the lands should be auto-tapped.
    let reg = registry();
    let (state, dagger, bears, forests) = setup_with_lands(&reg, "Silver-Inlaid Dagger", "Grizzly Bears", 2);

    let action = find_equip_action(&state, &reg, dagger, bears);

    // The action's tap_plan should reference both Forests.
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        assert_eq!(tap_plan.len(), 2, "tap_plan should tap both forests for {{2}} cost, got {tap_plan:?}");
        let tap_ids: Vec<ObjectId> = tap_plan.iter().map(|(id, _)| *id).collect();
        for f in &forests {
            assert!(tap_ids.contains(f), "tap_plan should reference forest {}", f.0);
        }
    } else {
        panic!("expected ActivateAbility");
    }

    let new_state = engine::submit_action(&state, &action, &reg);

    // After submit: equipment should be attached, both forests should be tapped,
    // mana pool should be empty (mana was spent on the equip cost).
    assert_eq!(
        new_state.get_object(dagger).unwrap().attached_to,
        Some(bears),
        "dagger should be attached to bears"
    );
    for f in &forests {
        assert!(new_state.get_object(*f).unwrap().tapped, "forest {} should be tapped after equip", f.0);
    }
    assert!(new_state.get_player(P0).mana_pool.is_empty(), "mana pool should be empty after paying equip cost");
}

#[test]
fn silver_inlaid_dagger_equip_grants_static_buff_to_human() {
    // After equip, Avacyn's Pilgrim (1/1 Human) should be 4/1.
    // Base dagger: +2/+0. Human bonus: +1/+0. So 1+3 = 4 power, 1 toughness.
    // Pilgrim has a {T}: Add {W} mana ability, but since we provide enough Forests
    // for the equip cost, the auto-tap should not need it.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    // Three Forests so the auto-tap can tap two and the test isn't sensitive to
    // whether the planner prefers lands or the creature mana ability.
    let _forests = untapped_lands(&mut state, &reg, "Forest", P0, 3);

    let action = find_equip_action(&state, &reg, dagger, pilgrim);
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.effective_power(pilgrim, &reg), Some(4), "pilgrim should be 4 power after equip");
    assert_eq!(new_state.effective_toughness(pilgrim, &reg), Some(1), "pilgrim should be 1 toughness after equip");
}

#[test]
fn silver_inlaid_dagger_not_offered_with_insufficient_lands() {
    // 1 Forest is not enough mana for {2}; the equip action must NOT appear.
    // We use Grizzly Bears (no mana ability) as the target creature so the
    // auto-tap planner only has the one Forest as a mana source.
    let reg = registry();
    let (state, dagger, _bears, _) = setup_with_lands(&reg, "Silver-Inlaid Dagger", "Grizzly Bears", 1);
    let legal = engine::legal_actions(&state, &reg);
    let any_dagger_activate = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == dagger));
    assert!(!any_dagger_activate, "equip should NOT appear with only 1 land for a {{2}} cost");
}

#[test]
fn silver_inlaid_dagger_offered_with_floating_mana_too() {
    // The legacy "mana pool only" path must still work.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 2);
    let action = find_equip_action(&state, &reg, dagger, pilgrim);
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        // Cost is already paid from the pool — tap_plan should be empty.
        assert!(tap_plan.is_empty(), "tap_plan should be empty when mana pool already has the cost");
    }
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(dagger).unwrap().attached_to, Some(pilgrim));
    assert!(new_state.get_player(P0).mana_pool.is_empty());
}

// ════════════════════════════════════════════════════════════════════
// Butcher's Cleaver — Equip {3}
// ════════════════════════════════════════════════════════════════════

#[test]
fn butchers_cleaver_equip_offered_with_three_untapped_lands() {
    let reg = registry();
    let (state, cleaver, bears, _) = setup_with_lands(&reg, "Butcher's Cleaver", "Grizzly Bears", 3);
    let action = find_equip_action(&state, &reg, cleaver, bears);
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        assert_eq!(tap_plan.len(), 3, "Equip {{3}} should tap 3 lands");
    }
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(cleaver).unwrap().attached_to, Some(bears));
    // Bears 2/2 + cleaver +3/+0 (non-Human, no Human bonus) = 5/2.
    // Oracle: "Equipped creature gets +3/+0. As long as equipped creature is a
    // Human, it has lifelink."
    assert_eq!(new_state.effective_power(bears, &reg), Some(5));
    assert_eq!(new_state.effective_toughness(bears, &reg), Some(2));
}

#[test]
fn butchers_cleaver_not_offered_with_two_lands() {
    let reg = registry();
    let (state, cleaver, _, _) = setup_with_lands(&reg, "Butcher's Cleaver", "Grizzly Bears", 2);
    let legal = engine::legal_actions(&state, &reg);
    let any = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == cleaver));
    assert!(!any, "Equip {{3}} should not be offered with only 2 lands");
}

// ════════════════════════════════════════════════════════════════════
// Cobbled Wings — Equip {1}
// ════════════════════════════════════════════════════════════════════

#[test]
fn cobbled_wings_equip_offered_with_one_untapped_land() {
    let reg = registry();
    let (state, wings, bears, _) = setup_with_lands(&reg, "Cobbled Wings", "Grizzly Bears", 1);
    let action = find_equip_action(&state, &reg, wings, bears);
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        assert_eq!(tap_plan.len(), 1);
    }
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(wings).unwrap().attached_to, Some(bears));
    assert!(new_state.has_keyword(bears, Keyword::Flying, &reg), "Bears should now have flying");
}

// ════════════════════════════════════════════════════════════════════
// Blazing Torch — Equip {1}
// ════════════════════════════════════════════════════════════════════

#[test]
fn blazing_torch_equip_offered_with_one_untapped_land() {
    let reg = registry();
    let (state, torch, bears, _) = setup_with_lands(&reg, "Blazing Torch", "Grizzly Bears", 1);
    let action = find_equip_action(&state, &reg, torch, bears);
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        assert_eq!(tap_plan.len(), 1);
    }
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(torch).unwrap().attached_to, Some(bears));
}

// ════════════════════════════════════════════════════════════════════
// Sharpened Pitchfork — Equip {1}
// ════════════════════════════════════════════════════════════════════

#[test]
fn sharpened_pitchfork_equip_offered_with_one_untapped_land() {
    let reg = registry();
    let (state, fork, bears, _) = setup_with_lands(&reg, "Sharpened Pitchfork", "Grizzly Bears", 1);
    let action = find_equip_action(&state, &reg, fork, bears);
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        assert_eq!(tap_plan.len(), 1, "Equip {{1}} should tap 1 land");
    }
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(fork).unwrap().attached_to, Some(bears));
}

// ════════════════════════════════════════════════════════════════════
// Wooden Stake — Equip {1}
// ════════════════════════════════════════════════════════════════════

#[test]
fn wooden_stake_equip_offered_with_one_untapped_land() {
    let reg = registry();
    let (state, equip, bears, _) = setup_with_lands(&reg, "Wooden Stake", "Grizzly Bears", 1);
    let action = find_equip_action(&state, &reg, equip, bears);
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(equip).unwrap().attached_to, Some(bears));
}

// ════════════════════════════════════════════════════════════════════
// Mask of Avacyn — Equip {3}
// ════════════════════════════════════════════════════════════════════

#[test]
fn mask_of_avacyn_equip_offered_with_three_untapped_lands() {
    let reg = registry();
    let (state, mask, bears, _) = setup_with_lands(&reg, "Mask of Avacyn", "Grizzly Bears", 3);
    let action = find_equip_action(&state, &reg, mask, bears);
    if let Action::ActivateAbility { tap_plan, .. } = &action {
        assert_eq!(tap_plan.len(), 3);
    }
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(mask).unwrap().attached_to, Some(bears));
    assert!(new_state.has_keyword(bears, Keyword::Hexproof, &reg));
}

// ════════════════════════════════════════════════════════════════════
// Cross-cutting: every ISD equipment with a mana equip cost gets tested.
//
// This is a parameterized smoke test: for each (equipment, equip_cost) pair,
// we set up the equipment + a creature + that many untapped Forests, and
// verify the engine offers an equip action whose tap_plan covers the cost
// and which actually attaches the equipment when submitted.
// ════════════════════════════════════════════════════════════════════

/// Every Equipment in the set whose equip cost is plain mana, as
/// (name, generic mana value). Read off the card rather than hand-listed, so a
/// newly added Equipment is covered the day it is registered and a renamed one
/// cannot silently drop out of the table.
fn equipment_with_a_mana_equip_cost() -> Vec<(String, usize)> {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mut out = Vec::new();
    for name in reg.all_names() {
        let Some(id) = reg.get_id_by_name(name) else { continue };
        let Some(data) = reg.card_data(id) else { continue };
        if !data.subtypes.iter().any(|s| s == "Equipment") {
            continue;
        }
        let obj = equipment(&mut state, &reg, name, P0);
        let Some(ability) = reg.get(id).unwrap().activated_abilities(&state, obj, &reg)
            .into_iter()
            .find(|a| a.description.to_lowercase().contains("equip"))
        else { continue };
        // Sacrifice-cost equips (Demonmail Hauberk) are not autotap cases.
        if !matches!(ability.sacrifice_cost, mtg_engine::cards::SacrificeCost::None) {
            continue;
        }
        let cost = ability.cost.mana_value() as usize;
        if cost > 0 {
            out.push(((*name).to_string(), cost));
        }
    }
    out.sort();
    out
}

#[test]
fn every_mana_cost_equipment_in_isd_can_be_equipped_via_autotap() {
    let reg = registry();
    let cases = equipment_with_a_mana_equip_cost();
    assert!(cases.len() >= 8,
        "only {} mana-cost Equipment found — this sweep has stopped covering the set",
        cases.len());
    for (name, equip_cost) in &cases {
        let (name, equip_cost) = (name.as_str(), *equip_cost);
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let bears = named_creature(&mut state, &reg, "Grizzly Bears", P0);
        let eq = equipment(&mut state, &reg, name, P0);
        let _forests = untapped_lands(&mut state, &reg, "Forest", P0, equip_cost);

        let legal = engine::legal_actions(&state, &reg);
        let action = legal.actions.iter().find(|a| matches!(a,
            Action::ActivateAbility { object_id, targets, .. }
                if *object_id == eq && targets == &[Target::Object(bears)]
        )).cloned();

        let action = action.unwrap_or_else(|| panic!(
            "[{name}] equip {{{equip_cost}}}: no Activate action found in legal_actions; the engine \
             is dropping mana-cost activated abilities again — see Bug A in the audit."
        ));

        if let Action::ActivateAbility { tap_plan, .. } = &action {
            assert_eq!(tap_plan.len(), equip_cost,
                "[{}] tap_plan should tap exactly {} land(s), got {}", name, equip_cost, tap_plan.len());
        }

        let new_state = engine::submit_action(&state, &action, &reg);
        assert_eq!(
            new_state.get_object(eq).unwrap().attached_to,
            Some(bears),
            "[{name}] equipment should be attached after submit"
        );
        assert!(
            new_state.get_player(P0).mana_pool.is_empty(),
            "[{name}] mana pool should be empty after equip"
        );
    }
}

// ════════════════════════════════════════════════════════════════════
// Non-equipment regression: Elder of Laurels' {3}{G} ability
//
// This was the second card the audit caught — same root cause (mana cost,
// not tap, not sacrifice). Including it here so the auto-tap fix is
// covered for non-equipment activated abilities too.
// ════════════════════════════════════════════════════════════════════

#[test]
fn elder_of_laurels_activated_ability_offered_with_untapped_lands() {
    let reg = registry();
    if reg.get_id_by_name("Elder of Laurels").is_none() {
        return;
    }
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let elder = named_creature(&mut state, &reg, "Elder of Laurels", P0);
    // Three Forests + one of any colour gives us {3}{G}: 4 generic mana, one green.
    let _forests = untapped_lands(&mut state, &reg, "Forest", P0, 4);

    let legal = engine::legal_actions(&state, &reg);
    let any_activate = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == elder));
    assert!(any_activate,
        "Elder of Laurels {{3}}{{G}} ability should be offered when 4 untapped Forests are available; \
         the engine is dropping mana-cost activated abilities again — see Bug A in the audit."
    );
}

#[test]
fn elder_of_laurels_activated_ability_not_offered_with_three_lands() {
    let reg = registry();
    if reg.get_id_by_name("Elder of Laurels").is_none() {
        return;
    }
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let elder = named_creature(&mut state, &reg, "Elder of Laurels", P0);
    let _forests = untapped_lands(&mut state, &reg, "Forest", P0, 3);

    let legal = engine::legal_actions(&state, &reg);
    let any_activate = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == elder));
    assert!(!any_activate, "Elder of Laurels should not be activatable with only 3 lands for a {{3}}{{G}} ability");
}

// ════════════════════════════════════════════════════════════════════
// Sorcery-speed gating: Equip is "activate only as a sorcery", so it must
// NOT appear during the opponent's turn even if the player has the mana.
// ════════════════════════════════════════════════════════════════════

#[test]
fn equip_is_not_offered_at_instant_speed() {
    let reg = registry();
    // Set the active player to P1, so it's the opponent's turn from P0's POV.
    let mut state = game_at_step(Step::PrecombatMain, P1);
    state.priority_player = Some(P0); // P0 has priority during opp's main phase
    let pilgrim = named_creature(&mut state, &reg, "Avacyn's Pilgrim", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let _forests = untapped_lands(&mut state, &reg, "Forest", P0, 2);

    let legal = engine::legal_actions(&state, &reg);
    let any = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
            if *object_id == dagger && targets == &[Target::Object(pilgrim)]));
    assert!(!any, "Equip is sorcery-speed; it must not appear on the opponent's turn");
}

// ════════════════════════════════════════════════════════════════════
// Targeting: equipment can only target creatures *you* control.
// (Already covered in tier9_equipment.rs but worth restating in this file
// since we now have a different mana setup that previously masked it.)
// ════════════════════════════════════════════════════════════════════

#[test]
fn equip_cannot_target_opponent_creature_via_autotap() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let opp_bears = named_creature(&mut state, &reg, "Grizzly Bears", P1);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let _forests = untapped_lands(&mut state, &reg, "Forest", P0, 2);

    let legal = engine::legal_actions(&state, &reg);
    let bad = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, .. }
            if *object_id == dagger && targets == &[Target::Object(opp_bears)]));
    assert!(!bad, "equip must not be able to target an opponent's creature even with autotap");
}

// ════════════════════════════════════════════════════════════════════
// Reattachment: equipping while already attached moves the equipment
// to the new creature.
// ════════════════════════════════════════════════════════════════════

#[test]
fn equip_can_reattach_via_autotap() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bears_a = named_creature(&mut state, &reg, "Grizzly Bears", P0);
    let bears_b = named_creature(&mut state, &reg, "Grizzly Bears", P0);
    let dagger = equipment(&mut state, &reg, "Silver-Inlaid Dagger", P0);
    let _forests = untapped_lands(&mut state, &reg, "Forest", P0, 4); // enough for two equips

    // First equip → bears_a
    let action_a = find_equip_action(&state, &reg, dagger, bears_a);
    let state = engine::submit_action(&state, &action_a, &reg);
    assert_eq!(state.get_object(dagger).unwrap().attached_to, Some(bears_a));

    // Second equip → bears_b. This should require a fresh tap_plan (the first
    // two forests are now tapped, so the engine should pick the other two).
    let action_b = find_equip_action(&state, &reg, dagger, bears_b);
    if let Action::ActivateAbility { tap_plan, .. } = &action_b {
        assert_eq!(tap_plan.len(), 2);
        // Make sure the new tap plan only contains untapped lands.
        for (id, _) in tap_plan {
            assert!(!state.get_object(*id).unwrap().tapped,
                "second equip should pick from untapped lands only");
        }
    }
    let state = engine::submit_action(&state, &action_b, &reg);
    assert_eq!(state.get_object(dagger).unwrap().attached_to, Some(bears_b));
    assert_ne!(state.get_object(dagger).unwrap().attached_to, Some(bears_a));
}

// ════════════════════════════════════════════════════════════════════
// Bug AJ regression: equipment equip-ability must appear exactly once
// in legal_actions, even when the equipment is already attached to a
// creature. The engine's attached-iteration loop in legal_actions
// (engine.rs:551-559) calls each attached object's `activated_abilities`
// with the *attached creature's* object_id; equipment cards must gate
// on `power.is_none()` or they will return their equip ability for the
// creature query too, producing a duplicate Action::ActivateAbility
// with `object_id = creature_id`. The duplicate then misroutes
// on_activate_ability to mutate the creature's `attached_to` field.
// ════════════════════════════════════════════════════════════════════

fn assert_equip_ability_unique_after_attach(reg: &CardRegistry, equip_name: &str, n_lands: usize) {
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bears_a = named_creature(&mut state, reg, "Grizzly Bears", P0);
    let bears_b = named_creature(&mut state, reg, "Grizzly Bears", P0);
    let eq = equipment(&mut state, reg, equip_name, P0);
    let _forests = untapped_lands(&mut state, reg, "Forest", P0, n_lands);

    // Attach to bears_a first.
    let action_a = find_equip_action(&state, reg, eq, bears_a);
    let state = engine::submit_action(&state, &action_a, reg);
    assert_eq!(state.get_object(eq).unwrap().attached_to, Some(bears_a));

    // Now look at the actions targeting bears_b. The equip ability belongs to
    // the equipment: exactly one, offered by `eq`. Without the Bug AJ fix a
    // second copy appears with `object_id = bears_a` — the equipment's own
    // ability list misapplied to the creature it is attached to, which is
    // recognisable by its missing `source_card_id`.
    //
    // Abilities the equipment *grants* to the equipped creature (Blazing
    // Torch's "{T}, Sacrifice: deal 2 damage") also show up on bears_a, and
    // those are correct — they carry the granting card's id.
    let legal = engine::legal_actions(&state, reg);
    let to_b: Vec<&Action> = legal.actions.iter().filter(|a| matches!(a,
        Action::ActivateAbility { targets, .. }
            if targets == &[Target::Object(bears_b)])).collect();

    let from_equipment = to_b.iter().filter(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == eq)).count();
    assert_eq!(from_equipment, 1,
        "{equip_name}: the equip ability should appear exactly once per target, got {from_equipment}: {to_b:?}");

    let misrouted: Vec<&&Action> = to_b.iter().filter(|a| matches!(a,
        Action::ActivateAbility { object_id, source_card_id: None, .. } if *object_id != eq)).collect();
    assert!(misrouted.is_empty(),
        "{equip_name}: the equipment's own ability was offered on the creature it is \
         attached to, not on the equipment: {misrouted:?}");
}

#[test]
fn no_equipment_offers_its_equip_ability_twice_after_attaching() {
    let reg = registry();
    let cases = equipment_with_a_mana_equip_cost();
    assert!(cases.len() >= 8,
        "only {} mana-cost Equipment found — this sweep has stopped covering the set",
        cases.len());
    for (name, equip_cost) in &cases {
        // Two creatures and enough lands to equip twice over.
        assert_equip_ability_unique_after_attach(&reg, name, equip_cost * 2);
    }
}

#[test]
fn bug_mask_of_avacyn_duplicate_equip_action() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Mask of Avacyn and two creatures
    let mask = named_creature(&mut state, &registry, "Mask of Avacyn", P0);
    if let Some(obj) = state.get_object_mut(mask) {
        obj.is_equipment = true;
    }
    let c1 = ready_creature(&mut state, P0, 2, 2);
    let _c2 = ready_creature(&mut state, P0, 3, 3);

    // Equip to c1
    if let Some(obj) = state.get_object_mut(mask) {
        obj.attached_to = Some(c1);
    }

    // Add mana for equip cost
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    // Get legal actions
    let legal = engine::legal_actions(&state, &registry);
    let equip_targets: Vec<_> = legal.actions.iter().filter_map(|a| match a {
        Action::ActivateAbility { object_id, targets, .. } if *object_id == mask => Some(targets.clone()),
        _ => None,
    }).collect();

    // The bug this guards is the attached-permanent loop offering the SAME
    // equip twice, so the check is for duplicates — not for a target count.
    // Both creatures are legal targets: CR 702.6a does not exclude the
    // creature the equipment is already attached to, and re-equipping to the
    // current host is a real play whenever the equip cost is what you want.
    let mut seen: Vec<&Vec<Target>> = Vec::new();
    for t in &equip_targets {
        assert!(!seen.contains(&t),
            "each equip target should be offered exactly once; {t:?} appears \
             twice in {equip_targets:?}");
        seen.push(t);
    }
    assert_eq!(equip_targets.len(), 2,
        "both creatures the player controls are legal equip targets, including \
         the one already wearing the Mask (CR 702.6a); got {equip_targets:?}");
}
