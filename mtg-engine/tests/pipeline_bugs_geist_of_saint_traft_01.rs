mod common;
use common::*;

use mtg_engine::cards::{AttackInfo, CardRegistry};
use mtg_engine::combat;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

fn setup_geist_attacking(state: &mut mtg_engine::state::GameState, reg: &CardRegistry) -> (mtg_engine::ids::ObjectId, mtg_engine::ids::ObjectId) {
    let geist = named_creature(state, reg, "Geist of Saint Traft", P0);
    state.combat = Some(mtg_engine::state::CombatState {
        attackers: [(geist, P1)].into_iter().collect(),
        blocker_assignments: std::collections::HashMap::new(),
        ..Default::default()
    });
    let behavior = reg.get(state.get_object(geist).unwrap().card_id).unwrap();
    behavior.on_attacks(state, geist, AttackInfo::new(geist, P1), &[], reg);

    let angel_id = state.objects.values()
        .find(|o| o.name == "Angel" && o.zone == Zone::Battlefield)
        .map(|o| o.id)
        .expect("Angel token should exist on battlefield after on_attacks");

    (geist, angel_id)
}

/// Oracle: "Exile that token at end of combat" creates a delayed triggered ability
/// that goes on the stack at the beginning of the end of combat step, giving players
/// priority to respond before the Angel is actually exiled.
///
/// Bug: the exile is implemented as a turn-based action inside combat::end_combat(),
/// which runs before any StepStarted{EndCombat} triggers fire. The Angel is unconditionally
/// exiled with no stack entry and no priority window.
#[test]
fn geist_angel_exile_is_triggered_not_turn_based() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::DeclareAttackers, P0);

    let (_geist, angel_id) = setup_geist_attacking(&mut state, &reg);

    // Simulate the engine's turn-based action that runs when entering the EndCombat step.
    // In the correct implementation this should not exile the Angel — the exile is a
    // triggered ability and must go on the stack first.
    combat::end_combat(&mut state, &reg);

    // Angel should still be on the battlefield: the exile triggered ability has not yet
    // gone on the stack, let alone resolved. Players have not had priority to respond.
    assert_eq!(
        state.get_object(angel_id).unwrap().zone,
        Zone::Battlefield,
        "Angel token should still be on the battlefield after turn-based actions; \
         oracle says exile is a triggered ability that goes on the stack, \
         not an immediate turn-based action that bypasses priority",
    );
}

/// Bug: Geist of Saint Traft registers a TriggerKind::EndCombat triggered ability
/// ("exile the Angel token"). Because on_end_combat is not implemented, this fires
/// as a no-op EndCombatTrigger every end of combat step while Geist is on the
/// battlefield — including turns where Geist did not attack and no Angel token exists.
///
/// Oracle says: the exile trigger only arises because Geist attacked and created a token.
/// On turns where Geist did not attack, no end-of-combat trigger should appear.
#[test]
fn geist_no_spurious_end_combat_trigger_when_did_not_attack() {
    let reg = CardRegistry::with_all_cards();
    let mut state = game_at_step(Step::EndCombat, P0);

    // Geist is on the battlefield but did not attack this turn.
    let _geist = named_creature(&mut state, &reg, "Geist of Saint Traft", P0);

    // Push the StepStarted{EndCombat} event and collect triggers without resolving them.
    // This lets us inspect what was queued before auto-resolution clears the stack.
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::EndCombat });
    mtg_engine::triggers::collect_triggers(&mut state, &reg);

    // No EndCombatTrigger should have been created: Geist didn't attack this turn,
    // so there is no Angel token to exile and no reason for a triggered ability.
    let has_end_combat_trigger = state.stack.iter()
        .filter_map(|e| e.as_trigger())
        .any(|t| matches!(t, PendingTrigger { source: TriggerSource { .. }, event: TriggerEvent::EndCombat }))
        || state.pending_trigger_pushes_ap.iter()
            .any(|t| matches!(t, PendingTrigger {
                source: TriggerSource { .. },
                event: TriggerEvent::EndCombat,
            }))
        || state.pending_trigger_pushes_nap.iter()
            .any(|t| matches!(t, PendingTrigger {
                source: TriggerSource { .. },
                event: TriggerEvent::EndCombat,
            }));

    assert_eq!(
        has_end_combat_trigger,
        false,
        "No EndCombatTrigger should be created for Geist of Saint Traft when it did \
         not attack this turn; the exile trigger only exists when an Angel token was created",
    );
}
