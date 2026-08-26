//! CR 115.4a: "any target" means any creature, player, planeswalker or battle.
//!
//! The engine enumerated battlefield targets with `o.power.is_some()`, which is
//! true of creatures and false of planeswalkers, so every "any target" spell and
//! ability in the set quietly refused to point at one. It was wrong in two
//! places — the cast-time enumerator and `cards/helpers.rs::any_targets`, which
//! is what an ability uses when it picks its target on resolution — so both
//! paths are checked here.
//!
//! This file used to be five per-card regressions about damage bypassing the
//! central pipeline. That rule is a build-failing source guard now
//! (`test_suite_guards.rs::only_the_damage_pipeline_marks_damage`), and what
//! the pipeline does with the damage is in `inline_damage.rs`; what is left is
//! the targeting half, swept across every card that says "any target".

mod common;
use common::*;

use mtg_engine::actions::Target;
use mtg_engine::cards::TargetRequirement;
use mtg_engine::types::*;

/// Put a planeswalker on `owner`'s battlefield. A real one from the registry,
/// because the bug was that `power` is `None` for planeswalkers and the
/// enumerator filtered on it.
fn planeswalker(state: &mut mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry, owner: PlayerId) -> ObjectId {
    let id = named_permanent(state, reg, "Garruk Relentless", owner);
    assert!(state.get_object(id).unwrap().power.is_none(),
        "test precondition: a planeswalker has no power, which is what the \
         broken filter keyed on");
    id
}

/// Every card in the set whose declared target requirement is `AnyTarget` must
/// offer a planeswalker when one is on the battlefield.
///
/// Derived from the registry rather than hand-listed: a new "any target" card
/// is covered the day it is added, and the floor makes a sweep that stops
/// finding anything fail rather than pass silently.
#[test]
fn every_any_target_spell_can_point_at_a_planeswalker() {
    let reg = registry();
    let mut checked = 0;

    let mut names: Vec<String> = reg.all_names().iter().map(|s| (*s).to_string()).collect();
    names.sort();
    for name in names {
        let card_id = reg.get_id_by_name(&name).expect("named card has an id");
        let Some(behavior) = reg.get(card_id) else { continue };
        if !matches!(behavior.target_requirement(), TargetRequirement::AnyTarget) {
            continue;
        }
        let data = behavior.card_data();
        // Only the ones castable as a spell go through the cast-time
        // enumerator; the rest are activated or triggered abilities.
        if data.cost.is_none() || data.card_types.iter().all(|t|
            !matches!(t, CardType::Instant | CardType::Sorcery)) {
            continue;
        }

        let mut state = game_at_step(Step::PrecombatMain, P0);
        let garruk = planeswalker(&mut state, &reg, P1);
        let spell = castable_spell(&mut state, &reg, &data.name, P0);

        let offered = offered_targets(&state, &reg, spell);
        assert!(offered.contains(&Target::Object(garruk)),
            "{}: 'any target' includes a planeswalker (CR 115.4a); offered {offered:?}",
            data.name);
        checked += 1;
    }

    assert!(checked >= 3,
        "expected the set's 'any target' spells to be found and checked, got {checked}");
}

/// The same rule on the resolution-time path: Pitchburn Devils' death trigger
/// enumerates its own targets through `cards/helpers.rs::any_targets`, which
/// was a separate copy of the same filter.
#[test]
fn an_ability_that_picks_any_target_on_resolution_offers_a_planeswalker() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let garruk = planeswalker(&mut state, &reg, P1);
    let devils = named_permanent(&mut state, &reg, "Pitchburn Devils", P0);

    kill_by_damage(&mut state, &reg, devils);
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    let options = match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice {
            choice: ResolutionChoiceKind::ChooseTarget { options, .. }, ..
        }) => options.clone(),
        other => panic!("Pitchburn Devils should ask where its 3 damage goes, got {other:?}"),
    };

    assert!(options.iter().any(|t| matches!(t, Target::Object(id) if *id == garruk)),
        "'it deals 3 damage to any target' includes a planeswalker; offered {options:?}");
}

/// And the damage, once pointed there, removes loyalty (CR 120.3c) — the
/// planeswalker branch of the pipeline, reached from an activated ability
/// rather than a spell.
#[test]
fn damage_from_an_activated_ability_takes_a_planeswalkers_loyalty() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let olivia = named_permanent(&mut state, &reg, "Olivia Voldaren", P0);
    let garruk = planeswalker(&mut state, &reg, P1);
    let before = counters_of(&state, garruk, CounterType::Loyalty);
    assert!(before > 0, "test precondition: Garruk entered with loyalty");

    add_mana(&mut state, P0, &[(ManaType::Colorless, 1), (ManaType::Red, 1)]);
    let state = activate(&state, &reg, olivia, 0, vec![Target::Object(garruk)]);

    assert_eq!(counters_of(&state, garruk, CounterType::Loyalty), before - 1,
        "Olivia's 1 damage removes one loyalty counter");
    assert_eq!(state.get_object(garruk).unwrap().damage_marked, 0,
        "and marks no damage on the permanent, which nothing would ever clear");
}
