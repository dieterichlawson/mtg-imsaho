//! Regression tests for Bug A (`BUG_REPORT_8SEAT.md)`: cards without a
//! `SelfDies` or `LeavesBattlefield` `TriggeredAbilityDef` were pushing empty
//! triggers onto the stack whenever they left the battlefield, polluting
//! the LLM prompt with no-op [RESPOND TO ...] cycles.

mod common;
use common::*;

use mtg_engine::cards::{CardRegistry, TriggerKind};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::{Step, Zone};

const P0: PlayerId = PlayerId(0);

/// Helper: kill a battlefield creature via lethal damage + SBA.
fn kill_creature(state: &mut mtg_engine::state::GameState, registry: &CardRegistry, id: mtg_engine::ids::ObjectId) {
    state.get_object_mut(id).unwrap().damage_marked = 99;
    mtg_engine::sba::check_state_based_actions(state, registry);
}

/// The whole set at once: a creature that declares no `SelfDies`,
/// `LeavesBattlefield` or `AnyCreatureDies` ability puts nothing on the stack
/// when it dies. Naming three vanilla creatures one test each covered three
/// cards; this covers every one that qualifies, including the next one added.
#[test]
fn no_creature_without_a_death_ability_puts_a_trigger_on_the_stack_when_it_dies() {
    let registry = CardRegistry::with_all_cards();

    let silent: Vec<String> = registry
        .all_names()
        .iter()
        .filter(|name| {
            let Some(id) = registry.get_id_by_name(name) else { return false };
            let Some(data) = registry.card_data(id) else { return false };
            data.card_types.contains(&mtg_engine::types::CardType::Creature)
                && !data.triggered_abilities.iter().any(|t| matches!(
                    t.kind,
                    TriggerKind::SelfDies
                        | TriggerKind::LeavesBattlefield
                        | TriggerKind::AnyCreatureDies
                ))
        })
        .map(|n| (*n).to_string())
        .collect();
    assert!(silent.len() >= 60,
        "only {} creatures have no death ability — this sweep has stopped covering the set",
        silent.len());

    for name in &silent {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let id = named_permanent(&mut state, &registry, name, P0);
        kill_creature(&mut state, &registry, id);
        mtg_engine::triggers::collect_triggers(&mut state, &registry);

        let triggers: Vec<_> = state.stack.iter()
            .filter_map(|e| match e {
                StackEntry::Trigger(t) => Some(format!("{:?}", t.event)),
                _ => None,
            })
            .collect();
        assert!(triggers.is_empty(),
            "{name} declares no death or leaves-the-battlefield ability, but dying \
             put {triggers:?} on the stack");
    }
}

#[test]
fn aura_leaving_battlefield_creates_no_ltb_trigger() {
    // Oracle: Dead Weight is an aura with no LTB text. Moving it off the
    // battlefield should not push an LTB trigger.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let target = named_permanent(&mut state, &registry, "Grizzly Bears", P0);
    let aura = named_permanent(&mut state, &registry, "Dead Weight", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(target);

    // Move aura off battlefield (simulates destruction).
    state.move_object(aura, Zone::Graveyard, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let ltb_count = state.stack.iter().filter(|e|
        matches!(e, StackEntry::Trigger(PendingTrigger {
            source: TriggerSource { .. },
            event: TriggerEvent::LeftBattlefield,
        }))
    ).count();
    assert_eq!(ltb_count, 0,
        "Dead Weight has no LTB text per oracle; LTB trigger should not be created");
}

#[test]
fn fiend_hunter_ltb_trigger_still_fires() {
    // Oracle: "When this creature leaves the battlefield, return the exiled
    // card to the battlefield under its owner's control."
    // Regression in the opposite direction: the gate must not break this.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hunter = named_permanent(&mut state, &registry, "Fiend Hunter", P0);

    state.move_object(hunter, Zone::Graveyard, &registry);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let has_ltb = state.stack.iter().any(|e|
        matches!(e, StackEntry::Trigger(PendingTrigger {
            source: TriggerSource { .. },
            event: TriggerEvent::LeftBattlefield,
        }))
    );
    assert!(has_ltb, "Fiend Hunter's LTB trigger must still fire");
}

#[test]
fn doomed_traveler_selfdies_trigger_still_fires() {
    // Oracle: "When this creature dies, create a 1/1 white Spirit creature
    // token with flying." Gate must not break this.
    let registry = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let traveler = named_permanent(&mut state, &registry, "Doomed Traveler", P0);

    kill_creature(&mut state, &registry, traveler);
    mtg_engine::triggers::collect_triggers(&mut state, &registry);

    let has_dies = state.stack.iter().any(|e|
        matches!(e, StackEntry::Trigger(PendingTrigger {
            source: TriggerSource { .. },
            event: TriggerEvent::SelfDies,
        }))
    );
    assert!(has_dies, "Doomed Traveler's dies trigger must still fire");
}
