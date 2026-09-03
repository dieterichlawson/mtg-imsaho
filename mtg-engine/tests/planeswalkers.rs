//! Planeswalkers as a card type: the loyalty state-based action (CR 704.5i)
//! and loyalty-ability activation (CR 606).
//!
//! These are engine rules, not card behaviour. They used Liliana of the Veil
//! as a fixture and lived in `cards_complex_creatures.rs` among tests about
//! what particular creatures do; the card is incidental here — any
//! planeswalker would serve.

mod common;
use common::*;
use mtg_engine::actions::Action;
use mtg_engine::engine;
use mtg_engine::sba::check_state_based_actions;
use mtg_engine::types::*;

#[test]
fn planeswalker_with_zero_loyalty_dies() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    // 0 loyalty counters.

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Graveyard);
}

#[test]
fn planeswalker_with_loyalty_survives() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    set_loyalty(&mut state, liliana, 3);

    check_state_based_actions(&mut state, &reg);

    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Battlefield);
}

#[test]
fn loyalty_abilities_appear_in_legal_actions() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    set_loyalty(&mut state, liliana, 3);

    let legal = engine::legal_actions(&state, &reg);

    // Should have loyalty ability actions.
    let loyalty_actions: Vec<_> = legal.actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { .. }))
        .collect();
    // +1 (no target) + -2 targeting P0 + -2 targeting P1 = at least 3 actions.
    // (Not -6, since loyalty is only 3.)
    assert!(loyalty_actions.len() >= 3, "Expected at least 3 loyalty actions (+1 untargeted, -2 x2 player targets), got {}", loyalty_actions.len());

    // Verify +1 has no targets and -2 has player targets.
    let plus_one: Vec<_> = loyalty_actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { ability_index: 0, .. }))
        .collect();
    assert_eq!(plus_one.len(), 1, "+1 should have exactly one action (untargeted)");

    let minus_two: Vec<_> = loyalty_actions.iter()
        .filter(|a| matches!(a, Action::ActivateLoyaltyAbility { ability_index: 1, .. }))
        .collect();
    assert_eq!(minus_two.len(), 2, "-2 should have two actions (targeting P0 and P1)");
}

/// CR 606.5: a loyalty ability is an activated ability and uses the stack.
/// Activating pays the cost immediately (CR 601.2h) but the effect waits for
/// resolution — the opponent gets a response window in between.
#[test]
fn loyalty_ability_uses_the_stack() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    set_loyalty(&mut state, liliana, 3);
    let _p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);

    let state = engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana,
        ability_index: 0,
        targets: vec![],
    }, &reg);

    // The cost is paid, the effect is not: the +1's discard has not happened.
    assert_eq!(state.get_counter_count(liliana, CounterType::Loyalty), 4);
    assert_eq!(state.stack.len(), 1, "the ability waits on the stack (CR 606.5)");
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 1,
        "no discard before the ability resolves");

    // Every player passes; the ability resolves and P0 discards.
    let mut state = state;
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);
    assert!(state.stack.is_empty(), "resolved");
    assert_eq!(state.objects_in_zone(Zone::Hand, P0).len(), 0,
        "the +1's discard happened at resolution");
}

/// The fuzz cluster behind issues #143/#146/#159 and seventeen more: Liliana's
/// -6, activated by paying her last 6 loyalty, put *herself* in the pile
/// prompt. The ability resolved on the spot, before CR 704.5i had moved her to
/// the graveyard, so "all permanents target player controls" included a
/// planeswalker that was gone by the time anyone could answer. On the stack,
/// the pile is built from the battlefield as it is at resolution (CR 608.2g
/// aside — this effect reads current state, not last known information).
#[test]
fn liliana_minus_six_pile_excludes_her_own_dead_body() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    set_loyalty(&mut state, liliana, 6);
    let bear = ready_creature(&mut state, P0, 2, 2);

    let state = engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana,
        ability_index: 2,
        targets: vec![mtg_engine::actions::Target::Player(P0)],
    }, &reg);

    // The -6 is on the stack; the loyalty payment took her to 0, and the SBA
    // check that runs before anyone receives priority (CR 117.5, the game
    // loop's job) buries her while the ability still waits.
    let mut state = state;
    assert_eq!(state.stack.len(), 1, "the -6 waits on the stack");
    assert!(state.awaiting_action.is_none(), "no pile prompt before resolution");
    check_state_based_actions(&mut state, &reg);
    assert_eq!(state.get_object(liliana).unwrap().zone, Zone::Graveyard,
        "0 loyalty is a state-based death (CR 704.5i)");
    assert_eq!(state.stack.len(), 1, "her -6 outlives her (CR 113.7a)");

    // Every player passes; the ability resolves.
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    match &state.awaiting_action {
        Some(mtg_engine::state::AwaitingAction::ResolutionChoice {
            choice: mtg_engine::state::ResolutionChoiceKind::DividePermanentsIntoPiles { permanents, .. }, ..
        }) => {
            assert!(permanents.contains(&bear), "the live permanent is in the piles");
            assert!(!permanents.contains(&liliana),
                "a permanent that left the battlefield is not in the piles (CR 700.3c)");
        }
        other => panic!("expected the pile prompt, got {other:?}"),
    }
}

#[test]
fn loyalty_ability_adjusts_counters() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let card_id = reg.get_id_by_name("Liliana of the Veil").unwrap();
    let liliana = state.create_object(card_id, P0, Zone::Battlefield, None, None);
    state.get_object_mut(liliana).unwrap().name = "Liliana of the Veil".into();
    set_loyalty(&mut state, liliana, 3);
    let _p0_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P0);
    let _p1_card = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);

    // Activate +1.
    let new_state = engine::submit_action(&state, &Action::ActivateLoyaltyAbility {
        object_id: liliana,
        ability_index: 0,
        targets: vec![],
    }, &reg);

    // Loyalty should be 4 (3 + 1).
    assert_eq!(new_state.get_counter_count(liliana, CounterType::Loyalty), 4);

}
