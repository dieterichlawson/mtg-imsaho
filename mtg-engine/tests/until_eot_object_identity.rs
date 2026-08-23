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
