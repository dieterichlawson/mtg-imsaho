//! CR 400.7: an object that leaves a zone and comes back is a new object,
//! and an ability of the old one does not act on it. An ability's stack
//! entry carries its source's zone change count as it went on the stack
//! (`GameState::put_on_stack`), and the counters, shields, taps, untaps and
//! transforms it would put on its source each refuse a source that has
//! changed zones since (`GameState::is_new_source`).
//!
//! Each case resolves one ability twice: with its source still there, where
//! the effect lands (so the case can see it), and with the source exiled and
//! returned in response (Fiend Hunter's exile undone by its death), where it
//! must not land on the creature that came back. Champion of the Parish's
//! trigger put its counter on a Champion that had been exiled and returned
//! in response.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::state::{GameState, StackEntry, TemporaryEffect};
use mtg_engine::triggers::{DeadCreature, PendingTrigger, TriggerEvent, TriggerSource};
use mtg_engine::types::*;

/// What an ability may change about its own source.
#[derive(Debug, PartialEq, Clone)]
struct Mark {
    counters: u32,
    shields: u32,
    tapped: bool,
    transformed: bool,
    zone: Option<Zone>,
    /// Until-end-of-turn effects on it.
    effects: usize,
}

fn mark(state: &GameState, id: ObjectId) -> Mark {
    let o = state.get_object(id);
    Mark {
        counters: o.map_or(0, |o| o.counters.values().sum()),
        shields: o.map_or(0, |o| o.regeneration_shields),
        tapped: o.is_some_and(|o| o.tapped),
        transformed: o.is_some_and(|o| o.is_transformed),
        zone: o.map(|o| o.zone),
        effects: state.until_end_of_turn.iter().filter(|e| about(e) == Some(id)).count(),
    }
}

/// The permanent an until-end-of-turn effect is about, if it is one's.
fn about(effect: &TemporaryEffect) -> Option<ObjectId> {
    use TemporaryEffect as T;
    match effect {
        T::ModifyPT { target, .. } | T::GrantKeyword { target, .. } | T::RemoveKeyword { target, .. }
        | T::CantBlock { target } | T::GrantProtection { target, .. } => Some(*target),
        _ => None,
    }
}

/// `id` exiled and returned to the battlefield: a new object (CR 400.7).
fn exile_and_return(state: &mut GameState, reg: &mtg_engine::cards::CardRegistry, id: ObjectId) {
    let before = state.get_object(id).expect("there").zone_change_count;
    state.move_object(id, Zone::Exile, reg);
    state.move_object(id, Zone::Battlefield, reg);
    let o = state.get_object(id).expect("returned");
    assert_eq!(o.zone, Zone::Battlefield);
    assert!(o.zone_change_count > before, "two zone changes are counted");
}

fn trigger(state: &GameState, source: ObjectId, event: TriggerEvent) -> StackEntry {
    let o = state.get_object(source).expect("the source");
    StackEntry::Trigger(PendingTrigger {
        source: TriggerSource::new(source, o.card_id, o.controller, o.name.clone())
            .from_face(o.is_transformed),
        event,
        ordered: false,
    })
}

fn dead(id: ObjectId, controller: PlayerId) -> DeadCreature {
    DeadCreature {
        id, name: String::new(), controller, damaged_by: vec![], toughness: 2,
        is_token: false, subtypes: vec!["Human".into()],
    }
}

type Setup = fn(&mut GameState, &mtg_engine::cards::CardRegistry, ObjectId) -> Ability;

/// The ability a case puts on the stack.
enum Ability {
    Trigger(TriggerEvent),
    /// Activated: its index.
    Activated(usize),
}

/// One case: the card, how to set up and which ability, and what lands on
/// the source when it is still there.
struct Case {
    name: &'static str,
    setup: Setup,
    lands: fn(&Mark, &Mark) -> bool,
}

fn more_counters(before: &Mark, after: &Mark) -> bool { after.counters > before.counters }
fn a_shield(before: &Mark, after: &Mark) -> bool { after.shields > before.shields }
fn untapped(before: &Mark, after: &Mark) -> bool { before.tapped && !after.tapped }
fn flipped(before: &Mark, after: &Mark) -> bool { before.transformed != after.transformed }
fn to_hand(_: &Mark, after: &Mark) -> bool { after.zone == Some(Zone::Hand) }
fn pumped(before: &Mark, after: &Mark) -> bool { after.effects > before.effects }

fn cases() -> Vec<Case> {
    vec![
        Case { name: "Champion of the Parish", lands: more_counters, setup: |state, reg, _| {
            let human = named_permanent(state, reg, "Doomed Traveler", P0);
            Ability::Trigger(TriggerEvent::CreatureEntered { entered: human, entered_controller: P0 })
        }},
        Case { name: "Unruly Mob", lands: more_counters, setup: |state, _, _| {
            let other = ready_creature(state, P0, 2, 2);
            Ability::Trigger(TriggerEvent::CreatureDied { dead: dead(other, P0) })
        }},
        Case { name: "Village Cannibals", lands: more_counters, setup: |state, _, _| {
            let other = ready_creature(state, P1, 2, 2);
            Ability::Trigger(TriggerEvent::CreatureDied { dead: dead(other, P1) })
        }},
        Case { name: "Lumberknot", lands: more_counters, setup: |state, _, _| {
            let other = ready_creature(state, P1, 2, 2);
            Ability::Trigger(TriggerEvent::CreatureDied { dead: dead(other, P1) })
        }},
        Case { name: "Gutter Grime", lands: more_counters, setup: |state, _, _| {
            let other = ready_creature(state, P0, 2, 2);
            Ability::Trigger(TriggerEvent::CreatureDied { dead: dead(other, P0) })
        }},
        Case { name: "Stromkirk Noble", lands: more_counters, setup: |_, _, _| {
            Ability::Trigger(TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 1 })
        }},
        Case { name: "Bloodcrazed Neonate", lands: more_counters, setup: |_, _, _| {
            Ability::Trigger(TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 2 })
        }},
        Case { name: "Stromkirk Patrol", lands: more_counters, setup: |_, _, _| {
            Ability::Trigger(TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 4 })
        }},
        Case { name: "Falkenrath Marauders", lands: more_counters, setup: |_, _, _| {
            Ability::Trigger(TriggerEvent::CombatDamageToPlayer { damaged_player: P1, amount: 2 })
        }},
        Case { name: "Galvanic Juggernaut", lands: untapped, setup: |state, _, me| {
            state.get_object_mut(me).expect("there").tapped = true;
            let other = ready_creature(state, P1, 2, 2);
            Ability::Trigger(TriggerEvent::CreatureDied { dead: dead(other, P1) })
        }},
        Case { name: "Gatstaf Shepherd", lands: flipped, setup: |state, _, _| {
            // CR 603.4: "if no spells were cast last turn", re-checked.
            state.num_spells_cast_last_turn.clear();
            Ability::Trigger(TriggerEvent::Upkeep)
        }},
        Case { name: "Mikaeus, the Lunarch", lands: more_counters, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Ludevic's Test Subject", lands: more_counters, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Manor Skeleton", lands: a_shield, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Lantern Spirit", lands: to_hand, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Darkthicket Wolf", lands: pumped, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Feral Ridgewolf", lands: pumped, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Kessig Wolf", lands: pumped, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Manor Gargoyle", lands: pumped, setup: |_, _, _| Ability::Activated(0) },
        Case { name: "Charmbreaker Devils", lands: pumped, setup: |state, reg, _| {
            let bolt = spell_in_hand(state, reg, "Brimstone Volley", P0);
            Ability::Trigger(TriggerEvent::SpellCast { caster: P0, spell_id: bolt })
        }},
    ]
}

/// The source's mark before and after its ability resolves, the source
/// exiled and returned in response or not.
fn resolve(case: &Case, returned: bool) -> (Mark, Mark) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let me = named_permanent(&mut state, &reg, case.name, P0);
    let ability = (case.setup)(&mut state, &reg, me);
    match ability {
        Ability::Trigger(event) => {
            let entry = trigger(&state, me, event);
            state.put_on_stack(entry);
        }
        Ability::Activated(index) => activate_via_hooks(&mut state, &reg, me, index, &[]),
    }
    if returned {
        exile_and_return(&mut state, &reg, me);
        // It came back untapped, on its front face, with no counters. The
        // one case that looks for an untap taps it again.
        if case.name == "Galvanic Juggernaut" {
            state.get_object_mut(me).expect("there").tapped = true;
        }
    }
    let before = mark(&state, me);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(state.awaiting_action.is_none(), "{}: the ability asked something: {:?}", case.name, state.awaiting_action);
    (before, mark(&state, me))
}

#[test]
fn an_ability_acts_on_its_source_while_the_source_is_the_object_it_was() {
    for case in cases() {
        let (before, after) = resolve(&case, false);
        assert!((case.lands)(&before, &after),
            "{}: its ability's effect on itself lands while it stays: {before:?} -> {after:?}", case.name);
    }
}

#[test]
fn an_ability_does_not_act_on_its_source_once_it_has_left_and_come_back() {
    for case in cases() {
        let (before, after) = resolve(&case, true);
        assert_eq!(before, after,
            "{}: exiled and returned in response, it is a new object, which its old ability does not act on (CR 400.7)",
            case.name);
    }
}

/// Moldgraf Monstrosity's "exile it" names the card its death put in the
/// graveyard. One that has since come back and died again is a new object
/// in the same graveyard, and stays there; the two creature cards are
/// returned either way (CR 608.2c, it does as much as it can).
#[test]
fn moldgraf_monstrosity_exiles_only_the_card_it_died_as() {
    for died_again in [false, true] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let moldgraf = named_permanent(&mut state, &reg, "Moldgraf Monstrosity", P0);
        for name in ["Grizzly Bears", "Walking Corpse"] {
            named_card_in_graveyard(&mut state, &reg, name, P0);
        }
        state.move_object(moldgraf, Zone::Graveyard, &reg);
        let entry = trigger(&state, moldgraf, TriggerEvent::SelfDies);
        state.put_on_stack(entry);
        if died_again {
            state.move_object(moldgraf, Zone::Battlefield, &reg);
            state.move_object(moldgraf, Zone::Graveyard, &reg);
        }
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
        let zone = state.get_object(moldgraf).map(|o| o.zone);
        let expected = if died_again { Zone::Graveyard } else { Zone::Exile };
        assert_eq!(zone, Some(expected), "died again in response: {died_again}");
        let returned = state.all_objects_in_zone(Zone::Battlefield).iter()
            .filter(|o| o.id != moldgraf && o.owner == P0).count();
        assert_eq!(returned, 2, "two creature cards come back either way");
    }
}

/// Fiend Hunter's exile is linked to the Hunter that exiled (CR 607.2a). A
/// Hunter exiled and returned before its enters trigger resolved is a new
/// object: the trigger still exiles its target, which the new Hunter's
/// leaving does not return, so it stays exiled (the card's ruling for a
/// Hunter gone before the trigger resolves).
#[test]
fn fiend_hunter_returned_before_its_trigger_resolves_keeps_nothing_it_exiles() {
    for returned in [false, true] {
        let reg = registry();
        let mut state = game_at_step(Step::PrecombatMain, P0);
        let victim = ready_creature(&mut state, P1, 3, 3);
        let hunter = named_permanent(&mut state, &reg, "Fiend Hunter", P0);
        let o = state.get_object(hunter).expect("there");
        let entry = StackEntry::Trigger(PendingTrigger {
            source: TriggerSource {
                chosen_targets: vec![Target::Object(victim)],
                ..TriggerSource::new(hunter, o.card_id, P0, "Fiend Hunter")
            },
            event: TriggerEvent::SelfEntered,
            ordered: false,
        });
        state.put_on_stack(entry);
        if returned {
            exile_and_return(&mut state, &reg, hunter);
        }
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
        // "You may": its controller says yes.
        state = mtg_engine::engine::submit_action(&state, &mtg_engine::actions::Action::ResolveChoice {
            choice: mtg_engine::actions::ResolvedChoice::ChosenTarget(Some(Target::Object(victim))),
        }, &reg);
        assert_eq!(state.get_object(victim).map(|o| o.zone), Some(Zone::Exile), "the target is exiled either way");
        // The Hunter on the battlefield now leaves, and its leaves trigger
        // resolves.
        state.move_object(hunter, Zone::Graveyard, &reg);
        let leaves = trigger(&state, hunter, TriggerEvent::LeftBattlefield);
        state.put_on_stack(leaves);
        mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
        let expected = if returned { Zone::Exile } else { Zone::Battlefield };
        assert_eq!(state.get_object(victim).map(|o| o.zone), Some(expected),
            "a Hunter returned in response is not the one that exiled: {returned}");
    }
}
