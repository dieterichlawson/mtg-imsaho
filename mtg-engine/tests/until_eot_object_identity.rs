//! Regression test for CR 400.7 object identity vs until-end-of-turn effects.
//!
//! This engine keeps a stable ObjectId across zone changes (bumping
//! zone_change_count). An until-end-of-turn effect keyed by ObjectId must not
//! re-apply to a *new* object that reuses the id after a same-turn zone
//! change — a creature that dies and returns is a new object with no relation
//! to the buffed one.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::TemporaryEffect;
use mtg_engine::types::*;

fn reg() -> CardRegistry {
    CardRegistry::with_all_cards()
}

#[test]
fn until_eot_pt_buff_does_not_survive_death_and_return() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let creature = ready_creature(&mut state, P0, 2, 2);
    // "+3/+3 until end of turn" (as Giant Growth-style pump would add).
    state.until_end_of_turn.push(TemporaryEffect::ModifyPT {
        target: creature,
        power_mod: 3,
        toughness_mod: 3,
    });
    assert_eq!(state.effective_power(creature, &r), Some(5));

    // The creature dies, then returns the same turn (same ObjectId — this
    // engine reuses ids across zone changes).
    state.move_object(creature, Zone::Graveyard, &r);
    state.move_object(creature, Zone::Battlefield, &r);

    // The returned object is new (CR 400.7): the buff must not re-apply.
    assert_eq!(state.effective_power(creature, &r), Some(2),
        "until-end-of-turn P/T buff must not survive a zone change");
    assert!(state.until_end_of_turn.is_empty(),
        "the stale effect targeting the departed object should have been dropped");
}

#[test]
fn until_eot_effect_on_a_different_creature_is_kept() {
    let r = reg();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let a = ready_creature(&mut state, P0, 2, 2);
    let b = ready_creature(&mut state, P0, 2, 2);
    state.until_end_of_turn.push(TemporaryEffect::ModifyPT {
        target: b, power_mod: 1, toughness_mod: 1,
    });

    // A different creature leaving must not drop B's buff.
    state.move_object(a, Zone::Graveyard, &r);

    assert_eq!(state.effective_power(b, &r), Some(3),
        "an unrelated creature leaving must not clear another's until-EOT buff");
    assert_eq!(state.until_end_of_turn.len(), 1);
}

/// An ability that resolves after its source has died applies to nothing:
/// the card in the graveyard is a new object (CR 400.7), so the pump must
/// not be recorded against it — reanimated this turn, it would come back
/// pumped. Found by fuzzing: Feral Ridgewolf's +2/+0 resolving after Smite
/// the Monstrous had killed it.
#[test]
fn a_pump_resolving_after_its_source_died_does_not_follow_the_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wolf = named_permanent(&mut state, &reg, "Feral Ridgewolf", P0);
    add_mana(&mut state, P0, &[(ManaType::Red, 1), (ManaType::Colorless, 1)]);
    let mut state = activate_onto_stack(&state, &reg, wolf, None);
    assert!(matches!(state.stack.last(), Some(mtg_engine::state::StackEntry::Ability { .. })), "test precondition");

    mtg_engine::destruction::try_destroy(&mut state, wolf, &reg);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert!(state.until_end_of_turn.is_empty(),
        "no effect is recorded for a permanent that is gone: {:?}", state.until_end_of_turn);
    state.move_object(wolf, Zone::Battlefield, &reg);
    assert_eq!(state.effective_power(wolf, &reg), Some(1),
        "the reanimated wolf is a new object with its printed power");
}
