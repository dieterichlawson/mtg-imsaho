//! Regression tests for protection-from-source during *activated ability*
//! targeting (CR 702.16b).
//!
//! `generate_ability_targets` used to call a `can_be_targeted` wrapper that
//! always passed `None` for `source_id`, so the protection check inside
//! `can_be_targeted_by` was silently skipped. Only the spell path
//! (`valid_targets_for_req`) threaded the source through. A creature with
//! protection from the ability's source therefore showed up as a legal
//! target for every activated ability.

mod common;
use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::engine;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::TemporaryEffect;
use mtg_engine::types::*;

/// Collect every target offered for an activated ability of `source`.
fn ability_targets(state: &mtg_engine::state::GameState, reg: &CardRegistry, source: ObjectId) -> Vec<Target> {
    engine::legal_actions(state, reg).actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { object_id, targets, .. } if *object_id == source => {
                Some(targets.clone())
            }
            _ => None,
        })
        .flatten()
        .collect()
}

/// Give `target` protection from Humans (the Spare from Evil shape).
fn protect_from_humans(state: &mut mtg_engine::state::GameState, target: ObjectId) {
    state.until_end_of_turn.push(TemporaryEffect::GrantProtection {
        target,
        filter: CreatureFilter::HasSubtype("Human".into()),
    });
}

/// Avacynian Priest ({1}, {T}: Tap target non-Human creature) is a Human
/// Cleric, so a non-Human creature with protection from Humans is an illegal
/// target for its ability. `TargetRequirement::CreatureWithFilter`.
#[test]
fn avacynian_priest_cannot_target_creature_with_protection_from_its_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let priest = named_permanent(&mut state, &reg, "Avacynian Priest", P0);
    let zombie = ready_creature(&mut state, P1, 2, 2);
    state.get_object_mut(zombie).unwrap().subtypes = vec!["Zombie".into()];
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 1);

    let before = ability_targets(&state, &reg, priest);
    assert!(before.contains(&Target::Object(zombie)),
        "test precondition: an unprotected non-Human creature is a legal target; got {before:?}");

    protect_from_humans(&mut state, zombie);

    let after = ability_targets(&state, &reg, priest);
    assert!(!after.contains(&Target::Object(zombie)),
        "a creature with protection from Humans must not be targetable by the \
         Human Avacynian Priest's ability; got {after:?}");
}

/// `TargetRequirement::Creature` takes the same path. Elder of Laurels
/// ({3}{G}, {T}: Target creature gets +X/+X ...) is a Human Shaman.
#[test]
fn elder_of_laurels_cannot_target_creature_with_protection_from_its_source() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let elder = named_permanent(&mut state, &reg, "Elder of Laurels", P0);
    let bear = ready_creature(&mut state, P0, 2, 2);
    state.get_player_mut(P0).mana_pool.add(ManaType::Green, 1);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let before = ability_targets(&state, &reg, elder);
    assert!(before.contains(&Target::Object(bear)),
        "test precondition: an unprotected creature is a legal target; got {before:?}");

    protect_from_humans(&mut state, bear);

    let after = ability_targets(&state, &reg, elder);
    assert!(!after.contains(&Target::Object(bear)),
        "a creature with protection from Humans must not be targetable by the \
         Human Elder of Laurels' ability; got {after:?}");
}

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: Grave Bramble has protection from Zombies, but Grimgrin's attack
/// trigger ("destroy target creature defending player controls") can still
/// target it. Protection should prevent targeting by Zombie sources.
/// The engine's `can_be_targeted` doesn't consider the source's subtypes.
#[test]
fn bug_protection_doesnt_prevent_zombie_source_targeting() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Grave Bramble for P1 (has protection from Zombies)
    let bramble = named_permanent(&mut state, &registry, "Grave Bramble", P1);

    // Place Grimgrin for P0 (is a Zombie)
    let grimgrin = named_permanent(&mut state, &registry, "Grimgrin, Corpse-Born", P0);

    // Grimgrin is a Zombie. Its attack trigger targets a creature defending player controls.
    // Grave Bramble has protection from Zombies, so Grimgrin's ability should not be
    // able to target it. But the engine only checks hexproof for targeting, not protection.

    // We can test this by checking if Grimgrin's on_attacks would present Grave Bramble
    // as a valid target. Set up combat state.
    state.step = Step::DeclareAttackers;
    attacks_unblocked(&mut state, grimgrin, P1);

    // Fire the attack trigger
    let behavior = registry.get(state.get_object(grimgrin).unwrap().card_id).unwrap();
    behavior.on_attacks(&mut state, grimgrin, AttackInfo::new(grimgrin, P1), &[], &registry);

    // Check if Grave Bramble is in the target options
    let bramble_is_target = match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options.iter().any(|t| matches!(t, Target::Object(id) if *id == bramble)),
        _ => {
            // If there's only one target (Grave Bramble), auto-applied
            // Check if Grave Bramble was destroyed
            state.get_object(bramble).is_some_and(|o| o.zone != Zone::Battlefield)
        }
    };

    // BUG: Grave Bramble appears as a valid target (or was auto-destroyed)
    // despite having protection from Zombies. Grimgrin is a Zombie, so its
    // ability should not be able to target creatures with protection from Zombies.
    assert!(!bramble_is_target,
        "Grave Bramble with protection from Zombies should not be targetable by Grimgrin (a Zombie)");
}

/// Bug: Grave Bramble has protection from Zombies, which means
/// Zombies can't block IT (and it can't be targeted/damaged by them).
/// But protection does NOT prevent the protected creature from blocking
/// Zombies — Grave Bramble SHOULD be able to block Zombie attackers.
#[test]
fn bug_protection_incorrectly_prevents_blocking_zombies() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareBlockers, P1);
    state.active_player = P0;

    // Place Grave Bramble for P1 (has Defender + Protection from Zombies)
    let bramble = named_permanent(&mut state, &registry, "Grave Bramble", P1);

    // Place a Zombie attacker for P0
    let zombie = ready_creature(&mut state, P0, 2, 2);
    if let Some(obj) = state.get_object_mut(zombie) {
        obj.subtypes = vec!["Zombie".into()];
    }

    // Set up combat — zombie is attacking
    attacks_unblocked(&mut state, zombie, P1);

    // Grave Bramble should be able to block the Zombie
    // (protection prevents the Zombie from blocking Bramble, NOT the other way around)
    let can_block = mtg_engine::combat::can_block_attacker(&state, bramble, zombie, &registry);

    // BUG: Protection incorrectly prevents Grave Bramble from blocking Zombies
    assert!(can_block,
        "Grave Bramble should be able to block Zombies — protection prevents Zombies from blocking IT, not the reverse");
}

/// Bug: Spare from Evil gives "protection from non-Human creatures."
/// Protection prevents damage from those sources (all damage, not just
/// combat). Non-combat damage from a non-Human creature source should
/// be prevented but may not be.
#[test]
fn bug_spare_from_evil_protection_non_combat_damage() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place a Human creature for P0
    let human = ready_creature(&mut state, P0, 2, 2);
    if let Some(obj) = state.get_object_mut(human) {
        obj.subtypes = vec!["Human".into()];
    }

    // Give it protection from non-Human creatures (Spare from Evil effect)
    // The protection is stored as a TemporaryEffect
    state.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantProtection {
        target: human,
        filter: mtg_engine::types::CreatureFilter::Not(Box::new(
            mtg_engine::types::CreatureFilter::HasSubtype("Human".into())
        )),
    });

    // A non-Human creature deals non-combat damage to the protected creature
    let zombie = ready_creature(&mut state, P1, 3, 3);
    if let Some(obj) = state.get_object_mut(zombie) {
        obj.subtypes = vec!["Zombie".into()];
    }

    // Deal non-combat damage through the engine pipeline (apply_pending_effect).
    // Protection should prevent this damage.
    mtg_engine::engine::apply_pending_effect(
        &mut state,
        &Target::Object(human),
        &mtg_engine::state::PendingEffect::DealDamage {
            amount: 3,
            source_id: zombie,
        },
        &registry,
    );

    // Protection from non-Human creatures should have prevented the damage.
    let damage = state.get_object(human).unwrap().damage_marked;
    assert_eq!(damage, 0,
        "Protection from non-Human creatures should prevent non-combat damage. Got: {damage}");
}
