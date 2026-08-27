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
/// Each check land, with a basic that satisfies its condition and one that
/// doesn't.
const CHECK_LANDS: &[(&str, &str, &str)] = &[
    ("Clifftop Retreat", "Mountain", "Island"),
    ("Hinterland Harbor", "Forest", "Swamp"),
    ("Isolated Chapel", "Plains", "Mountain"),
    ("Sulfur Falls", "Island", "Forest"),
    ("Woodland Cemetery", "Swamp", "Plains"),
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
    for (land, good, _) in CHECK_LANDS {
        let mut state = game_at_step(Step::PrecombatMain, P0);
        play_land(&mut state, &reg, good, P0);
        let id = play_land(&mut state, &reg, land, P0);

        assert!(!state.get_object(id).unwrap().tapped,
            "{land} should enter untapped while you control a {good}");
    }
}

#[test]
fn check_land_enters_tapped_when_condition_is_not_met() {
    let reg = registry();
    for (land, _, bad) in CHECK_LANDS {
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
    for (land, good, bad) in CHECK_LANDS {
        for basic in [good, bad] {
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

/// "This creature enters tapped" holds however it enters — cast, reanimated,
/// or put onto the battlefield by anything else (CR 614.1c).
///
/// Diregraf Ghoul used to tap itself inside `on_resolve`, which only runs when
/// the card is *cast*. Innistrad has several ways to put a creature card from a
/// graveyard onto the battlefield — Unburial Rites, Grimoire of the Dead, Back
/// from the Brink — and down every one of those paths the Ghoul arrived
/// untapped. Its own comment said "'enters tapped' is a static/replacement
/// ability, NOT a triggered ability", and the code did not follow it.
#[test]
fn a_creature_that_enters_tapped_does_so_however_it_arrives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Not cast: reanimated straight from the graveyard onto the battlefield.
    let ghoul = named_card_in_graveyard(&mut state, &reg, "Diregraf Ghoul", P0);
    assert!(!state.get_object(ghoul).unwrap().tapped, "test premise: not tapped in the yard");

    state.move_object(ghoul, Zone::Battlefield, &reg);

    assert!(state.get_object(ghoul).unwrap().tapped,
        "a reanimated Diregraf Ghoul still enters tapped — the replacement \
         applies to the entering event, not to being cast (CR 614.1c)");
}

/// CR 614.12: "**As** [this] enters, choose ..." is a replacement effect. The
/// choice is made as the permanent enters — there is no moment where it is on
/// the battlefield with the choice not yet made, and nothing goes on the stack
/// for anyone to respond to.
///
/// Nevermore ("As this enchantment enters, choose a nonland card name. Spells
/// with the chosen name can't be cast.") declared an `EntersBattlefield`
/// triggered ability instead. It resolved onto the battlefield with no name
/// chosen and the choice sitting on the stack — a window in which an opponent
/// could cast the very card it was about to name, which is the entire point of
/// the card.
#[test]
fn a_name_chosen_as_a_permanent_enters_is_chosen_before_anyone_has_priority() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    stock_library(&mut state, &reg, P0, 4);

    let nevermore = castable_spell(&mut state, &reg, "Nevermore", P0);
    let state = cast_onto_stack(&state, &reg, nevermore, vec![]);
    let mut state = state;
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(nevermore).unwrap().zone, Zone::Battlefield,
        "test premise: it resolved onto the battlefield");
    assert!(state.awaiting_action.is_some(),
        "the name is chosen AS it enters (CR 614.12), so the choice is already \
         pending the moment it arrives");

    triggers::collect_triggers(&mut state, &reg);
    assert!(state.stack.is_empty(),
        "and nothing is on the stack — a replacement effect is not a triggered \
         ability, so no priority window opens before the name exists");
}
