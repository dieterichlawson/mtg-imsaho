//! Two rules the engine had no way to express, so cards worked around them.
//!
//! `TriggerScope` had only `Each` and `Your`, but a Curse says "at the
//! beginning of ENCHANTED PLAYER's upkeep" — normally an opponent, so neither
//! fits. All three upkeep Curses fell back to `Each` and wrote the same
//! early-return in their handler, which left an inert trigger on the stack
//! during every other player's upkeep (CR 603.2).
//!
//! And equip target generation excluded the creature the equipment was already
//! attached to. CR 702.6a has no such restriction, and re-equipping to the
//! current host is a real play whenever the equip COST is the point.

mod common;

use common::*;
use mtg_engine::actions::{Action, Target};
use mtg_engine::ids::ObjectId;
use mtg_engine::state::{GameState, StackEntry};
use mtg_engine::triggers::{self, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;
const UPKEEP_CURSES: &[&str] = &[
    "Curse of Oblivion",
    "Curse of the Bloody Tome",
    "Curse of the Pierced Heart",
];

fn upkeep_entries(state: &GameState, curse: ObjectId) -> usize {
    state.stack.iter()
        .filter(|e| matches!(e, StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: object_id, .. },
                event: TriggerEvent::Upkeep }) if *object_id == curse))
        .count()
}

/// P0 controls the curse, P1 is enchanted. It fires on P1's upkeep only.
#[test]
fn upkeep_curses_fire_only_on_the_enchanted_players_upkeep() {
    let reg = registry();
    for name in UPKEEP_CURSES {
        for (active, should_fire) in [(P1, true), (P0, false)] {
            let mut state = game_at_step(Step::Upkeep, active);
            let curse = attach_curse_to_player(&mut state, &reg, name, P0, P1);

            state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
            triggers::collect_triggers(&mut state, &reg);

            assert_eq!(upkeep_entries(&state, curse) > 0, should_fire,
                "{name} is on p1; with p{} active its upkeep trigger should{} \
                 be on the stack", active.0, if should_fire { "" } else { " NOT" });
        }
    }
}

/// The scope is declared, not hand-rolled in the handler.
#[test]
fn upkeep_curses_declare_the_attached_player_scope() {
    let reg = registry();
    for name in UPKEEP_CURSES {
        let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown {name}"));
        let scope = reg.get(card_id).unwrap()
            .step_trigger_scope(&mtg_engine::cards::TriggerKind::Upkeep, false);
        assert_eq!(scope, mtg_engine::cards::TriggerScope::AttachedPlayer,
            "{name} triggers on the enchanted player's upkeep, so it must say \
             so through TriggerScope rather than filtering in its handler");
    }
}

/// CR 702.6a: equip may target the creature already wearing the equipment.
#[test]
fn equip_can_target_the_already_equipped_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let gear = named_equipment(&mut state, &reg, "Cobbled Wings", P0);
    let only_creature = named_permanent(&mut state, &reg, "Walking Corpse", P0);
    state.get_object_mut(gear).unwrap().attached_to = Some(only_creature);
    state.get_player_mut(P0).mana_pool.add(ManaType::Colorless, 3);

    let targets: Vec<Target> = mtg_engine::engine::legal_actions(&state, &reg).actions.iter()
        .filter_map(|a| match a {
            Action::ActivateAbility { object_id, targets, .. } if *object_id == gear => Some(targets.clone()),
            _ => None,
        })
        .flatten()
        .collect();

    assert!(targets.contains(&Target::Object(only_creature)),
        "with one creature on the battlefield already wearing Cobbled Wings, \
         equip must still be activatable targeting it — excluding it removed \
         the ability entirely; got {targets:?}");
}
