//! Failing tests for bugs documented in `audits/AUDIT_BUGS.md`.
//! Each test is expected to FAIL until the corresponding bug is
//! fixed. Once the fix lands the test transitions from "proves the
//! bug exists" to "regression-protects against the bug coming back".
//!
//! This file covers the "Damage helper bypass — non-combat damage
//! skips central pipeline" family. The root pattern is Bug 9F-002:
//! several damage sources inline `obj.damage_marked += N` instead of
//! routing through the central `PendingEffect::DealDamage` helper,
//! which means each one independently has to remember to push
//! `damaged_by`, decrement planeswalker loyalty, check protection,
//! etc. The narrower bugs in this file expose specific consequences.
//!
//! Bugs covered in this file:
//! - Bug T: Skirsdag Cultist and Rolling Temblor don't push to
//!   `obj.damaged_by` when they apply damage
//! - Bug BQ: `TargetRequirement::AnyTarget` enumeration filters by
//!   `o.power.is_some()` so planeswalkers can't be chosen as the
//!   "any target" — affects Brimstone Volley, Devil's Play, etc.
//! - Bug BZ: `cards/helpers.rs::any_targets` (the on-resolve enumerator
//!   used by Pitchburn Devils' death trigger) omits planeswalkers for
//!   the same reason
//! - Bug BR: Olivia Voldaren's first ability (`obj.damage_marked += 1`)
//!   and Curse of the Pierced Heart's life-subtract bypass the central
//!   damage helper and don't push source to `damaged_by`

mod common;
use common::*;

use mtg_engine::actions::{Action, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::types::*;

/// Bug T (`audits/AUDIT_BUGS.md)`: Skirsdag Cultist's activated-ability
/// damage doesn't push the source onto the target's `damaged_by` vector.
///
/// Oracle (Skirsdag Cultist): "{R}, {T}, Sacrifice a creature: Skirsdag
/// Cultist deals 2 damage to any target."
///
/// Failure mode: `skirsdag_cultist.rs:54-58` does
/// `obj.damage_marked += 2` directly, without the
/// `obj.damaged_by.push(source_id)` that the rest of the codebase pairs
/// it with. `damaged_by` is consulted by SBA 704.5h (deathtouch
/// destruction) and by death-watch triggers that care about who killed
/// what — leaving it stale silently breaks both. Compare to Olivia
/// Voldaren's first-ability bite (`olivia_voldaren.rs:104-106`) which
/// pushes correctly.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug T is fixed.
#[test]
fn bug_t_skirsdag_cultist_pushes_damaged_by() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let cultist = named_creature(&mut state, &registry, "Skirsdag Cultist", P0);
    let victim = ready_creature(&mut state, P1, 3, 3);

    // Drive the activated ability directly. We don't care about cost
    // payment for this test — only the damage code path.
    let cultist_card_id = state.get_object(cultist).unwrap().card_id;
    let behavior = registry.get(cultist_card_id).unwrap();
    behavior.on_activate_ability(
        &mut state,
        cultist,
        0,
        &[Target::Object(victim)],
        &registry,
    );

    let victim_obj = state.get_object(victim).unwrap();
    assert!(
        victim_obj.damage_marked >= 2,
        "Test setup: Skirsdag Cultist should have written 2 damage to \
         the victim, but damage_marked = {}",
        victim_obj.damage_marked
    );
    assert!(
        victim_obj.damaged_by.contains(&cultist),
        "Skirsdag Cultist should push its ObjectId onto the victim's \
         damaged_by vector when dealing damage. Bug T: the ability \
         increments damage_marked directly without touching damaged_by, \
         so deathtouch SBAs and death-watch triggers can't see who \
         dealt the lethal damage. damaged_by = {:?}",
        victim_obj.damaged_by,
    );
}

/// Bug T (`audits/AUDIT_BUGS.md)`: Rolling Temblor doesn't push itself
/// onto each affected creature's `damaged_by` vector.
///
/// Oracle (Rolling Temblor): "Rolling Temblor deals 2 damage to each
/// creature without flying."
///
/// Failure mode: `rolling_temblor.rs:36-39` writes
/// `obj.damage_marked += 2` for each non-flying creature without the
/// matching `obj.damaged_by.push(source)`.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug T is fixed.
#[test]
fn bug_t_rolling_temblor_pushes_damaged_by() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let _on_p0 = ready_creature(&mut state, P0, 2, 2);
    let _on_p1 = ready_creature(&mut state, P1, 2, 2);

    // Resolve Rolling Temblor by calling its on_resolve directly.
    let temblor_card_id = registry.get_id_by_name("Rolling Temblor").unwrap();
    let temblor = state.create_object(temblor_card_id, P0, Zone::Stack, None, None);
    state.get_object_mut(temblor).unwrap().name = "Rolling Temblor".into();
    let behavior = registry.get(temblor_card_id).unwrap();
    behavior.on_resolve(&mut state, temblor, &[], &registry);

    // Every non-flying creature should have damage_marked >= 2 AND
    // should list temblor in its damaged_by.
    let creatures: Vec<_> = state
        .objects
        .values()
        .filter(|o| o.zone == Zone::Battlefield && o.power.is_some())
        .map(|o| (o.id, o.damage_marked, o.damaged_by.clone()))
        .collect();
    assert!(!creatures.is_empty(), "Test setup: should have creatures in play");

    for (id, damage, damaged_by) in &creatures {
        assert!(
            *damage >= 2,
            "Test setup: creature {id:?} should have damage_marked >= 2"
        );
        assert!(
            damaged_by.contains(&temblor),
            "Rolling Temblor should push itself onto creature {id:?}'s \
             damaged_by vector when dealing damage. Bug T: the on_resolve \
             handler increments damage_marked directly without touching \
             damaged_by. damaged_by = {damaged_by:?}",
        );
    }
}

/// Bug BQ (`audits/AUDIT_BUGS.md)`: `TargetRequirement::AnyTarget`
/// filters battlefield objects with `o.power.is_some()`, which
/// excludes planeswalkers. So Brimstone Volley, Devil's Play,
/// Geistflame, Skirsdag Cultist, Blazing Torch, and Heretic's
/// Punishment can't choose a planeswalker as their "any target".
///
/// Oracle (Brimstone Volley): "Brimstone Volley deals 3 damage to any
/// target. Morbid — Brimstone Volley deals 5 damage instead if a
/// creature died this turn."
///
/// Per CR 115.4a, "any target" is the modern oracle phrasing that
/// means "any creature, player, planeswalker, or battle". A
/// planeswalker should always be a legal target for these spells.
///
/// Failure mode: `engine.rs:1378-1393` (the cast-time arm) and
/// `engine.rs:1890+` (the activated-ability arm) both filter via
/// `obj.power.is_some()`. Garruk Relentless has `power = None` in the
/// registry, so the filter drops him. The "any target" prompt only
/// lists creatures + players.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BQ is fixed.
#[test]
fn bug_bq_brimstone_volley_can_target_planeswalker() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Garruk Relentless on P1's side. He's a Planeswalker, so
    // obj.power = None — exactly the case the AnyTarget filter drops.
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(garruk).unwrap().name = "Garruk Relentless".into();
    state
        .get_object_mut(garruk)
        .unwrap()
        .card_types = vec![CardType::Planeswalker];

    // Brimstone Volley in P0's hand with mana ready.
    let volley = castable_spell(&mut state, &registry, "Brimstone Volley", P0);

    let legal = engine::legal_actions(&state, &registry);
    let can_target_garruk = legal.actions.iter().any(|a| matches!(
        a,
        Action::CastSpell { object_id, targets, .. }
            if *object_id == volley
                && targets.iter().any(|t| matches!(t, Target::Object(id) if *id == garruk))
    ));

    assert!(
        can_target_garruk,
        "Brimstone Volley's 'any target' should include Garruk Relentless \
         (a planeswalker on the battlefield). Bug BQ: \
         TargetRequirement::AnyTarget filters by o.power.is_some() and \
         drops planeswalkers. legal_actions = {:?}",
        legal
            .actions
            .iter()
            .filter(|a| matches!(a, Action::CastSpell { object_id, .. } if *object_id == volley))
            .collect::<Vec<_>>(),
    );
}

/// Bug BZ (`audits/AUDIT_BUGS.md)`: `cards/helpers.rs::any_targets`
/// builds its target list from `creature_targets` + players, omitting
/// planeswalkers. This is the on-resolve sibling of Bug BQ — Pitchburn
/// Devils' on-death trigger uses this helper to enumerate targets, so
/// the model is never offered Garruk or Liliana as a death-trigger
/// target.
///
/// Oracle (Pitchburn Devils): "When this creature dies, it deals 3
/// damage to any target."
///
/// Failure mode: `cards/helpers.rs:182-187` does
/// `let mut targets = creature_targets(state); for player in
/// &state.players { ... }` and `creature_targets` filters with
/// `o.power.is_some()`. Planeswalkers are dropped. Pitchburn Devils'
/// `on_dies` (at `pitchburn_devils.rs:37`) calls `any_targets(state)`
/// and feeds the result into `present_target_choice` — the
/// `awaiting_action`'s options end up missing the planeswalker.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BZ is fixed.
#[test]
fn bug_bz_pitchburn_devils_offers_planeswalker_as_target() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Garruk Relentless on P1's side.
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P1, Zone::Battlefield, None, None);
    state.get_object_mut(garruk).unwrap().name = "Garruk Relentless".into();
    state
        .get_object_mut(garruk)
        .unwrap()
        .card_types = vec![CardType::Planeswalker];

    // Pitchburn Devils on P0's side. Drive the death via SBA so the
    // SelfDies trigger enters the pipeline with stack-time targeting.
    let pd = named_creature(&mut state, &registry, "Pitchburn Devils", P0);
    state.get_object_mut(pd).unwrap().damage_marked = 3;
    state.events.clear();
    mtg_engine::sba::check_state_based_actions(&mut state, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let garruk_in_options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. },
            ..
        }) => options
            .iter()
            .any(|t| matches!(t, Target::Object(id) if *id == garruk)),
        _ => false,
    };

    assert!(
        garruk_in_options,
        "Pitchburn Devils' on-death 'deal 3 damage to any target' should \
         offer Garruk Relentless as a target. Bug BZ: any_targets() builds \
         from creature_targets() (filtered by o.power.is_some()) plus \
         players, so planeswalkers are dropped. awaiting_action = {:?}",
        state.awaiting_action,
    );
}

/// Bug BR (`audits/AUDIT_BUGS.md)`: Olivia Voldaren's first ability
/// (`obj.damage_marked += 1`) bypasses the central damage helper. If
/// Olivia is pointed at a planeswalker, the code writes
/// `damage_marked` instead of decrementing the planeswalker's
/// loyalty counters. The central helper at
/// `engine.rs:2854-2877` handles the planeswalker branch.
///
/// Oracle (Olivia Voldaren, first ability): "{1}{R}: This creature
/// deals 1 damage to another target creature. That creature becomes
/// a Vampire in addition to its other types."
///
/// Failure mode: `olivia_voldaren.rs:104-106` inline-writes
/// `obj.damage_marked += 1; obj.damaged_by.push(object_id);` without
/// checking whether the target is a planeswalker. We force the
/// issue by manually calling `on_activate_ability` with a
/// planeswalker target — even though `TargetFilter::Another` / the
/// activated-ability target enumerator wouldn't ordinarily surface a
/// planeswalker (Bug BQ), the engine has no defense-in-depth at the
/// handler level. After firing, Garruk's `damage_marked` should be
/// 0 and his loyalty counters should be 1 less than before.
///
/// This test asserts the EXPECTED CORRECT behavior, so it currently
/// fails. It will start passing as soon as Bug BR is fixed.
#[test]
fn bug_br_olivia_damage_decrements_planeswalker_loyalty() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_creature(&mut state, &registry, "Olivia Voldaren", P0);

    // Garruk Relentless on P1's side with a starting loyalty of 3.
    let garruk_card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
    let garruk = state.create_object(garruk_card_id, P1, Zone::Battlefield, None, None);
    {
        let obj = state.get_object_mut(garruk).unwrap();
        obj.name = "Garruk Relentless".into();
        obj.card_types = vec![CardType::Planeswalker];
        obj.counters.insert(CounterType::Loyalty, 3);
    }

    // Fire Olivia's first ability targeting Garruk.
    let olivia_card_id = registry.get_id_by_name("Olivia Voldaren").unwrap();
    let behavior = registry.get(olivia_card_id).unwrap();
    behavior.on_activate_ability(
        &mut state,
        olivia,
        0,
        &[Target::Object(garruk)],
        &registry,
    );

    let loyalty = counters_of(&state, garruk, CounterType::Loyalty);
    let damage_marked = state.get_object(garruk).unwrap().damage_marked;

    assert!(
        loyalty <= 2 && damage_marked == 0,
        "Olivia Voldaren's 1 damage to a planeswalker should decrement \
         the planeswalker's loyalty counters, not write to damage_marked. \
         Bug BR: the handler inline-writes obj.damage_marked without \
         checking if the target is a planeswalker. \
         loyalty={loyalty}, damage_marked={damage_marked}",
    );
}
