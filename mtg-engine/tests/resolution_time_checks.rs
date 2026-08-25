//! Three ways a resolving effect can read the game wrong.
//!
//! - It can act on an object that has moved since the ability was put on the
//!   stack (Moldgraf Monstrosity exiling a creature that is back on the
//!   battlefield).
//! - It can skip a choice the player is entitled to make (Ghost Quarter
//!   shuffling a library whose owner never agreed to search it).
//! - It can decide for a player what they can afford (Frightful Delusion
//!   counting only mana already floating, when CR 608.2g lets the player tap
//!   for it).

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::state::{AwaitingAction, ResolutionChoiceKind};
use mtg_engine::types::*;
// ---------------------------------------------------------------------------
// Moldgraf Monstrosity: "When this creature dies, exile it, then return two
// creature cards at random from your graveyard to the battlefield."
// ---------------------------------------------------------------------------

/// Two Monstrosities die together, so two triggers go on the stack. The first
/// can return the second to the battlefield; the second trigger must then
/// leave it there rather than exiling a live creature.
#[test]
fn moldgraf_simultaneous_death_second_trigger_does_not_exile_live_creature() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let first = named_creature(&mut state, &reg, "Moldgraf Monstrosity", P0);
    let second = named_creature(&mut state, &reg, "Moldgraf Monstrosity", P0);
    let fodder = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);

    mtg_engine::destruction::try_destroy_all(&mut state, &[first, second], &reg);
    assert_eq!(state.get_object(second).unwrap().zone, Zone::Graveyard,
        "test precondition: both died");

    let behavior = reg.get(state.get_object(first).unwrap().card_id).unwrap();

    // First trigger: exiles itself and returns two creature cards. The
    // graveyard holds the second Monstrosity and the fodder, so both come
    // back.
    behavior.on_dies(&mut state, first, &[], &reg);
    assert_eq!(state.get_object(first).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(second).unwrap().zone, Zone::Battlefield,
        "test precondition: the first trigger returned the second Monstrosity");
    assert_eq!(state.get_object(fodder).unwrap().zone, Zone::Battlefield);

    // Second trigger: its card is on the battlefield now, not in a graveyard.
    behavior.on_dies(&mut state, second, &[], &reg);
    assert_eq!(state.get_object(second).unwrap().zone, Zone::Battlefield,
        "the second trigger must not exile a creature that is back on the \
         battlefield — 'exile it' applies to the card in the graveyard");
}

/// The return happens even when the exile can't: an ability does as much as
/// it can (CR 608.2).
#[test]
fn moldgraf_exile_skipped_when_already_exiled_still_returns_creatures() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let monstrosity = named_creature(&mut state, &reg, "Moldgraf Monstrosity", P0);
    let a = named_card_in_graveyard(&mut state, &reg, "Walking Corpse", P0);
    let b = named_card_in_graveyard(&mut state, &reg, "Chapel Geist", P0);

    mtg_engine::destruction::try_destroy(&mut state, monstrosity, &reg);
    // Something else exiles it from the graveyard before the trigger resolves.
    state.move_object(monstrosity, Zone::Exile, &reg);

    reg.get(state.get_object(monstrosity).unwrap().card_id).unwrap()
        .on_dies(&mut state, monstrosity, &[], &reg);

    assert_eq!(state.get_object(a).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(b).unwrap().zone, Zone::Battlefield,
        "the two creature cards come back whether or not the exile happened");
}

// ---------------------------------------------------------------------------
// Ghost Quarter: "Its controller MAY search their library for a basic land
// card, put it onto the battlefield, then shuffle."
// ---------------------------------------------------------------------------

/// With no basic land to find, the controller is still asked, and declining
/// means no search — so no shuffle.
#[test]
fn ghost_quarter_may_choice_offered_when_no_basics() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let quarter = named_creature(&mut state, &reg, "Ghost Quarter", P0);
    let victim = named_creature(&mut state, &reg, "Kessig Wolf Run", P1);

    // P1's library: no basics, and in a known order.
    let library: Vec<_> = ["Chapel Geist", "Walking Corpse", "Avacyn's Pilgrim"].iter()
        .map(|name| {
            let id = state.create_object(reg.get_id_by_name(name).unwrap(), P1, Zone::Library, None, None);
            state.get_player_mut(P1).library_order.push(id);
            id
        })
        .collect();

    reg.get(state.get_object(quarter).unwrap().card_id).unwrap()
        .on_activate_ability(&mut state, quarter, 0, &[Target::Object(victim)], &reg);

    assert!(matches!(&state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice {
            player, choice: ResolutionChoiceKind::ChooseTarget { optional: true, .. }, .. })
        if *player == P1),
        "the land's controller must still be offered the 'may search'; got {:?}",
        state.awaiting_action);

    let actions = mtg_engine::engine::legal_actions(&state, &reg).actions;
    assert!(actions.len() == 1
        && matches!(&actions[0], Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) }),
        "with no basic land to find, declining is the only answer; got {actions:?}");

    let state = mtg_engine::engine::submit_action(&state, &actions[0], &reg);
    assert_eq!(state.get_player(P1).library_order, library,
        "a player who declines to search does not shuffle");
}

/// Declining a search that *could* have found something also skips the
/// shuffle.
#[test]
fn ghost_quarter_declining_the_search_does_not_shuffle() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let quarter = named_creature(&mut state, &reg, "Ghost Quarter", P0);
    let victim = named_creature(&mut state, &reg, "Kessig Wolf Run", P1);

    let library: Vec<_> = ["Forest", "Island", "Chapel Geist", "Walking Corpse"].iter()
        .map(|name| {
            let id = state.create_object(reg.get_id_by_name(name).unwrap(), P1, Zone::Library, None, None);
            state.get_player_mut(P1).library_order.push(id);
            id
        })
        .collect();

    reg.get(state.get_object(quarter).unwrap().card_id).unwrap()
        .on_activate_ability(&mut state, quarter, 0, &[Target::Object(victim)], &reg);

    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::ChosenTarget(None),
    }, &reg);

    assert_eq!(state.get_player(P1).library_order, library,
        "declining means no search happened, so no shuffle");
    assert_eq!(state.get_object(victim).unwrap().zone, Zone::Graveyard,
        "the land is destroyed either way");
}

// ---------------------------------------------------------------------------
// Frightful Delusion: "Counter target spell unless its controller pays {1}."
// ---------------------------------------------------------------------------

/// CR 608.2g: the controller may tap for the {1}. Having nothing floating is
/// not the same as being unable to pay.
#[test]
fn auto_counter_when_controller_has_no_floating_mana_but_has_lands() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // P0's spell on the stack, an untapped Island, and an empty pool.
    let bears = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(bears, Zone::Stack, &reg);
    named_creature(&mut state, &reg, "Island", P0);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "test precondition");

    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    state.move_object(fd, Zone::Stack, &reg);
    reg.get(state.get_object(fd).unwrap().card_id).unwrap()
        .on_resolve(&mut state, fd, &[Target::Object(bears)], &reg);

    assert!(matches!(&state.awaiting_action,
        Some(AwaitingAction::ResolutionChoice {
            player, choice: ResolutionChoiceKind::PayOrNot { .. }, .. }) if *player == P0),
        "the spell's controller must be asked, not auto-countered; got {:?}",
        state.awaiting_action);

    let actions = mtg_engine::engine::legal_actions(&state, &reg).actions;
    assert!(actions.iter().any(|a| matches!(a,
        Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) })),
        "paying must be offered — the Island can produce the {{1}}; got {actions:?}");

    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::PayDecision(true),
    }, &reg);
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Stack,
        "the spell is saved once the {{1}} is paid");
}

/// Positive control: the if-branch works when the mana is already floating.
#[test]
fn player_offered_choice_when_controller_has_floating_mana() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(bears, Zone::Stack, &reg);
    state.get_player_mut(P0).mana_pool.add(ManaType::Blue, 1);

    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    state.move_object(fd, Zone::Stack, &reg);
    reg.get(state.get_object(fd).unwrap().card_id).unwrap()
        .on_resolve(&mut state, fd, &[Target::Object(bears)], &reg);

    let actions = mtg_engine::engine::legal_actions(&state, &reg).actions;
    assert!(actions.iter().any(|a| matches!(a,
        Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) })));

    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::PayDecision(true),
    }, &reg);
    assert_eq!(state.get_player(P0).mana_pool.total(), 0, "the {{1}} was spent");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Stack);
}

/// Saying "pay" without the mana must not save the spell for free — the
/// engine used to ignore whether the payment succeeded.
#[test]
fn claiming_to_pay_without_the_mana_does_not_save_the_spell() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = spell_in_hand(&mut state, &reg, "Walking Corpse", P0);
    state.move_object(bears, Zone::Stack, &reg);

    let fd = spell_in_hand(&mut state, &reg, "Frightful Delusion", P1);
    state.move_object(fd, Zone::Stack, &reg);
    reg.get(state.get_object(fd).unwrap().card_id).unwrap()
        .on_resolve(&mut state, fd, &[Target::Object(bears)], &reg);

    // No lands, no floating mana — "pay" is not even offered, but a
    // hand-built action must still be handled correctly.
    let actions = mtg_engine::engine::legal_actions(&state, &reg).actions;
    assert!(!actions.iter().any(|a| matches!(a,
        Action::ResolveChoice { choice: ResolvedChoice::PayDecision(true) })),
        "paying must not be offered with no way to produce the mana");

    let state = mtg_engine::engine::submit_action(&state, &Action::ResolveChoice {
        choice: ResolvedChoice::PayDecision(true),
    }, &reg);
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Graveyard,
        "an unpayable cost is unpaid, so the spell is countered");
}
