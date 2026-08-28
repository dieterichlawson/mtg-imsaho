//! "Enters tapped unless ..." is a replacement effect (CR 614.1d), not a
//! triggered ability.
//!
//! The five Innistrad check lands used to model it as an ETB trigger that
//! tapped the land when it resolved. The end state was usually right, which is
//! why nothing caught it, but three things were observably wrong:
//!
//! 1. The land entered UNTAPPED and stayed that way until the trigger
//!    resolved, so its controller could tap it for mana in response to their
//!    own trigger and have it enter tapped anyway — a free mana.
//! 2. The condition was read at resolution, so an opponent could destroy or
//!    bounce the controller's only Mountain in response and tap a land that
//!    should have entered untapped.
//! 3. Even when the condition was met and nothing needed to happen, a trigger
//!    still went on the stack and handed everyone a priority window.
//!
//! A replacement effect has none of those: it modifies the entering event
//! itself, before `EnteredBattlefield` is emitted.

mod common;

use common::*;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::state::GameState;
use mtg_engine::triggers::{self, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;
/// Every Innistrad check land: the two land types that satisfy its condition,
/// a basic that does not, and the two mana it adds.
///
/// This used to be two tables — this one, and an `AUDITED` subset that the
/// stricter tests looped over, because judging a card against wording nobody
/// had fetched is exactly what the audit procedure forbids. All five have now
/// been audited against their fetched oracle text, so the gate has done its
/// job and the tables are one again. Adding a sixth check land means auditing
/// it first.
const CHECK_LANDS: &[(&str, [&str; 2], &str, [ManaType; 2])] = &[
    ("Clifftop Retreat",  ["Mountain", "Plains"],   "Island",   [ManaType::Red,   ManaType::White]),
    ("Hinterland Harbor", ["Forest",   "Island"],   "Swamp",    [ManaType::Green, ManaType::Blue]),
    ("Isolated Chapel",   ["Plains",   "Swamp"],    "Mountain", [ManaType::White, ManaType::Black]),
    ("Sulfur Falls",      ["Island",   "Mountain"], "Forest",   [ManaType::Blue,  ManaType::Red]),
    ("Woodland Cemetery", ["Swamp",    "Forest"],   "Plains",   [ManaType::Black, ManaType::Green]),
];

/// Put a land onto the battlefield the way a land drop does — through
/// `move_object`, so entering replacements apply.
fn play_land(state: &mut GameState, reg: &CardRegistry, name: &str, owner: mtg_engine::ids::PlayerId) -> ObjectId {
    let card_id = reg.get_id_by_name(name).unwrap_or_else(|| panic!("unknown card {name}"));
    let id = state.create_object(card_id, owner, Zone::Hand, None, None);
    state.get_object_mut(id).unwrap().name = name.into();
    state.move_object(id, Zone::Battlefield, reg);
    id
}

#[test]
fn check_land_enters_untapped_when_condition_is_met() {
    let reg = registry();
    for (land, satisfying, _, _) in CHECK_LANDS {
        // Both halves of "unless you control an X or a Y" — a condition that
        // had dropped one of them would still pass a test that only ever
        // played the first.
        for good in satisfying {
            let mut state = game_at_step(Step::PrecombatMain, P0);
            play_land(&mut state, &reg, good, P0);
            let id = play_land(&mut state, &reg, land, P0);

            assert!(!state.get_object(id).unwrap().tapped,
                "{land} should enter untapped while you control a {good}");
        }
    }
}

#[test]
fn check_land_enters_tapped_when_condition_is_not_met() {
    let reg = registry();
    for (land, _, bad, _) in CHECK_LANDS {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        play_land(&mut state, &reg, bad, P0);
        let id = play_land(&mut state, &reg, land, P0);

        assert!(state.get_object(id).unwrap().tapped,
            "{land} should enter tapped when you control only a {bad}");
    }
}

/// The tap is decided before the permanent finishes entering, so it is already
/// tapped by the time anyone could act — there is no untapped window to
/// exploit for a free mana.
#[test]
fn check_land_is_already_tapped_before_anyone_gets_priority() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let id = play_land(&mut state, &reg, "Clifftop Retreat", P0);

    // Straight after move_object and before any trigger processing.
    assert!(state.get_object(id).unwrap().tapped,
        "the replacement applies during the zone change, so the land is never \
         observable in an untapped state");
    assert!(state.get_player(P0).mana_pool.total() == 0,
        "test precondition: no mana floating");
}

/// A replacement effect uses no stack, so nothing goes on it either way.
#[test]
fn check_land_puts_no_trigger_on_the_stack() {
    let reg = registry();
    for (land, satisfying, bad, _) in CHECK_LANDS {
        for basic in satisfying.iter().chain(std::iter::once(bad)) {
            let mut state = game_at_step(Step::PrecombatMain, P0);
            play_land(&mut state, &reg, basic, P0);
            let id = play_land(&mut state, &reg, land, P0);

            triggers::collect_triggers(&mut state, &reg);
            let entries = state.stack.iter()
                .filter(|e| matches!(e, mtg_engine::state::StackEntry::Trigger(
                    PendingTrigger { source: TriggerSource { id: object_id, .. },
                        event: TriggerEvent::SelfEntered }) if *object_id == id))
                .count();
            assert_eq!(entries, 0,
                "{land} (with a {basic}) must not put an ETB trigger on the \
                 stack — the tapping is a replacement effect, not an ability");
        }
    }
}

/// The condition is locked in at entry. Removing the qualifying land
/// afterwards cannot retroactively tap a land that entered untapped — which
/// the trigger-based version got wrong, because it re-read the board at
/// resolution.
#[test]
fn condition_is_evaluated_at_entry_not_later() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mountain = play_land(&mut state, &reg, "Mountain", P0);
    let id = play_land(&mut state, &reg, "Clifftop Retreat", P0);
    assert!(!state.get_object(id).unwrap().tapped, "test precondition: entered untapped");

    // An opponent removes the Mountain in what would have been the response
    // window; the Retreat stays untapped because it already finished entering.
    state.move_object(mountain, Zone::Graveyard, &reg);
    triggers::collect_triggers(&mut state, &reg);

    assert!(!state.get_object(id).unwrap().tapped,
        "the land entered untapped and must stay untapped — the condition is \
         not re-read after the permanent has entered");
}

/// A check land that sees only the OTHER check lands still enters tapped —
/// they have no basic land subtypes of their own.
#[test]
fn check_lands_do_not_satisfy_each_other() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    play_land(&mut state, &reg, "Sulfur Falls", P0);
    let id = play_land(&mut state, &reg, "Clifftop Retreat", P0);

    assert!(state.get_object(id).unwrap().tapped,
        "Sulfur Falls is not a Mountain or a Plains, so Clifftop Retreat \
         enters tapped");
}

/// "unless **you control** a Forest or an Island." An opponent's is not yours,
/// and no test put the qualifying land on the other side of the board — a
/// condition that scanned the whole battlefield would have passed every
/// existing case.
#[test]
fn a_check_land_is_not_satisfied_by_an_opponents_land() {
    let reg = registry();
    for (land, satisfying, _, _) in CHECK_LANDS {
        for basic in satisfying {
            let mut state = game_at_step(Step::PrecombatMain, P0);
            play_land(&mut state, &reg, basic, P1);
            let id = play_land(&mut state, &reg, land, P0);
            assert!(state.get_object(id).unwrap().tapped,
                "{land}: an opponent's {basic} is not one you control");

            // The same basic on your own side does satisfy it, so the
            // assertion above is about *whose* land it is and not about the
            // land being absent.
            let mut state = game_at_step(Step::PrecombatMain, P0);
            play_land(&mut state, &reg, basic, P0);
            let id = play_land(&mut state, &reg, land, P0);
            assert!(!state.get_object(id).unwrap().tapped,
                "{land}: your own {basic} does satisfy it");
        }
    }
}

/// "{T}: Add {G} **or** {U}." The shared sweep counts two mana abilities,
/// which a land exposing the same one twice would also satisfy.
#[test]
fn each_check_land_taps_for_both_of_its_colours() {
    let reg = registry();
    let state = game_at_step(Step::PrecombatMain, P0);
    for (land, _, _, colours) in CHECK_LANDS {
        let card_id = reg.get_id_by_name(land).unwrap();
        let produced: Vec<Vec<(ManaType, u32)>> = reg.get(card_id).unwrap()
            .mana_abilities(&state, mtg_engine::ids::ObjectId(0))
            .into_iter()
            .map(|a| a.produced)
            .collect();

        assert_eq!(produced.len(), 2, "{land}: two mana abilities");
        for colour in colours {
            assert!(produced.contains(&vec![(*colour, 1)]),
                "{land} should add {colour:?}; got {produced:?}");
        }
    }
}
