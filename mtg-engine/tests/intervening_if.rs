//! Regression tests for intervening-if trigger conditions (CR 603.4).
//!
//! "At the beginning of each upkeep, **if** no spells were cast last turn,
//! transform this creature" is an intervening-if clause: the condition is
//! checked when the ability *would* trigger. If it's false, the ability never
//! goes on the stack at all.
//!
//! The dispatch in `collect_triggers` used to queue an `UpkeepTrigger` for
//! every permanent whose active face had a non-empty upkeep description, and
//! left the condition to `on_upkeep` at resolution. The *outcome* was right —
//! nothing transformed — but a phantom trigger sat on the stack and handed
//! every player a priority window that CR 603.4 says shouldn't exist. That
//! window is observable game state: it's a chance to cast an instant.
//!
//! These tests assert on the queue after `collect_triggers` rather than on
//! the post-resolution board, because the board was already correct.

mod common;
use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::state::{GameState, StackEntry};
use mtg_engine::triggers::{self, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// Dispatch a beginning-of-upkeep event and count the stack entries `object`
/// put there. `collect_triggers` runs the whole dispatch path — including
/// `process_pending_trigger_pushes`, which drains the APNAP queues onto the
/// stack — but stops short of resolving anything. So this counts exactly what
/// the ticket is about: whether a stack entry exists to hold priority open.
fn upkeep_stack_entries(state: &mut GameState, reg: &CardRegistry, object: ObjectId) -> usize {
    state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::Upkeep });
    triggers::collect_triggers(state, reg);
    state.stack.iter()
        .filter(|e| matches!(e, StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: object_id, .. },
                event: TriggerEvent::Upkeep }) if *object_id == object))
        .count()
}

/// Put a werewolf on the battlefield already transformed to its back face.
fn transformed(state: &mut GameState, reg: &CardRegistry, name: &str, owner: PlayerId) -> ObjectId {
    let id = named_creature(state, reg, name, owner);
    state.get_object_mut(id).unwrap().is_transformed = true;
    id
}

// ── Front face: "if no spells were cast last turn" ────────────────

#[test]
fn gatstaf_shepherd_front_trigger_skipped_when_spell_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.num_spells_cast_last_turn.insert(P0, 1);
    let shepherd = named_creature(&mut state, &reg, "Gatstaf Shepherd", P0);

    assert_eq!(upkeep_stack_entries(&mut state, &reg, shepherd), 0,
        "a spell was cast last turn, so the intervening-if is false and the \
         transform ability must not trigger at all (CR 603.4)");
}

#[test]
fn gatstaf_shepherd_front_trigger_fires_when_no_spells_cast() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    let shepherd = named_creature(&mut state, &reg, "Gatstaf Shepherd", P0);

    assert_eq!(upkeep_stack_entries(&mut state, &reg, shepherd), 1,
        "no spells were cast last turn, so the ability must still trigger \
         normally — the gate must not suppress legitimate triggers");
}

// ── Back face: "if a player cast two or more spells last turn" ────

#[test]
fn gatstaf_howler_back_trigger_skipped_when_one_spell_per_player() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    // Two spells were cast last turn, but no *single* player cast two.
    state.num_spells_cast_last_turn.insert(P0, 1);
    state.num_spells_cast_last_turn.insert(P1, 1);
    let howler = transformed(&mut state, &reg, "Gatstaf Shepherd", P0);

    assert_eq!(upkeep_stack_entries(&mut state, &reg, howler), 0,
        "the back face needs one player to have cast two or more spells; one \
         each does not satisfy it, so nothing may go on the stack");
}

#[test]
fn gatstaf_howler_back_trigger_fires_when_a_player_cast_two() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.num_spells_cast_last_turn.insert(P1, 2);
    let howler = transformed(&mut state, &reg, "Gatstaf Shepherd", P0);

    assert_eq!(upkeep_stack_entries(&mut state, &reg, howler), 1,
        "one player cast two spells, so the back face's ability must trigger");
}

// ── The gate is per-trigger-kind, not per-card ────────────────────

/// Howlpack Alpha (Mayor of Avabruck's back face) has *two* triggers: the
/// conditional upkeep transform and an unconditional "at the beginning of your
/// end step, create a 2/2 Wolf". Gating the card must not silence the Wolf.
#[test]
fn howlpack_alpha_end_step_trigger_is_not_gated() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);
    // No player cast two or more spells: the *upkeep* condition is false.
    state.num_spells_cast_last_turn.insert(P0, 1);
    let alpha = transformed(&mut state, &reg, "Mayor of Avabruck", P0);
    assert_eq!(count_tokens_named(&state, "Wolf"), 0);

    fire_step_trigger(&mut state, Step::EndStep, &reg);

    assert_eq!(count_tokens_named(&state, "Wolf"), 1,
        "the unconditional end-step trigger on the same face must still fire; \
         object {alpha:?}");
}

// ── The whole cluster, not just the exemplar ──────────────────────

/// Every werewolf DFC in the set carries the same intervening-if clause, and
/// the fix is one shared helper — so assert the whole family, front and back.
#[test]
fn every_werewolf_upkeep_trigger_respects_its_intervening_if() {
    let reg = registry();
    let werewolves = [
        "Daybreak Ranger", "Gatstaf Shepherd", "Grizzled Outcasts",
        "Hanweir Watchkeep", "Instigator Gang", "Kruin Outlaw",
        "Mayor of Avabruck", "Reckless Waif", "Tormented Pariah",
        "Ulvenwald Mystics", "Village Ironsmith", "Villagers of Estwald",
    ];

    for name in werewolves {
        // Front face, condition false: a spell was cast last turn.
        let mut state = game_at_step(Step::Upkeep, P0);
        state.num_spells_cast_last_turn.insert(P0, 1);
        let front = named_creature(&mut state, &reg, name, P0);
        assert_eq!(upkeep_stack_entries(&mut state, &reg, front), 0,
            "{name} (front) must not trigger when a spell was cast last turn");

        // Front face, condition true: no spells cast last turn.
        let mut state = game_at_step(Step::Upkeep, P0);
        let front = named_creature(&mut state, &reg, name, P0);
        assert_eq!(upkeep_stack_entries(&mut state, &reg, front), 1,
            "{name} (front) must trigger when no spells were cast last turn");

        // Back face, condition false: nobody cast two or more.
        let mut state = game_at_step(Step::Upkeep, P0);
        state.num_spells_cast_last_turn.insert(P0, 1);
        state.num_spells_cast_last_turn.insert(P1, 1);
        let back = transformed(&mut state, &reg, name, P0);
        assert_eq!(upkeep_stack_entries(&mut state, &reg, back), 0,
            "{name} (back) must not trigger when no player cast two or more spells");

        // Back face, condition true: a player cast two.
        let mut state = game_at_step(Step::Upkeep, P0);
        state.num_spells_cast_last_turn.insert(P1, 2);
        let back = transformed(&mut state, &reg, name, P0);
        assert_eq!(upkeep_stack_entries(&mut state, &reg, back), 1,
            "{name} (back) must trigger when a player cast two or more spells");
    }
}

/// Found by the family test above: Wildblood Pack (Instigator Gang's back
/// face) declared only its `AnyCreatureAttacks` ability, and
/// `face_trigger_description` reads the *visible* face's list — so the back
/// face had no upkeep trigger and could never turn back into a Human, no
/// matter how many spells were cast. (Ticket instigator_gang-01.)
#[test]
fn wildblood_pack_can_transform_back() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);
    state.num_spells_cast_last_turn.insert(P1, 2);
    let pack = transformed(&mut state, &reg, "Instigator Gang", P0);

    fire_step_trigger(&mut state, Step::Upkeep, &reg);

    let obj = state.get_object(pack).unwrap();
    assert!(!obj.is_transformed,
        "Wildblood Pack should transform back to Instigator Gang when a player \
         cast two or more spells last turn; name is {}", obj.name);
}

// ── Morbid: the same clause on an enters-the-battlefield trigger ──

/// Count the stack entries an ETB dispatch produced for `object`.
fn etb_stack_entries(state: &mut GameState, reg: &CardRegistry, object: ObjectId) -> usize {
    state.events.push(mtg_engine::events::GameEvent::EnteredBattlefield {
        object,
        controller: state.get_object(object).unwrap().controller,
    });
    triggers::collect_triggers(state, reg);
    state.stack.iter()
        .filter(|e| matches!(e, StackEntry::Trigger(
            PendingTrigger {
                source: TriggerSource { id: object_id, .. },
                event: TriggerEvent::SelfEntered }) if *object_id == object))
        .count()
}

/// "Morbid — When this creature enters, if a creature died this turn, ..." is
/// the same CR 603.4 shape on the ETB path. (Ticket woodland_sleuth-01.)
#[test]
fn morbid_etb_triggers_only_when_a_creature_died() {
    let reg = registry();
    for name in ["Woodland Sleuth", "Hollowhenge Scavenger", "Morkrut Banshee"] {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        assert!(!state.creature_died_this_turn, "test precondition");
        let id = named_creature(&mut state, &reg, name, P0);
        assert_eq!(etb_stack_entries(&mut state, &reg, id), 0,
            "{name}: no creature died this turn, so the morbid ability must not \
             trigger — no stack entry, no priority window");

        let mut state = game_at_step(Step::PrecombatMain, P0);
        state.creature_died_this_turn = true;
        // Morkrut Banshee targets a creature; leave it as the only creature on
        // the battlefield so the single legal target is auto-picked. With two
        // choices the dispatch would stop to prompt instead of pushing, which
        // would fail this assertion for a reason unrelated to CR 603.4.
        let id = named_creature(&mut state, &reg, name, P0);
        assert_eq!(etb_stack_entries(&mut state, &reg, id), 1,
            "{name}: a creature died this turn, so the morbid ability must trigger");
    }
}

// ── Morbid on the other trigger timings ──────────────────────────

/// "Morbid — At the beginning of each end step, if a creature died this turn,
/// destroy target non-Demon creature." The same CR 603.4 clause on an end-step
/// trigger rather than an ETB, so it has to be checked in the step dispatch
/// too. Both arms, because "nothing was destroyed" is also what an engine that
/// never triggers at all would produce.
#[test]
fn reaper_from_the_abyss_end_step_trigger_respects_its_morbid_clause() {
    for died in [false, true] {
        let reg = registry();
        let mut state = game_at_step(Step::EndStep, P0);
        state.creature_died_this_turn = died;

        named_creature(&mut state, &reg, "Reaper from the Abyss", P0);
        let victim = ready_creature(&mut state, P1, 3, 3);

        // The step event is what the ability watches for, and it targets — so
        // the prompt has to be answered too (CR 603.3d).
        state.events.push(mtg_engine::events::GameEvent::StepStarted { step: Step::EndStep });
        process_triggers_auto_target_opponent(&mut state, &reg);

        if died {
            assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard,
                "a creature died this turn, so the Reaper's trigger destroys one");
        } else {
            assert_eq!(state.get_object(victim).unwrap().zone, Zone::Battlefield,
                "no creature died this turn, so the ability never triggers");
            assert!(state.stack.is_empty(),
                "and nothing sits on the stack holding a priority window open");
        }
    }
}

/// The morbid clause also gates an "enters with counters" replacement effect
/// (CR 614.1c) rather than a trigger — a different path through the engine, so
/// it needs its own check that the condition is consulted.
#[test]
fn morbid_enters_with_counters_only_when_a_creature_died() {
    // (card, counters it enters with when a creature died this turn)
    const CARDS: &[(&str, u32)] = &[
        ("Festerhide Boar", 2),
        ("Somberwald Spider", 2),
    ];

    for &(name, counters) in CARDS {
        for died in [false, true] {
            let reg = registry();
            let mut state = game_at_step(Step::PrecombatMain, P0);
            state.creature_died_this_turn = died;

            let spell = castable_spell(&mut state, &reg, name, P0);
            let state = cast_and_resolve(&state, &reg, spell, vec![]);

            let expected = if died { counters } else { 0 };
            assert_eq!(counters_of(&state, spell, CounterType::PlusOnePlusOne), expected,
                "{name} with creature_died_this_turn = {died}");
        }
    }
}

/// A morbid ETB that returns a creature card from your graveyard can return the
/// source itself, if the source died in response to its own trigger: CR 113.7a
/// keeps the ability on the stack, and by the time it resolves the card is in
/// the graveyard and is as legal a choice as anything else there.
#[test]
fn woodland_sleuth_can_return_itself_after_dying_to_its_own_trigger() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.creature_died_this_turn = true;

    let sleuth = named_creature(&mut state, &reg, "Woodland Sleuth", P0);
    let card_id = state.get_object(sleuth).unwrap().card_id;
    state.stack.push(StackEntry::Trigger(PendingTrigger::new(
        TriggerSource::new(sleuth, card_id, P0, "Woodland Sleuth"),
        TriggerEvent::SelfEntered,
    )));
    // Killed with its own ETB trigger already on the stack.
    state.move_object(sleuth, Zone::Graveyard, &reg);

    triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(sleuth).unwrap().zone, Zone::Hand,
        "the only creature card in the graveyard is the Sleuth itself, so that \
         is what comes back");
}
