//! Regression tests for the `SacrificeCost` auto-pick bug (audit finding).
//!
//! Background: in earlier versions, when an activated ability with
//! `SacrificeCost::SacrificeCreature` resolved, the engine auto-picked the
//! first creature in the zone as the sacrifice. For Demonmail Hauberk's
//! "Equip — Sacrifice a creature" ability that meant the engine could (and
//! did, in the audit log) sacrifice the very creature being targeted by the
//! equip — fizzling the ability. The player lost a creature for nothing.
//!
//! Same bug applied to Disciple of Griselbrand and Skirsdag Cultist (both use
//! `SacrificeCreature`). The auto-pick also gave Disciple a non-optimal sacrifice
//! since the player couldn't pick the highest-toughness creature.
//!
//! The fix: `legal_actions` enumerates one `Action::ActivateAbility` per
//! (target, sacrifice) combo, mirroring how `CastSpell` handles spell-side
//! sacrifice costs. The apply path uses the explicit sacrifice rather than
//! auto-picking. The (target, sacrifice) pairs where sacrifice == target are
//! filtered out so the player can never accidentally pick a fizzling combo.
//!
//! These tests pin all of those properties.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::state::GameState;
use mtg_engine::types::*;

fn registry() -> CardRegistry {
    CardRegistry::with_all_cards()
}

fn equipment(state: &mut GameState, reg: &CardRegistry, name: &str, owner: PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown card {name}"));
    let id = state.create_object(card_id, owner, Zone::Battlefield, None, None);
    let obj = state.get_object_mut(id).unwrap();
    obj.name = name.into();
    obj.is_equipment = true;
    id
}

// ════════════════════════════════════════════════════════════════════
// Demonmail Hauberk — Equip — Sacrifice a creature
// ════════════════════════════════════════════════════════════════════

#[test]
fn hauberk_legal_actions_enumerate_target_sacrifice_combos() {
    // 3 creatures + Hauberk on board. The engine should enumerate one
    // ActivateAbility per (target, sacrifice) pair where target != sacrifice.
    // 3 targets × 2 valid sacrifices = 6 combos.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = equipment(&mut state, &reg, "Demonmail Hauberk", P0);
    let a = ready_creature(&mut state, P0, 1, 1);
    let b = ready_creature(&mut state, P0, 2, 2);
    let c = ready_creature(&mut state, P0, 3, 3);

    let legal = engine::legal_actions(&state, &reg);
    let combos: Vec<(ObjectId, ObjectId)> = legal.actions.iter().filter_map(|act| {
        if let Action::ActivateAbility { object_id, targets, sacrifice, .. } = act {
            if *object_id != hauberk { return None; }
            let target_id = match targets.first()? {
                Target::Object(id) => *id,
                Target::Player(_) => return None,
            };
            let sac_id = (*sacrifice)?;
            Some((target_id, sac_id))
        } else { None }
    }).collect();

    assert_eq!(combos.len(), 6, "should enumerate 3 targets × 2 sacrifices = 6 combos, got {combos:?}");

    // Check no combo has target == sacrifice.
    for (t, s) in &combos {
        assert_ne!(t, s, "no combo should have target == sacrifice (would fizzle)");
    }

    // Check we cover every (target, non-target sacrifice) pair.
    for &t in &[a, b, c] {
        for &s in &[a, b, c] {
            if t == s { continue; }
            assert!(combos.contains(&(t, s)), "missing combo (target={}, sac={})", t.0, s.0);
        }
    }
}

#[test]
fn hauberk_explicit_sacrifice_attaches_correctly() {
    // The audit log scenario: hauberk + 3 creatures, model wants to equip
    // creature_b and sacrifice creature_a. With the fix, the engine respects
    // the player's explicit choice and the equip succeeds.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = equipment(&mut state, &reg, "Demonmail Hauberk", P0);
    let creature_a = ready_creature(&mut state, P0, 1, 1);
    let creature_b = ready_creature(&mut state, P0, 2, 2);

    let action = Action::ActivateAbility {
        object_id: hauberk,
        ability_index: 0,
        targets: vec![Target::Object(creature_b)],
        tap_plan: vec![],
        sacrifice: Some(creature_a),
        x_value: None,
    };
    let new_state = engine::submit_action(&state, &action, &reg);

    // creature_a sacrificed, creature_b equipped, hauberk attached.
    assert_eq!(new_state.get_object(creature_a).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(creature_b).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(hauberk).unwrap().attached_to, Some(creature_b));
    // Hauberk gives +4/+2 → creature_b should be 2+4 = 6 power, 2+2 = 4 toughness.
    assert_eq!(new_state.effective_power(creature_b, &reg), Some(6));
    assert_eq!(new_state.effective_toughness(creature_b, &reg), Some(4));
}

#[test]
fn hauberk_with_only_one_creature_offers_no_legal_actions() {
    // Hauberk + 1 creature: the only creature is also the only valid target.
    // After filtering combos where target == sac, there's nothing left, so
    // the ability should NOT appear in legal actions.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = equipment(&mut state, &reg, "Demonmail Hauberk", P0);
    let _solo = ready_creature(&mut state, P0, 2, 2);

    let legal = engine::legal_actions(&state, &reg);
    let any_hauberk = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == hauberk));
    assert!(!any_hauberk,
        "Demonmail Hauberk should not be activatable with only one creature \
         (it would have to sacrifice the same creature it's trying to equip)");
}

#[test]
fn hauberk_does_not_offer_combo_where_target_equals_sacrifice() {
    // Even with multiple creatures, no combo should ever set target == sacrifice.
    // This is the regression case for the original bug.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hauberk = equipment(&mut state, &reg, "Demonmail Hauberk", P0);
    let _a = ready_creature(&mut state, P0, 1, 1);
    let _b = ready_creature(&mut state, P0, 2, 2);
    let _c = ready_creature(&mut state, P0, 3, 3);

    let legal = engine::legal_actions(&state, &reg);
    for act in &legal.actions {
        if let Action::ActivateAbility { object_id, targets, sacrifice, .. } = act {
            if *object_id != hauberk { continue; }
            let target_id = match targets.first() {
                Some(Target::Object(id)) => *id,
                _ => continue,
            };
            assert_ne!(
                Some(target_id), *sacrifice,
                "engine offered a target=sacrifice combo, which would fizzle"
            );
        }
    }
}

// ════════════════════════════════════════════════════════════════════
// Disciple of Griselbrand — {1}, Sacrifice a creature: gain life equal to toughness
// ════════════════════════════════════════════════════════════════════

#[test]
fn disciple_of_griselbrand_player_picks_highest_toughness_sacrifice() {
    // Disciple has an untargeted SacrificeCreature ability — the player
    // should be able to pick which creature to sacrifice (i.e. the one with
    // the highest toughness for max life gain).
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_creature(&mut state, &reg, "Disciple of Griselbrand", P0);
    let small = ready_creature(&mut state, P0, 1, 1);
    let big = ready_creature(&mut state, P0, 2, 6);

    // Mana for the {1} cost.
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    // Should enumerate one ActivateAbility per eligible sacrifice (3 creatures total).
    let activates: Vec<&Action> = legal.actions.iter().filter(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == disciple)).collect();
    assert_eq!(activates.len(), 3,
        "should offer one option per sacrificable creature (disciple itself + small + big), got {}",
        activates.len());

    // Now pick the option that sacrifices the big creature.
    let big_action = activates.iter().find(|a| matches!(a,
        Action::ActivateAbility { sacrifice: Some(s), .. } if *s == big)).unwrap();
    let life_before = state.get_player(P0).life;
    let new_state = engine::submit_action(&state, big_action, &reg);
    let gained = new_state.get_player(P0).life - life_before;
    assert_eq!(gained, 6, "should gain 6 life (big creature's toughness)");
    assert_eq!(new_state.get_object(big).unwrap().zone, Zone::Graveyard);
    // Disciple and small both still on board.
    assert_eq!(new_state.get_object(disciple).unwrap().zone, Zone::Battlefield);
    assert_eq!(new_state.get_object(small).unwrap().zone, Zone::Battlefield);
}

#[test]
fn disciple_of_griselbrand_can_sacrifice_itself() {
    // Disciple is its own valid sacrifice — gain 1 life from its 1 toughness
    // (and lose the disciple). The legal_actions list should include this combo.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_creature(&mut state, &reg, "Disciple of Griselbrand", P0);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let self_sac_action = legal.actions.iter().find(|a| matches!(a,
        Action::ActivateAbility { object_id, sacrifice: Some(s), .. }
            if *object_id == disciple && *s == disciple)).cloned();
    assert!(self_sac_action.is_some(),
        "disciple should be allowed to sacrifice itself for the cost");

    let new_state = engine::submit_action(&state, &self_sac_action.unwrap(), &reg);
    // Disciple in graveyard, gained 1 life.
    assert_eq!(new_state.get_object(disciple).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_player(P0).life, 21);
}

// ════════════════════════════════════════════════════════════════════
// Skirsdag Cultist — {R}, {T}, Sacrifice a creature: 2 damage to any target
// ════════════════════════════════════════════════════════════════════

#[test]
fn skirsdag_cultist_explicit_sacrifice() {
    // Cultist + fodder + opponent creature. Player picks fodder as the
    // sacrifice and the opponent creature as the damage target.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    let fodder = ready_creature(&mut state, P0, 1, 1);
    let target = ready_creature(&mut state, P1, 3, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let action = Action::ActivateAbility {
        object_id: cultist,
        ability_index: 0,
        targets: vec![Target::Object(target)],
        tap_plan: vec![],
        sacrifice: Some(fodder),
        x_value: None,
    };
    let new_state = engine::submit_action(&state, &action, &reg);
    assert_eq!(new_state.get_object(target).unwrap().damage_marked, 2);
    assert_eq!(new_state.get_object(fodder).unwrap().zone, Zone::Graveyard);
    assert_eq!(new_state.get_object(cultist).unwrap().zone, Zone::Battlefield,
        "cultist should still be alive (we sacrificed the fodder, not itself)");
}

#[test]
fn skirsdag_cultist_combo_count_excludes_self_targeting_sacrifices() {
    // Setup: cultist + fodder + opp creature + 1 untapped Red mana floating.
    // "Any target" damage: legal targets are cultist, fodder, opp creature,
    // P0 (you), and P1 (opp) — 5 targets total.
    // Sacrifices: cultist or fodder — 2 options.
    // Naive enumeration would be 5 × 2 = 10 combos, but for the two
    // own-creature targets we filter out the matching sacrifice (target ==
    // sacrifice would fizzle). So:
    //   target=cultist:    sac in {fodder}        = 1
    //   target=fodder:     sac in {cultist}        = 1
    //   target=opp_creature: sac in {cultist, fodder} = 2
    //   target=P0:         sac in {cultist, fodder} = 2
    //   target=P1:         sac in {cultist, fodder} = 2
    //   total = 8
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    let _target = ready_creature(&mut state, P1, 3, 3);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    let count = legal.actions.iter().filter(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == cultist)).count();
    assert_eq!(count, 8,
        "expected 8 (target, sacrifice) combos for cultist after filtering out self-fizzles");

    // And confirm the two filtered-out combos really aren't there.
    for act in &legal.actions {
        if let Action::ActivateAbility { object_id, targets, sacrifice, .. } = act {
            if *object_id != cultist { continue; }
            if let Some(Target::Object(t)) = targets.first() {
                assert_ne!(Some(*t), *sacrifice,
                    "no combo should target and sacrifice the same creature");
            }
        }
    }
}

#[test]
fn skirsdag_cultist_targeting_own_creature_excludes_fizzling_combo() {
    // If the player targets their own creature (e.g. to ping a creature with
    // 2 toughness for free) and uses cultist's ability, sacrificing that same
    // creature would fizzle the damage. The combo must be filtered out.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let cultist = named_creature(&mut state, &reg, "Skirsdag Cultist", P0);
    let own_creature = ready_creature(&mut state, P0, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Red, 1);

    let legal = engine::legal_actions(&state, &reg);
    for act in &legal.actions {
        if let Action::ActivateAbility { object_id, targets, sacrifice, .. } = act {
            if *object_id != cultist { continue; }
            if let Some(Target::Object(t)) = targets.first() {
                assert_ne!(Some(*t), *sacrifice,
                    "no combo should have target == sacrifice");
            }
        }
    }

    // Check the specific case: target=own_creature, sacrifice=own_creature.
    let bad_combo = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, targets, sacrifice: Some(s), .. }
            if *object_id == cultist
            && targets == &[Target::Object(own_creature)]
            && *s == own_creature));
    assert!(!bad_combo, "should not offer cultist targeting and sacrificing the same creature");
}

// ════════════════════════════════════════════════════════════════════
// No-mana-autotap-with-sacrifice rule
// ════════════════════════════════════════════════════════════════════

#[test]
fn disciple_does_not_appear_with_only_untapped_lands_and_no_floating_mana() {
    // Disciple's {1} cost: with 1 untapped land and no floating mana, the
    // ability should NOT auto-tap (sacrifice abilities require manual mana).
    // This protects against the engine tapping a creature mana source for the
    // {1} and then sacrificing that same creature, or other autotap weirdness.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_creature(&mut state, &reg, "Disciple of Griselbrand", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    // 1 untapped Forest, but no mana floating in the pool.
    let forest_id = reg.get_id_by_name("Forest").unwrap();
    let forest = state.create_object(forest_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(forest).unwrap().name = "Forest".into();
    assert!(state.get_player(P0).mana_pool.is_empty());

    let legal = engine::legal_actions(&state, &reg);
    let any_disciple = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == disciple));
    assert!(!any_disciple,
        "disciple's ability should NOT appear when only untapped lands are available — \
         the player must manually tap their lands first to float the mana");
}

#[test]
fn disciple_appears_when_mana_is_already_in_the_pool() {
    // Same setup as above but with the {1} already floating in the mana pool —
    // now the ability should appear.
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let disciple = named_creature(&mut state, &reg, "Disciple of Griselbrand", P0);
    let _fodder = ready_creature(&mut state, P0, 1, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let legal = engine::legal_actions(&state, &reg);
    let any_disciple = legal.actions.iter().any(|a| matches!(a,
        Action::ActivateAbility { object_id, .. } if *object_id == disciple));
    assert!(any_disciple,
        "disciple's ability should appear once the mana is in the pool");
}
