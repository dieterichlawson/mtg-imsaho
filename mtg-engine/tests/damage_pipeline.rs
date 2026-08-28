//! Regression tests for the unified damage pipeline.
//!
//! Historically each damage path hand-rolled its own subset of checks:
//! fight damage skipped Unbreathing Horde's prevent-and-remove-counter
//! replacement, and noncombat damage skipped the deathtouch flag and
//! player-damage lifelink. All damage now flows through
//! `mtg_engine::damage::deal_damage`.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::damage::{deal_damage, DamageKind};
use mtg_engine::engine;
use mtg_engine::events::DamageTarget;
use mtg_engine::types::*;
use mtg_engine::sba::check_state_based_actions;

/// Fight damage is noncombat damage and must respect Unbreathing Horde's
/// "prevent that damage, remove a +1/+1 counter" replacement (CR 614.1a).
#[test]
fn fight_damage_respects_prevent_damage_remove_counter() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let horde = named_permanent(&mut state, &reg, "Unbreathing Horde", P0);
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

// -------------------------------------------------------------------------
// From the bug-audit files, re-filed by the rule each one exercises.
// -------------------------------------------------------------------------

/// Bug: `PendingEffect::DealDamage` marks `damage_marked` on planeswalkers
/// instead of removing loyalty counters.
/// Planeswalkers take damage as loyalty counter removal, not as `damage_marked`.
#[test]
fn bug_planeswalker_damage_uses_damage_marked_not_loyalty() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Place Garruk Relentless (starting loyalty 3) for P1
    let garruk = {
        let card_id = registry.get_id_by_name("Garruk Relentless").unwrap();
        let id = state.create_object(card_id, P1, Zone::Battlefield, None, None);
        let obj = state.get_object_mut(id).unwrap();
        obj.name = "Garruk Relentless".into();
        obj.summoning_sick = false;
        // Set loyalty counters
        state.add_counters(id, CounterType::Loyalty, 3);
        id
    };

    // Verify starting loyalty
    let loyalty_before = state.get_counter_count(garruk, CounterType::Loyalty);
    assert_eq!(loyalty_before, 3, "Garruk should start with 3 loyalty");

    // Deal 2 damage to the planeswalker via DealDamage pending effect
    // (simulating Curse of the Pierced Heart or similar)
    engine::apply_pending_effect(
        &mut state,
        &Target::Object(garruk),
        &mtg_engine::state::PendingEffect::DealDamage { source_id: garruk, amount: 2 },
        &registry,
    );

    // Loyalty should decrease by 2 (3 -> 1)
    let loyalty_after = state.get_counter_count(garruk, CounterType::Loyalty);

    // BUG: Loyalty is still 3 because DealDamage adds to damage_marked
    // instead of removing loyalty counters
    assert_eq!(loyalty_after, 1,
        "Planeswalker should lose loyalty from damage. Loyalty: {loyalty_after} (expected 1)");
}

/// Bug: Prey Upon uses `CombatDamageDealt` instead of `NonCombatDamageDealt`.
/// Fight damage is NOT combat damage per MTG rules.
#[test]
fn bug_prey_upon_uses_combat_damage_for_fight() {
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let my_creature = ready_creature(&mut state, P0, 3, 3);
    let their_creature = ready_creature(&mut state, P1, 2, 2);

    // Cast Prey Upon
    let prey = castable_spell(&mut state, &registry, "Prey Upon", P0);
    state = cast_and_resolve(&state, &registry, prey,
        vec![Target::Object(my_creature), Target::Object(their_creature)]);

    // Check events — fight damage should be NonCombatDamageDealt
    let has_combat_damage = state.events.iter().any(|e| {
        matches!(e, mtg_engine::events::GameEvent::CombatDamageDealt { .. })
    });
    let has_non_combat_damage = state.events.iter().any(|e| {
        matches!(e, mtg_engine::events::GameEvent::NonCombatDamageDealt { .. })
    });

    // BUG: Fight emits CombatDamageDealt instead of NonCombatDamageDealt
    assert!(!has_combat_damage,
        "Fight damage should NOT emit CombatDamageDealt");
    assert!(has_non_combat_damage,
        "Fight damage should emit NonCombatDamageDealt");
}

// -------------------------------------------------------------------------
// Marked damage is not a toughness reduction
// -------------------------------------------------------------------------

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

/// CR 120.8: "If a source would deal 0 damage, it does not deal damage at
/// all." Nothing that would trigger on damage being dealt triggers, and
/// nothing that reads what damaged a creature sees the source.
///
/// The rule lives in `deal_damage`'s guard rather than in each card. Harvest
/// Pyre reaches it with a real zero — its X is the number of cards exiled, and
/// X=0 is a legal cast — and used to carry a `count > 0` guard of its own,
/// which meant the card was right and the rule was untested.
#[test]
fn zero_damage_is_not_damage() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let source = ready_creature(&mut state, P0, 2, 2);
    let target = ready_creature(&mut state, P1, 3, 3);

    state.events.clear();
    deal_damage(&mut state, source, DamageTarget::Object(target), 0, DamageKind::NonCombat, &reg);

    assert_eq!(state.get_object(target).unwrap().damage_marked, 0);
    assert!(state.get_object(target).unwrap().damaged_by.is_empty(),
        "a creature dealt no damage was not damaged by anything");
    assert!(!state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::NonCombatDamageDealt { .. })),
        "and no damage event is emitted for anything to trigger on");

    // The same through a card: Harvest Pyre cast for X=0.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let victim = ready_creature(&mut state, P1, 3, 3);
    let pyre = castable_spell(&mut state, &reg, "Harvest Pyre", P0);
    let mut state = engine::submit_action(
        &state,
        &mtg_engine::actions::Action::CastSpell {
            object_id: pyre,
            targets: vec![Target::Object(victim)],
            sacrifice: None, exile_count: Some(0), exile_ids: vec![],
            alternative_cost: None, tap_plan: vec![],
        },
        &reg,
    );
    state.events.clear();
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(victim).unwrap().damage_marked, 0);
    assert!(state.get_object(victim).unwrap().damaged_by.is_empty(),
        "X=0 deals no damage, so nothing damaged it");
    assert!(state.events.iter().any(|e| matches!(e,
        mtg_engine::events::GameEvent::SpellResolved { object } if *object == pyre)),
        "the spell still resolves — it is not countered, it just does nothing");
}
