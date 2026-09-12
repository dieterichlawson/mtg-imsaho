//! The fuzzing oracle checks itself: every invariant family in
//! `mtg_engine::invariants` must flag the exact corruption it claims to
//! catch, and a clean state must flag nothing.
//!
//! These tests exist because the checker is load-bearing test
//! infrastructure — ~110k invariant-checked games run nightly against it —
//! and the full mutation sweep (issues #26–#34) showed that a mutant which
//! silently blinds one of its clauses would go unnoticed: the fuzzer only
//! reports what the checker reports. Each test builds a healthy state,
//! corrupts one property, and asserts the corresponding violation message
//! appears (and, via the clean-state test, that no clause fires when
//! nothing is wrong).

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::{CardId, ObjectId};
use mtg_engine::invariants::{check_core, check_settled};
use mtg_engine::state::{CombatState, StackEntry};
use mtg_engine::types::*;

/// Assert `check_settled` reports a violation containing `needle`.
#[track_caller]
fn assert_flags(state: &mtg_engine::state::GameState, reg: &CardRegistry, needle: &str) {
    let v = check_settled(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)),
        "expected a violation containing {needle:?}, got: {v:?}");
}

/// A RICH clean state: every zone populated the way real games populate
/// them. This is the false-positive half of the oracle's contract — a
/// mutant that inverts a check fires on healthy structures, and only a
/// state that actually has libraries, graveyards, a stack, attachments,
/// loyalty, and combat can notice.
#[test]
fn a_clean_state_has_no_violations() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // `game_at_step` leaves the turn counter at 1 with the first-turn flag
    // off, which is not a state a game can reach; a later turn is.
    state.turn_number = 3;

    // Battlefield: creatures, a land, an attached Aura, attached Equipment,
    // a planeswalker with loyalty. The bear carries healthy in-game marks —
    // non-lethal damage (with the record of what dealt it) and a +1/+1
    // counter — because an inverted checker clause flags exactly the
    // healthy version of what it polices. Real cards throughout: the
    // checker asks the registry about every card.
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let opposing_bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    {
        let o = state.get_object_mut(bear).unwrap();
        o.damage_marked = 1;
        o.damaged_by.push(opposing_bear);
        o.counters.insert(CounterType::PlusOnePlusOne, 1);
    }
    named_permanent(&mut state, &reg, "Forest", P1);
    // A legend on the battlefield with its twin in the graveyard: only
    // battlefield copies count for CR 704.5j.
    let geist_id = reg.get_id_by_name("Geist of Saint Traft").unwrap();
    let geist = state.create_object(geist_id, P0, Zone::Battlefield, Some(2), Some(2));
    state.get_object_mut(geist).unwrap().name = "Geist of Saint Traft".into();
    let dead_geist = state.create_object(geist_id, P0, Zone::Graveyard, Some(2), Some(2));
    state.get_object_mut(dead_geist).unwrap().name = "Geist of Saint Traft".into();
    let aura_id = reg.get_id_by_name("Pacifism").unwrap();
    let aura = state.create_object(aura_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().name = "Pacifism".into();
    state.get_object_mut(aura).unwrap().attached_to = Some(bear);
    let blade_id = reg.get_id_by_name("Trepanation Blade").unwrap();
    let blade = state.create_object(blade_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(blade).unwrap().name = "Trepanation Blade".into();
    state.get_object_mut(blade).unwrap().attached_to = Some(bear);
    named_permanent(&mut state, &reg, "Liliana of the Veil", P1);

    // Libraries with real order, hands, graveyards, for both players.
    for p in [P0, P1] {
        for name in ["Island", "Swamp", "Plains"] {
            let c = spell_in_hand(&mut state, &reg, name, p);
            state.get_object_mut(c).unwrap().zone = Zone::Library;
            state.get_player_mut(p).library_order.push(c);
        }
        spell_in_hand(&mut state, &reg, "Moment of Heroism", p);
        let dead = spell_in_hand(&mut state, &reg, "Victim of Night", p);
        state.get_object_mut(dead).unwrap().zone = Zone::Graveyard;
    }

    assert_eq!(check_core(&state, &reg), Vec::<String>::new());
    assert_eq!(check_settled(&state, &reg), Vec::<String>::new());

    // A spell properly on the stack passes check_core too.
    let bolt = castable_spell(&mut state, &reg, "Moment of Heroism", P0);
    let mut state2 = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(bear)]);
    // The loop collects triggers before it asks anyone; the fixture stops short of that.
    state2.trigger_event_index = state2.events.len();
    assert_eq!(check_core(&state2, &reg), Vec::<String>::new());

    // A healthy declared combat passes check_settled at the combat steps.
    let mut combat_state = game_at_step(Step::DeclareBlockers, P0);
    combat_state.turn_number = 3;
    let attacker = named_permanent(&mut combat_state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut combat_state, &reg, "Grizzly Bears", P1);
    mtg_engine::combat::declare_attackers(&mut combat_state, &[(attacker, P1)], &[], &reg);
    mtg_engine::combat::declare_blockers(&mut combat_state, &[(blocker, attacker)]);
    combat_state.trigger_event_index = combat_state.events.len();
    assert_eq!(check_settled(&combat_state, &reg), Vec::<String>::new());
}

#[test]
fn object_id_allocation_and_player_indexing_are_checked() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P0, 1, 1);
    state.next_object_id = 0;
    assert_flags(&state, &reg, "next_object_id");

    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.active_player = mtg_engine::ids::PlayerId(9);
    assert_flags(&state, &reg, "active_player 9 out of range");
    state.active_player = P0;
    state.priority_player = Some(mtg_engine::ids::PlayerId(9));
    assert_flags(&state, &reg, "priority_player 9 out of range");
}

#[test]
fn the_library_bijection_is_checked_in_both_directions() {
    let reg = registry();

    // An id listed twice.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let card = spell_in_hand(&mut state, &reg, "Forest", P0);
    state.get_object_mut(card).unwrap().zone = Zone::Library;
    state.get_player_mut(P0).library_order.push(card);
    state.get_player_mut(P0).library_order.push(card);
    assert_flags(&state, &reg, "listed twice");

    // A listing with no object behind it.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P0).library_order.push(ObjectId(99_999));
    assert_flags(&state, &reg, "missing object");

    // A listing whose object is in another zone.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c = ready_creature(&mut state, P0, 1, 1);
    state.get_player_mut(P0).library_order.push(c);
    assert_flags(&state, &reg, "but its zone is");

    // A listing whose object belongs to the other player.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let theirs = spell_in_hand(&mut state, &reg, "Island", P1);
    state.get_object_mut(theirs).unwrap().zone = Zone::Library;
    state.get_player_mut(P0).library_order.push(theirs);
    assert_flags(&state, &reg, "owned by");

    // An object in the library zone that no order lists.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let stray = spell_in_hand(&mut state, &reg, "Swamp", P0);
    state.get_object_mut(stray).unwrap().zone = Zone::Library;
    assert_flags(&state, &reg, "not in library_order");
}

#[test]
fn stack_accounting_is_checked_in_both_directions() {
    let reg = registry();

    // A stack entry whose object is elsewhere (and one with no object).
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let in_hand = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    state.stack.push(StackEntry::Spell(in_hand));
    assert_flags(&state, &reg, "is in zone");
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.stack.push(StackEntry::Spell(ObjectId(99_999)));
    assert_flags(&state, &reg, "has no object");

    // An object in the stack zone that no entry, resolution, or pending
    // cast accounts for.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let orphan = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    state.get_object_mut(orphan).unwrap().zone = Zone::Stack;
    assert_flags(&state, &reg, "on no stack entry");
}

#[test]
fn bookkeeping_watermarks_and_forbidden_zones_are_checked() {
    let reg = registry();

    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.trigger_event_index = state.events.len() + 1;
    assert_flags(&state, &reg, "past the");

    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(c).unwrap().attacked_on_turn = Some(state.turn_number + 5);
    assert_flags(&state, &reg, "future turn");

    let mut state = game_at_step(Step::PrecombatMain, P0);
    let lost = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(lost).unwrap().zone = Zone::Command;
    assert_flags(&state, &reg, "command zone");
}

#[test]
fn attachment_shape_violations_are_checked() {
    let reg = registry();

    // Attached to an object and a player at once.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let a = ready_creature(&mut state, P0, 1, 1);
    let b = ready_creature(&mut state, P0, 1, 1);
    {
        let o = state.get_object_mut(a).unwrap();
        o.attached_to = Some(b);
        o.attached_to_player = Some(P1);
    }
    assert!(check_core(&state, &reg).iter().any(|m| m.contains("attached to both")));

    // A two-object attachment cycle.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let x = ready_creature(&mut state, P0, 1, 1);
    let y = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(x).unwrap().attached_to = Some(y);
    state.get_object_mut(y).unwrap().attached_to = Some(x);
    assert!(check_core(&state, &reg).iter().any(|m| m.contains("attachment cycle")));
}

#[test]
fn combat_maps_may_only_name_declared_attackers() {
    let reg = registry();
    let mut state = game_at_step(Step::CombatDamage, P0);
    let never_attacked = ready_creature(&mut state, P0, 1, 1);
    let mut combat = CombatState::new();
    combat.blocked_attackers.insert(never_attacked);
    state.combat = Some(combat);
    assert!(check_core(&state, &reg).iter().any(|m| m.contains("never attacked")));

    let mut state = game_at_step(Step::CombatDamage, P0);
    let ghost = ready_creature(&mut state, P0, 1, 1);
    let mut combat = CombatState::new();
    combat.blocker_assignments.insert(ghost, vec![]);
    state.combat = Some(combat);
    assert!(check_core(&state, &reg).iter().any(|m| m.contains("never attacked")));

    let mut state = game_at_step(Step::CombatDamage, P0);
    let walker_hunter = ready_creature(&mut state, P0, 1, 1);
    let mut combat = CombatState::new();
    combat.planeswalker_defenders.insert(walker_hunter, ObjectId(77));
    state.combat = Some(combat);
    assert!(check_core(&state, &reg).iter().any(|m| m.contains("never attacked")));
}

#[test]
fn a_token_outside_the_battlefield_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let t = ready_creature(&mut state, P0, 1, 1);
    {
        let o = state.get_object_mut(t).unwrap();
        o.is_token = true;
        o.zone = Zone::Graveyard;
    }
    assert_flags(&state, &reg, "still exists");
}

#[test]
fn battlefield_only_markings_off_the_battlefield_are_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dead = spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    {
        let o = state.get_object_mut(dead).unwrap();
        o.zone = Zone::Graveyard;
        o.tapped = true;
        o.damage_marked = 2;
        o.attached_to = Some(ObjectId(1));
        o.counters.insert(CounterType::PlusOnePlusOne, 1);
        o.regeneration_shields = 1;
    }
    for needle in ["tapped in", "damage marked in", "still attached in",
                   "still has counters in", "regeneration shield in"] {
        assert_flags(&state, &reg, needle);
    }
}

#[test]
fn unapplied_loss_conditions_are_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P1).life = 0;
    assert_flags(&state, &reg, "has not lost");

    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.get_player_mut(P1).has_drawn_from_empty = true;
    assert_flags(&state, &reg, "drew from an empty");
}

#[test]
fn a_creature_that_should_be_dead_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let hurt = ready_creature(&mut state, P0, 2, 2);
    state.get_object_mut(hurt).unwrap().damage_marked = 2;
    assert_flags(&state, &reg, "alive with 2 damage");

    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P0, 0, 0);
    assert_flags(&state, &reg, "alive at toughness");

    // Deathtouch damage of any size (CR 704.5h).
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let nicked = ready_creature(&mut state, P0, 3, 3);
    {
        let o = state.get_object_mut(nicked).unwrap();
        o.damage_marked = 1;
        o.dealt_deathtouch_damage = true;
    }
    assert_flags(&state, &reg, "deathtouch damage");
}

/// CR 614.12b: a permanent whose enters-as-a-copy choice is still queued has
/// not entered. There is no exemption to leak any more — being on the
/// battlefield in that state is itself the violation.
#[test]
fn a_permanent_on_the_battlefield_awaiting_its_copy_choice_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let queued = ready_creature(&mut state, P0, 0, 0);
    state.pending_entry_choices.push(queued);
    assert_flags(&state, &reg, "queued for its enters-as-copy choice");
}

#[test]
fn combat_state_outside_combat_steps_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.combat = Some(CombatState::new());
    assert_flags(&state, &reg, "combat state present");
}

#[test]
fn combat_controller_drift_is_flagged() {
    let reg = registry();

    // An attacker controlled by the non-active player.
    let mut state = game_at_step(Step::CombatDamage, P0);
    let theirs = ready_creature(&mut state, P1, 2, 2);
    let mut combat = CombatState::new();
    combat.attackers.insert(theirs, P1);
    combat.blocker_assignments.insert(theirs, vec![]);
    state.combat = Some(combat);
    assert_flags(&state, &reg, "not the active player");

    // A blocker controlled by someone other than the defending player.
    let mut state = game_at_step(Step::CombatDamage, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let fake_blocker = ready_creature(&mut state, P0, 2, 2);
    let mut combat = CombatState::new();
    combat.attackers.insert(attacker, P1);
    combat.blocker_assignments.insert(attacker, vec![fake_blocker]);
    combat.blocked_attackers.insert(attacker);
    state.combat = Some(combat);
    assert_flags(&state, &reg, "not the defending player");
}

#[test]
fn triggers_still_queued_at_priority_are_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut state, P0, 1, 1);
    let card_id = state.get_object(src).unwrap().card_id;
    // All three queues count toward "still queued".
    let make = |desc: &str| mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(src, card_id, P0, desc),
        mtg_engine::triggers::TriggerEvent::Upkeep,
    );
    state.pending_triggers.push(make("queued"));
    state.pending_trigger_pushes_ap.push(make("ap push"));
    state.pending_trigger_pushes_nap.push(make("nap push"));
    state.priority_player = Some(P0);
    state.awaiting_action = None;
    let v = check_settled(&state, &reg);
    assert!(v.iter().any(|m| m.contains("3 collected trigger(s) still queued")),
        "all three queues are counted: {v:?}");

    // The clause is about a player HOLDING PRIORITY with triggers still
    // queued. Mid-resolution nobody does, and a prompt is the choice the
    // queue is waiting on — neither is a violation.
    let needle = "collected trigger(s) still queued";
    let mut s = state.clone();
    s.priority_player = None;
    assert!(!check_settled(&s, &reg).iter().any(|m| m.contains(needle)),
        "no one holds priority mid-resolution, so the queue may still be full");

    let mut s = state.clone();
    s.awaiting_action = Some(mtg_engine::state::AwaitingAction::MulliganDecision { player: P0 });
    assert!(!check_settled(&s, &reg).iter().any(|m| m.contains(needle)),
        "a queue waiting on a prompt is the prompt doing its job");
}

#[test]
fn a_creature_without_power_or_toughness_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    // A registry card with no printed P/T that has become a creature: the
    // death rules can't compare damage against anything.
    let blank = named_permanent(&mut state, &reg, "Forest", P0);
    state.get_object_mut(blank).unwrap().card_types.push(CardType::Creature);
    assert_flags(&state, &reg, "no power/toughness");

    // Half a P/T box is no better: the death rules need both numbers, so
    // either one missing is the violation.
    for (power, toughness) in [(Some(2), None), (None, Some(2))] {
        let mut s = game_at_step(Step::PrecombatMain, P0);
        let half = named_permanent(&mut s, &reg, "Forest", P0);
        s.get_object_mut(half).unwrap().card_types.push(CardType::Creature);
        s.get_object_mut(half).unwrap().power = power;
        s.get_object_mut(half).unwrap().toughness = toughness;
        assert_flags(&s, &reg, "no power/toughness");
    }
}

#[test]
fn attachment_and_loyalty_type_rules_are_flagged() {
    let reg = registry();

    // A creature "attached" to another object.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c = ready_creature(&mut state, P0, 1, 1);
    let host = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(c).unwrap().attached_to = Some(host);
    assert_flags(&state, &reg, "no Aura or Equipment");

    // Loyalty counters on a non-planeswalker.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let imposter = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(imposter).unwrap().counters.insert(CounterType::Loyalty, 3);
    assert_flags(&state, &reg, "no planeswalker");
}

#[test]
fn a_planeswalker_at_zero_loyalty_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let walker = named_permanent(&mut state, &reg, "Liliana of the Veil", P0);
    // named_permanent seeds the printed starting loyalty; drain it.
    state.get_object_mut(walker).unwrap().counters.remove(&CounterType::Loyalty);
    assert_flags(&state, &reg, "alive at 0 loyalty");
}

#[test]
fn the_legend_rule_left_unapplied_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let card_id = reg.get_id_by_name("Geist of Saint Traft").unwrap();
    for _ in 0..2 {
        let id = state.create_object(card_id, P0, Zone::Battlefield, Some(2), Some(2));
        state.get_object_mut(id).unwrap().name = "Geist of Saint Traft".into();
    }
    state.awaiting_action = None;
    assert_flags(&state, &reg, "two legendary");
}

#[test]
fn opposing_counters_that_should_annihilate_are_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let c = ready_creature(&mut state, P0, 3, 3);
    {
        let o = state.get_object_mut(c).unwrap();
        o.counters.insert(CounterType::PlusOnePlusOne, 1);
        o.counters.insert(CounterType::MinusOneMinusOne, 1);
    }
    assert_flags(&state, &reg, "both +1/+1 and -1/-1");
}

#[test]
fn aura_and_equipment_attachment_states_are_flagged() {
    let reg = registry();

    // An Aura on the battlefield with no host at all.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let pacifism_id = reg.get_id_by_name("Pacifism").unwrap();
    let aura = state.create_object(pacifism_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(aura).unwrap().name = "Pacifism".into();
    assert_flags(&state, &reg, "battlefield unattached");

    // An Aura attached to something that left.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ghost = ready_creature(&mut state, P0, 1, 1);
    state.get_object_mut(ghost).unwrap().zone = Zone::Graveyard;
    let aura = state.create_object(pacifism_id, P0, Zone::Battlefield, None, None);
    {
        let o = state.get_object_mut(aura).unwrap();
        o.name = "Pacifism".into();
        o.attached_to = Some(ghost);
    }
    assert_flags(&state, &reg, "not on the battlefield");

    // Equipment attached to a non-creature.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let blade_id = reg.get_id_by_name("Trepanation Blade").unwrap();
    let blade = state.create_object(blade_id, P0, Zone::Battlefield, None, None);
    let land = named_permanent(&mut state, &reg, "Forest", P0);
    {
        let o = state.get_object_mut(blade).unwrap();
        o.name = "Trepanation Blade".into();
        o.attached_to = Some(land);
    }
    assert_flags(&state, &reg, "non-creature");
}

// Silence the unused-import warning if CardId ends up unneeded on some
// configurations: it is used in this file's type ascriptions above.
#[allow(dead_code)]
fn _use(_: CardId) {}

// ── The 2026-09-02 batch's oracle additions ─────────────────────────

/// CR 508.1a/508.2: a repeated id in AttackersDeclared fires the attack
/// trigger once per repeat (issue #108's shape). The event buffer is the
/// only place this is visible — combat's own maps dedupe.
#[test]
fn duplicate_attackers_in_the_declared_event_are_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    let a = ready_creature(&mut state, P0, 2, 2);
    state.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(a, P1), (a, P1)],
    });
    assert_flags(&state, &reg, "more than once");
}

/// CR 509.1b: one blocker, one block. A blocker listed under two attackers
/// (or twice under one) is flagged.
#[test]
fn a_double_assigned_blocker_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let a1 = ready_creature(&mut state, P0, 2, 2);
    let a2 = ready_creature(&mut state, P0, 2, 2);
    let b = ready_creature(&mut state, P1, 2, 2);
    let mut combat = CombatState::default();
    combat.attackers.insert(a1, P1);
    combat.attackers.insert(a2, P1);
    combat.blocker_assignments.insert(a1, vec![b]);
    combat.blocker_assignments.insert(a2, vec![b]);
    state.combat = Some(combat);
    assert_flags(&state, &reg, "at once (CR 509.1b)");

    let mut combat = CombatState::default();
    combat.attackers.insert(a1, P1);
    combat.blocker_assignments.insert(a1, vec![b, b]);
    state.combat = Some(combat);
    assert_flags(&state, &reg, "listed twice against attacker");
}

/// Every life transition goes through change_life and its LifeChanged
/// event; a broken chain or a final link that disagrees with the actual
/// life total means a life change bypassed the pipeline (issue #129's
/// family, mechanically checked).
#[test]
fn a_life_change_that_bypassed_the_event_chain_is_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Chain break: 20 -> 18, then an event claiming to start from 19.
    state.events.push(mtg_engine::events::GameEvent::LifeChanged {
        player: P0, old: 20, new_life: 18 });
    state.events.push(mtg_engine::events::GameEvent::LifeChanged {
        player: P0, old: 19, new_life: 17 });
    state.get_player_mut(P0).life = 17;
    assert_flags(&state, &reg, "LifeChanged chain breaks");

    // Final link disagrees with the actual total.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.events.push(mtg_engine::events::GameEvent::LifeChanged {
        player: P0, old: 20, new_life: 18 });
    // life is still 20: something moved it back without an event, or the
    // event lied.
    assert_flags(&state, &reg, "but life is");
}

/// A prompt with nothing to choose is a stuck game; an X-funding prompt
/// without its stash panics when answered; a stash without its prompt is a
/// leak (the #123 cancel path must clear both together).
#[test]
fn incoherent_prompts_and_stashes_are_flagged() {
    let reg = registry();

    // Empty choice.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut state, P0, 1, 1);
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        player: P0,
        source: src,
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseTarget {
            description: "pick".into(),
            options: vec![],
            optional: false,
            effect: mtg_engine::state::PendingEffect::CardEffect { source_id: src, key: String::new() },
        },
    });
    assert_flags(&state, &reg, "nothing to choose");

    // The same clause, for the damage-effect order prompt (CR 616.1). Each
    // kind of choice is its own arm, so each needs its own state: a blinded
    // arm leaves the fuzzer reporting nothing while a game sits on a prompt
    // that cannot be answered.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut state, P0, 1, 1);
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        player: P0,
        source: src,
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseDamageEffect {
            description: "which applies first".into(),
            effects: vec![],
            options: vec![],
            source: src,
            target: mtg_engine::events::DamageTarget::Player(P0),
            amount: 2,
            kind: mtg_engine::damage::DamageKind::NonCombat,
        },
    });
    assert_flags(&state, &reg, "nothing to choose");

    // Spell funding prompt with no stashed cast.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut state, P0, 1, 1);
    state.awaiting_action = Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
        player: P0,
        source: src,
        choice: mtg_engine::state::ResolutionChoiceKind::ChooseXFunding {
            description: "fund".into(),
            options: mtg_engine::funding::FundingOptions {
                pool: std::collections::BTreeMap::new(),
                groups: vec![],
                max_x: 1,
                x_discount: 0,
            },
            source_id: src,
            is_ability: false,
        },
    });
    assert_flags(&state, &reg, "no pending_spell_cast");

    // Stash with no prompt: build the real funding prompt via a cast, then
    // drop only the prompt — the leak the invariant exists to catch.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    for _ in 0..2 {
        named_permanent(&mut state, &reg, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    let cast = legal.actions.iter().find(|a| matches!(a,
        mtg_engine::actions::Action::CastSpell { object_id, .. } if *object_id == dp))
        .expect("Devil's Play castable").clone();
    let mut state = mtg_engine::engine::submit_action(&state, &cast, &reg);
    assert!(state.pending_spell_cast.is_some(), "setup: the cast is stashed");
    state.awaiting_action = None;
    assert_flags(&state, &reg, "leak");
}

/// Every kind of prompt that can be raised with an empty list is a stuck
/// game, not just the target picker. Each arm of the `empty` match is its
/// own clause; the suite pinned only `ChooseTarget`.
#[test]
fn every_kind_of_empty_prompt_is_flagged() {
    use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind as K};

    let reg = registry();
    let mut base = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut base, P0, 1, 1);

    let empties = [
        ("a target picker", K::ChooseTarget {
            description: "pick".into(),
            options: vec![],
            optional: false,
            effect: PendingEffect::CardEffect { source_id: src, key: String::new() },
        }),
        ("cards looked at", K::ChooseFromLookedAt {
            description: "keep one".into(),
            looked_at: vec![],
        }),
        ("a discard", K::ChooseCardFromHand {
            description: "discard".into(),
            player: P0,
            cards: vec![],
            discard_immediately: true,
            remaining: 1,
        }),
        ("a trigger order", K::ChooseTriggerOrder {
            description: "order".into(),
            options: vec![],
            ap_queue: true,
            indices: vec![],
            details: vec![],
        }),
        ("a damage assignment order", K::ChooseDamageAssignmentOrder {
            description: "order".into(),
            attacker: src,
            remaining: vec![],
            options: vec![],
        }),
        ("a pile division", K::DividePermanentsIntoPiles {
            description: "divide".into(),
            permanents: vec![],
            target_player: P1,
            source_id: src,
        }),
    ];

    for (what, choice) in empties {
        let mut state = base.clone();
        state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: P0, source: src, choice,
        });
        let v = check_settled(&state, &reg);
        assert!(v.iter().any(|m| m.contains("nothing to choose")),
            "{what} with an empty list is a stuck game, got: {v:?}");
    }

    // The same prompts with something in them are fine.
    base.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0, source: src,
        choice: K::ChooseTriggerOrder {
            description: "order".into(),
            options: vec!["a".into(), "b".into()],
            ap_queue: true,
            indices: vec![0, 1],
            details: vec![],
        },
    });
    assert!(!check_settled(&base, &reg).iter().any(|m| m.contains("nothing to choose")),
        "a prompt with options is not a stuck game");
}

/// An X-funding prompt is answered by reaching for the stash it was raised
/// with; without it, answering panics. The ability half of that pairing was
/// unpinned — only the spell half had a test.
#[test]
fn an_ability_funding_prompt_without_its_stash_is_flagged() {
    use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind as K};

    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut state, P0, 1, 1);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0,
        source: src,
        choice: K::ChooseXFunding {
            description: "fund".into(),
            options: mtg_engine::funding::FundingOptions {
                pool: std::collections::BTreeMap::new(),
                groups: vec![],
                max_x: 1,
                x_discount: 0,
            },
            source_id: src,
            is_ability: true,
        },
    });

    assert_flags(&state, &reg, "no pending_ability_effect stashed");
    assert!(!check_settled(&state, &reg).iter()
        .any(|m| m.contains("no pending_spell_cast stashed")),
        "an ability prompt is not missing a SPELL stash");
}

/// Every prompt names the player who has to answer it, and a prompt
/// addressed to a seat that does not exist is unanswerable — the game is
/// stuck and, worse, reading through the id panics.
#[test]
fn a_prompt_addressed_to_no_one_is_flagged() {
    use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind as K};

    let reg = registry();
    let mut base = game_at_step(Step::PrecombatMain, P0);
    let src = ready_creature(&mut base, P0, 1, 1);
    let ghost = mtg_engine::ids::PlayerId(u8::try_from(base.players.len()).unwrap());

    // A resolution choice.
    let mut state = base.clone();
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: ghost,
        source: src,
        choice: K::ChooseTarget {
            description: "pick".into(),
            options: vec![Target::Object(src)],
            optional: false,
            effect: PendingEffect::CardEffect { source_id: src, key: String::new() },
        },
    });
    assert_flags(&state, &reg, "awaiting_action prompts out-of-range p2");

    // And each of the three prompts that share the guarded arm.
    for (what, awaiting) in [
        ("a mulligan", AwaitingAction::MulliganDecision { player: ghost }),
        ("bottoming after a mulligan", AwaitingAction::BottomAfterMulligan {
            player: ghost, count: 1 }),
        ("a discard to hand size", AwaitingAction::DiscardToHandSize {
            player: ghost, discard_count: 1 }),
    ] {
        let mut state = base.clone();
        state.awaiting_action = Some(awaiting);
        let v = check_settled(&state, &reg);
        assert!(v.iter().any(|m| m.contains("awaiting_action prompts out-of-range p2")),
            "{what} addressed to a seat that does not exist, got: {v:?}");
    }

    // The same prompts addressed to a real seat say nothing.
    let mut state = base.clone();
    state.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P1 });
    assert!(!check_settled(&state, &reg).iter().any(|m| m.contains("out-of-range")),
        "a mulligan prompt for a real seat is fine");
}

/// The checker runs on states that are already corrupt — that is its job —
/// so every id it reads through is range-checked first. A player id one
/// past the end is the case that turns a missing check into a panic instead
/// of a message.
#[test]
fn a_player_id_past_the_last_seat_is_reported_not_dereferenced() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let ghost = mtg_engine::ids::PlayerId(u8::try_from(state.players.len()).unwrap());

    state.num_spells_cast_this_turn.insert(ghost, 1);
    assert_flags(&state, &reg, "spell count for p2 who is not a player");

    // A life-change event for a seat that does not exist: the last-link
    // check has to range-check before it compares against that seat's life.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.events.push(mtg_engine::events::GameEvent::LifeChanged {
        player: ghost, old: 20, new_life: 18 });
    let v = check_settled(&state, &reg);
    assert!(!v.iter().any(|m| m.contains("last LifeChanged says")),
        "a seat that does not exist has no life to disagree with, got: {v:?}");
}

/// Everything in the stack zone is accounted for by exactly one thing: a
/// stack entry, the resolution in progress, or the cast still being paid
/// for — and the cast that excuses it has to be the cast OF it.
#[test]
fn a_half_cast_spell_excuses_only_itself_from_the_stack_zone() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let dp = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    let decoy = spell_in_hand(&mut state, &reg, "Devil's Play", P0);
    for _ in 0..2 {
        named_permanent(&mut state, &reg, "Mountain", P0);
    }
    let _ = ready_creature(&mut state, P1, 2, 2);

    let cast = mtg_engine::engine::legal_actions(&state, &reg).actions.into_iter()
        .find(|a| matches!(a, mtg_engine::actions::Action::CastSpell { object_id, .. } if *object_id == dp))
        .expect("Devil's Play castable");
    let mut state = mtg_engine::engine::submit_action(&state, &cast, &reg);
    assert!(state.pending_spell_cast.is_some(), "setup: the cast is stashed");

    // A cast that is still being paid for can have the card in the stack
    // zone already — it is on no stack entry until the payment completes.
    state.get_object_mut(dp).unwrap().zone = Zone::Stack;
    assert!(!state.stack.iter().any(|e| e.as_spell() == Some(dp)),
        "setup: and is on no stack entry yet");
    assert!(!check_core(&state, &reg).iter()
        .any(|m| m.contains("in stack zone but on no stack entry")),
        "the cast in progress accounts for it");

    // The same spell in the stack zone with the stash naming a different
    // card is unaccounted for.
    let mut s = state.clone();
    s.pending_spell_cast.as_mut().unwrap().object_id = decoy;
    assert_flags(&s, &reg, "in stack zone but on no stack entry");
}

/// CR 704.3: state-based actions look at permanents on the battlefield.
/// A creature card anywhere else has no toughness to die of, however small
/// its characteristic-defining ability makes it.
#[test]
fn a_zero_toughness_creature_card_off_the_battlefield_is_not_a_dead_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Geist-Honored Monk's P/T is the number of creatures you control
    // (CR 208.2), so with an empty battlefield the card in the graveyard
    // reads 0/0 — a size no permanent could survive.
    let card_id = reg.get_id_by_name("Geist-Honored Monk").unwrap();
    let monk = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(monk).unwrap().name = "Geist-Honored Monk".into();
    assert_eq!(state.effective_toughness(monk, &reg), Some(0),
        "setup: the CDA reads 0 with no creatures on the battlefield");
    assert!(state.is_creature(monk, &reg), "setup: it is still a creature card");

    let v = check_settled(&state, &reg);
    assert!(!v.iter().any(|m| m.contains("alive at toughness")),
        "a creature card in the graveyard is not a permanent the death check \
         applies to, got: {v:?}");
}

/// The bookkeeping that has to be internally consistent before anything
/// reads it: the object map, the library's two views, and the attachment
/// graph.
#[test]
fn the_object_map_and_the_library_views_agree_with_themselves() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let library = stock_library(&mut state, &reg, P0, 3);
    for id in &library {
        state.get_object_mut(*id).unwrap().name = "Forest".into();
    }

    // An object filed under an id that is not its own: the map key and the
    // object's own id are the same handle, or nothing that looks one up by
    // id gets the object it asked for.
    let mut s = state.clone();
    let stray = s.get_object(bear).unwrap().clone();
    s.objects.insert(ObjectId(4242), stray);
    assert_flags(&s, &reg, "not stored under its own id");

    // And an id the allocator has not handed out yet.
    let mut s = state.clone();
    let mut ahead = s.get_object(bear).unwrap().clone();
    ahead.id = ObjectId(4242);
    s.objects.insert(ObjectId(4242), ahead);
    assert_flags(&s, &reg, "at or past next_object_id");

    // CR 401.2: the library's order and its zone are the same set, both ways.
    let mut s = state.clone();
    s.get_player_mut(P0).library_order.push(library[0]);
    assert_flags(&s, &reg, "listed twice in library_order");

    let mut s = state.clone();
    s.get_player_mut(P0).library_order.push(ObjectId(4242));
    assert_flags(&s, &reg, "library_order lists missing object 4242");

    let mut s = state.clone();
    s.get_object_mut(library[0]).unwrap().zone = Zone::Hand;
    assert_flags(&s, &reg, "but its zone is Hand");

    let mut s = state.clone();
    s.get_player_mut(P0).library_order.retain(|id| *id != library[0]);
    assert_flags(&s, &reg, "in library zone but not in library_order");

    let mut s = state.clone();
    s.get_object_mut(library[0]).unwrap().owner = P1;
    assert_flags(&s, &reg, "library_order lists");

    // Attached to an object and to a player at once.
    let mut s = state.clone();
    let aura = named_permanent(&mut s, &reg, "Pacifism", P0);
    s.get_object_mut(aura).unwrap().attached_to = Some(bear);
    s.get_object_mut(aura).unwrap().attached_to_player = Some(P1);
    assert_flags(&s, &reg, "attached to both an object and a player");

    // An attachment ring that never reaches a host standing on its own.
    let mut s = state.clone();
    let a = named_permanent(&mut s, &reg, "Pacifism", P0);
    let b = named_permanent(&mut s, &reg, "Pacifism", P0);
    s.get_object_mut(a).unwrap().attached_to = Some(b);
    s.get_object_mut(b).unwrap().attached_to = Some(a);
    assert_flags(&s, &reg, "sits in an attachment cycle");

    // A stack entry whose object is not in the stack zone.
    let mut s = state.clone();
    s.stack.push(StackEntry::Spell(bear));
    assert_flags(&s, &reg, "stack entry Spell(");

    // Nothing in this pool uses the command zone.
    let mut s = state.clone();
    s.get_object_mut(library[0]).unwrap().zone = Zone::Command;
    assert_flags(&s, &reg, "in the unused command zone");

    // A stamp from a turn that has not happened.
    let mut s = state.clone();
    s.get_object_mut(bear).unwrap().attacked_on_turn = Some(state.turn_number + 5);
    assert_flags(&s, &reg, "attacked on future turn");
}

/// Combat's maps only ever name creatures that were declared as attackers,
/// combat itself only exists in the combat steps, and a declaration names
/// each attacker once (issue #108).
#[test]
fn combats_maps_only_name_declared_attackers() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareBlockers, P0);
    let attacker = ready_creature(&mut state, P0, 2, 2);
    let bystander = ready_creature(&mut state, P0, 2, 2);
    let mut c = CombatState::new();
    c.any_attackers_declared = true;
    c.attackers.insert(attacker, P1);
    state.combat = Some(c);

    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocked_attackers.insert(bystander);
    assert_flags(&s, &reg, "blocked_attackers holds");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocker_assignments.insert(bystander, vec![]);
    assert_flags(&s, &reg, "blocker_assignments holds");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().planeswalker_defenders.insert(bystander, attacker);
    assert_flags(&s, &reg, "planeswalker_defenders holds");

    // CR 508.1a: combat exists only in the combat steps.
    let mut s = state.clone();
    s.step = Step::PrecombatMain;
    assert_flags(&s, &reg, "combat state present in step PrecombatMain");

    // CR 508.1: one entry per attacker in the declaration.
    let mut s = state.clone();
    s.events.push(mtg_engine::events::GameEvent::AttackersDeclared {
        attackers: vec![(attacker, P1), (attacker, P1)] });
    assert_flags(&s, &reg, "AttackersDeclared lists attacker");
}

/// CR 704.5i/704.5n: a planeswalker at zero loyalty is gone, and an Aura on
/// the battlefield attached to nothing is gone with it.
#[test]
fn a_dead_planeswalker_and_an_unattached_aura_are_flagged() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let aura = named_permanent(&mut state, &reg, "Pacifism", P0);
    state.get_object_mut(aura).unwrap().attached_to = Some(bear);

    let mut s = state.clone();
    s.get_object_mut(aura).unwrap().attached_to = None;
    assert_flags(&s, &reg, "on the battlefield unattached");

    let mut s = state.clone();
    let walker = named_permanent(&mut s, &reg, "Liliana of the Veil", P0);
    s.get_object_mut(walker).unwrap().counters.insert(CounterType::Loyalty, 0);
    assert_flags(&s, &reg, "alive at 0 loyalty");
}
