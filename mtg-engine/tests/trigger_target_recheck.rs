//! CR 608.2b: when a triggered ability tries to resolve, its targets are
//! re-checked. If they have all become illegal, the ability is countered by
//! the game rules.
//!
//! The re-check ran only the generic half — zone, hexproof, target filter —
//! and skipped `is_valid_target`, the card's own restriction on what it may
//! target. `resolve_spell` had always run both. So a trigger resolved happily
//! against a target that had stopped satisfying the card's wording: Grimgrin's
//! "creature the defending player controls" survived that creature changing
//! controller in response.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::state::StackEntry;
use mtg_engine::triggers::{PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// Angel of Flight Alabaster targets "a Spirit card in your graveyard". A
/// card that stops being a legal target between announcement and resolution
/// makes the ability fizzle.
#[test]
fn a_trigger_fizzles_when_its_target_stops_satisfying_the_cards_restriction() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();

    // A card in the graveyard that is NOT a Spirit. It satisfies the generic
    // half of legality — right zone, no hexproof, matches the target filter —
    // and is rejected only by the card's own `is_valid_target` ("target Spirit
    // card"). That is precisely the half the re-check used to skip.
    let not_a_spirit = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    assert!(!state.has_subtype(not_a_spirit, "Spirit", &reg), "test precondition");

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(not_a_spirit)], ..TriggerSource::new(angel, angel_card, P0, "Angel of Flight Alabaster") },
        event: TriggerEvent::Upkeep,
    }));
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(not_a_spirit).unwrap().zone, Zone::Graveyard,
        "the only target does not satisfy 'target Spirit card', so the ability \
         is countered on resolution rather than returning it (CR 608.2b)");
}

/// The happy path still works: a legal target is still returned.
#[test]
fn a_trigger_with_a_still_legal_target_resolves() {
    let reg = registry();
    let mut state = game_at_step(Step::Upkeep, P0);

    let angel = named_permanent(&mut state, &reg, "Angel of Flight Alabaster", P0);
    let angel_card = reg.get_id_by_name("Angel of Flight Alabaster").unwrap();
    let spirit = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    state.stack.push(StackEntry::Trigger(PendingTrigger {
        source: TriggerSource { chosen_targets: vec![Target::Object(spirit)], ..TriggerSource::new(angel, angel_card, P0, "Angel of Flight Alabaster") },
        event: TriggerEvent::Upkeep,
    }));
    mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

    assert_eq!(state.get_object(spirit).unwrap().zone, Zone::Hand,
        "a legal Spirit card in the graveyard is returned to hand");
}

/// Civilized Scholar's "unless it attacked this turn" marker is stamped with
/// the turn it happened on. A bare marker could not be told apart from one
/// left over from a previous turn, and the clearing path only ran on the back
/// face's end step — so a front-face attack in turn N stuck forever and
/// stopped the Brute transforming back in every later turn.
#[test]
fn an_attack_in_an_earlier_turn_does_not_keep_the_brute_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let scholar = named_permanent(&mut state, &reg, "Civilized Scholar", P0);
    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();

    // It attacked on the front face this turn...
    state.step = Step::DeclareAttackers;
    state.get_object_mut(scholar).unwrap().summoning_sick = false;
    mtg_engine::combat::declare_attackers(&mut state, &[(scholar, P1)], &reg);
    state.step = Step::EndStep;
    // ...then a later turn begins, and it transforms.
    state.turn_number += 1;
    mtg_engine::cards::helpers::apply_transform(&mut state, scholar, &reg);
    assert!(state.get_object(scholar).unwrap().is_transformed, "test precondition");

    behavior.on_end_step(&mut state, scholar, &[], &reg);

    assert!(!state.get_object(scholar).unwrap().is_transformed,
        "the attack was in a PREVIOUS turn, so Homicidal Brute did not attack \
         this turn and must tap and transform back");
}

/// The same-turn case still holds: an attack this turn keeps it transformed
/// (CR 712.8 — transforming does not make a new object).
#[test]
fn an_attack_this_turn_keeps_the_brute_transformed() {
    let reg = registry();
    let mut state = game_at_step(Step::EndStep, P0);

    let scholar = named_permanent(&mut state, &reg, "Civilized Scholar", P0);
    let behavior = reg.get(state.get_object(scholar).unwrap().card_id).unwrap();

    state.step = Step::DeclareAttackers;
    state.get_object_mut(scholar).unwrap().summoning_sick = false;
    mtg_engine::combat::declare_attackers(&mut state, &[(scholar, P1)], &reg);
    state.step = Step::EndStep;
    mtg_engine::cards::helpers::apply_transform(&mut state, scholar, &reg);
    behavior.on_end_step(&mut state, scholar, &[], &reg);

    assert!(state.get_object(scholar).unwrap().is_transformed,
        "it attacked this turn, so it stays a Homicidal Brute");
}

// -------------------------------------------------------------------------
// Per-card cases
// -------------------------------------------------------------------------

/// Everything about the game that a resolving trigger could change, for the
/// two objects involved.
///
/// A countered trigger must leave all of it alone. Each of the cases below
/// used to assert only the one thing its own card does — Grimgrin's counter,
/// Morkrut Banshee's -4/-4, Snapcaster's flashback grant — so a trigger that
/// fizzled its main effect and still did something else would have passed.
#[derive(Debug, PartialEq)]
struct Footprint {
    target_zone: Option<Zone>,
    target_power: Option<i32>,
    target_toughness: Option<i32>,
    target_counters: u32,
    source_counters: u32,
    temporary_effects: usize,
    objects: usize,
}

fn footprint(
    state: &mtg_engine::state::GameState,
    reg: &mtg_engine::cards::CardRegistry,
    source: ObjectId,
    target: ObjectId,
) -> Footprint {
    Footprint {
        target_zone: state.get_object(target).map(|o| o.zone),
        target_power: state.effective_power(target, reg),
        target_toughness: state.effective_toughness(target, reg),
        target_counters: counters_of(state, target, CounterType::PlusOnePlusOne),
        source_counters: counters_of(state, source, CounterType::PlusOnePlusOne),
        temporary_effects: state.until_end_of_turn.len(),
        objects: state.objects.len(),
    }
}

/// Six abilities, six ways for the chosen target to stop being legal between
/// announcement and resolution (CR 608.2b). Three of them are the generic half
/// — the target left the zone, or gained hexproof — and three are the card's
/// own restriction, which is the half the re-check used to skip.
#[test]
fn a_trigger_whose_target_became_illegal_changes_nothing() {
    type Sabotage = fn(&mut mtg_engine::state::GameState, ObjectId, &mtg_engine::cards::CardRegistry);

    // (card, where its target sits, who starts out controlling it, the event,
    //  what makes the target illegal, why)
    let cases: &[(&str, bool, PlayerId, fn(ObjectId) -> TriggerEvent, Sabotage, &str)] = &[
        ("Angel of Flight Alabaster", true, P1, |_| TriggerEvent::Upkeep,
         |state, id, reg| state.move_object(id, Zone::Exile, reg),
         "the Spirit card it named was exiled out of the graveyard"),
        ("Snapcaster Mage", true, P1, |_| TriggerEvent::SelfEntered,
         |state, id, reg| state.move_object(id, Zone::Exile, reg),
         "the instant it named was exiled out of the graveyard"),
        ("Grimgrin, Corpse-Born", false, P1, |src| TriggerEvent::Attacks { attacker: src, defending_player: P1 },
         |state, id, _| state.get_object_mut(id).unwrap().keywords.push(Keyword::Hexproof),
         "an opponent's creature with hexproof can't be targeted (CR 702.11b)"),
        ("Morkrut Banshee", false, P1, |_| TriggerEvent::SelfEntered,
         |state, id, _| state.get_object_mut(id).unwrap().keywords.push(Keyword::Hexproof),
         "same, on an enters-the-battlefield trigger"),
        ("Reaper from the Abyss", false, P1, |_| TriggerEvent::EndStep,
         |state, id, reg| state.move_object(id, Zone::Hand, reg),
         "a creature in hand is not on the battlefield to be destroyed"),
        ("Reaper from the Abyss", false, P1, |_| TriggerEvent::EndStep,
         |state, id, _| state.get_object_mut(id).unwrap().subtypes.push("Demon".into()),
         "'target non-Demon creature' stopped being true of it — the card's own \
          restriction, not the generic half"),
        // The only case here whose target starts under the trigger controller:
        // "target creature *you control*" is the one restriction that a change
        // of control breaks, and none of the five above can be broken that way.
        ("Elder Cathar", false, P0, |_| TriggerEvent::SelfDies,
         |state, id, _| state.get_object_mut(id).unwrap().controller = P1,
         "'target creature you control' stopped being true of it — the target \
          changed controller in response (CR 608.2b)"),
    ];

    for &(name, target_in_graveyard, target_controller, event_of, sabotage, why) in cases {
        let reg = registry();
        let mut state = game_at_step(Step::EndStep, P0);
        state.creature_died_this_turn = true; // for the morbid ones

        let source = named_permanent(&mut state, &reg, name, P0);
        let card_id = state.get_object(source).unwrap().card_id;
        let target = if target_in_graveyard {
            named_card_in_graveyard(&mut state, &reg, "Think Twice", P0)
        } else {
            ready_creature(&mut state, target_controller, 4, 4)
        };

        state.stack.push(StackEntry::Trigger(PendingTrigger {
            source: TriggerSource {
                chosen_targets: vec![Target::Object(target)],
                ..TriggerSource::new(source, card_id, P0, name)
            },
            event: event_of(source),
        }));

        sabotage(&mut state, target, &reg);
        let before = footprint(&state, &reg, source, target);

        mtg_engine::triggers::resolve_next_trigger(&mut state, &reg);

        assert_eq!(footprint(&state, &reg, source, target), before,
            "{name}: {why} — the ability is countered by game rules, so nothing \
             about either permanent may change (CR 608.2b)");
    }
}
