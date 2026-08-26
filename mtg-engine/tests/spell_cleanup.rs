//! Regression tests for engine-owned spell cleanup (CR 608.2m).
//!
//! A spell that presents player choices during resolution must stay on the
//! stack until the choice chain completes; moving it to the graveyard is
//! the FINAL step of resolution and is owned by the engine, not card code.
//! Divine Reckoning previously moved itself to the graveyard before
//! presenting its keep-a-creature choices.

mod common;
use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine;
use mtg_engine::state::AwaitingAction;
use mtg_engine::types::*;

/// Answer the current ChooseTarget resolution choice with a specific object.
fn answer_choice_with(
    state: &mtg_engine::state::GameState,
    reg: &CardRegistry,
    chosen: mtg_engine::ids::ObjectId,
) -> mtg_engine::state::GameState {
    let Some(AwaitingAction::ResolutionChoice { choice, .. }) = &state.awaiting_action else {
        panic!("expected a pending resolution choice, got {:?}", state.awaiting_action);
    };
    let mtg_engine::state::ResolutionChoiceKind::ChooseTarget { options, .. } = choice else {
        panic!("expected ChooseTarget, got {choice:?}");
    };
    let target = Target::Object(chosen);
    assert!(options.contains(&target), "{chosen:?} not among options {options:?}");
    engine::submit_action(
        state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(target)) },
        reg,
    )
}

/// Divine Reckoning must stay on the stack while its keep-a-creature
/// choices are pending, and reach the graveyard only after the chain
/// completes.
#[test]
fn divine_reckoning_stays_on_stack_until_choices_complete() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    // Both players control two creatures, so both get a real choice.
    let keep0 = ready_creature(&mut state, P0, 1, 1);
    let kill0 = ready_creature(&mut state, P0, 2, 2);
    let keep1 = ready_creature(&mut state, P1, 3, 3);
    let kill1 = ready_creature(&mut state, P1, 4, 4);

    let spell = castable_spell(&mut state, &reg, "Divine Reckoning", P0);
    state = cast_onto_stack(&state, &reg, spell, vec![]);

    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    // Mid-resolution: the spell must still be on the stack (CR 608.2m —
    // moving to the graveyard is the final step of resolution).
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Stack,
        "Divine Reckoning must remain on the stack while choices are pending");
    assert!(state.awaiting_action.is_some(), "first player's keep choice should be pending");

    // P0 keeps keep0; then P1 keeps keep1.
    state = answer_choice_with(&state, &reg, keep0);
    assert!(state.awaiting_action.is_some(), "second player's keep choice should be pending");
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Stack,
        "spell must still be on the stack between chained choices");
    state = answer_choice_with(&state, &reg, keep1);

    // Chain complete: engine moves the spell to the graveyard.
    assert!(state.awaiting_action.is_none());
    assert_eq!(state.get_object(spell).unwrap().zone, Zone::Graveyard,
        "engine must move the spell to the graveyard once the choice chain completes");

    // And the effect itself worked: each player kept exactly one creature.
    assert_eq!(state.get_object(keep0).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(keep1).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.get_object(kill0).unwrap().zone, Zone::Graveyard);
    assert_eq!(state.get_object(kill1).unwrap().zone, Zone::Graveyard);
}

/// A spell with no player choices is cleaned up immediately by resolve_spell,
/// and the resolving-spell tracker does not leak into later actions.
#[test]
fn immediate_resolution_cleanup_and_no_tracker_leak() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let target = ready_creature(&mut state, P1, 4, 4);
    let bolt = castable_spell(&mut state, &reg, "Lightning Bolt", P0);
    state = cast_onto_stack(&state, &reg, bolt, vec![Target::Object(target)]);
    mtg_engine::stack::resolve_top_of_stack(&mut state, &reg);

    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Graveyard);
    assert!(state.resolving_spell.is_none(), "tracker must be cleared after immediate cleanup");
}

// -------------------------------------------------------------------------
// Night Terrors: the same rule on a spell whose choice picks a card in a
// hand rather than a permanent on the battlefield.
// -------------------------------------------------------------------------

/// Night Terrors is "target player reveals their hand, you choose a nonland
/// card from it, exile that card". With more than one nonland card the
/// controller gets a choice, so the spell is mid-resolution and must stay on
/// the stack until that choice is answered (CR 608.2m).
///
/// The choice is answered by *name*, not by taking `options.first()`: a
/// Night Terrors that offered the wrong cards — or the land — would still
/// have a first option to take.
#[test]
fn night_terrors_stays_on_stack_until_its_choice_is_answered() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);

    let bears = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let bolt = spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);
    let growth = spell_in_hand(&mut state, &reg, "Giant Growth", P1);
    // A land in the same hand: "nonland card" is part of the effect, and
    // without one here the filter would never be exercised.
    let forest = spell_in_hand(&mut state, &reg, "Forest", P1);

    let nt = castable_spell(&mut state, &reg, "Night Terrors", P0);
    state = cast_and_resolve(&state, &reg, nt, vec![Target::Player(P1)]);

    assert_eq!(state.get_object(nt).unwrap().zone, Zone::Stack,
        "Night Terrors must stay on the stack while its choice is pending (CR 608.2m)");

    let mut offered = pending_choice_options(&state);
    offered.sort_by_key(|t| match t { Target::Object(o) => o.0, _ => u64::MAX });
    let mut expected = vec![Target::Object(bears), Target::Object(bolt), Target::Object(growth)];
    expected.sort_by_key(|t| match t { Target::Object(o) => o.0, _ => u64::MAX });
    assert_eq!(offered, expected,
        "every nonland card in the revealed hand is offered, and the land is not");

    // The controller of Night Terrors makes the choice, not the revealing player.
    let Some(AwaitingAction::ResolutionChoice { player, .. }) = &state.awaiting_action else {
        unreachable!("just asserted a ChooseTarget prompt is pending");
    };
    assert_eq!(*player, P0, "Night Terrors' controller chooses the card to exile");

    state = answer_choice_with(&state, &reg, bolt);

    assert!(state.awaiting_action.is_none());
    assert_eq!(state.get_object(nt).unwrap().zone, Zone::Graveyard,
        "the spell reaches the graveyard once its choice chain completes");
    assert_eq!(state.get_object(bolt).unwrap().zone, Zone::Exile, "the chosen card is exiled");
    for (id, name) in [(bears, "Grizzly Bears"), (growth, "Giant Growth"), (forest, "Forest")] {
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Hand,
            "{name} was not chosen and stays in hand");
    }
}

/// The other two arms of the same effect. One nonland card is not a choice —
/// the engine must not stop for a prompt with a single option — and no nonland
/// card at all is not a choice either; both finish the spell in one pass.
#[test]
fn night_terrors_without_a_choice_to_make_resolves_in_one_pass() {
    let reg = registry();

    // Exactly one nonland card: auto-selected, no prompt.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bears = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    let forest = spell_in_hand(&mut state, &reg, "Forest", P1);
    let nt = castable_spell(&mut state, &reg, "Night Terrors", P0);
    state = cast_and_resolve(&state, &reg, nt, vec![Target::Player(P1)]);

    assert!(state.awaiting_action.is_none(),
        "one nonland card is not a choice — the engine must not prompt for it");
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Exile);
    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Hand);
    assert_eq!(state.get_object(nt).unwrap().zone, Zone::Graveyard);

    // No nonland card: nothing is exiled and the spell still finishes.
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let forest = spell_in_hand(&mut state, &reg, "Forest", P1);
    let nt = castable_spell(&mut state, &reg, "Night Terrors", P0);
    state = cast_and_resolve(&state, &reg, nt, vec![Target::Player(P1)]);

    assert!(state.awaiting_action.is_none());
    assert_eq!(state.get_object(forest).unwrap().zone, Zone::Hand,
        "a land is not a legal pick, so the hand is untouched");
    assert_eq!(state.get_object(nt).unwrap().zone, Zone::Graveyard,
        "a Night Terrors that finds nothing to exile still leaves the stack");
}

/// Night Terrors exiles for good. It is not a Fiend Hunter-style
/// exile-and-return: the spell records nothing about what it exiled, and no
/// later event brings the card back.
#[test]
fn night_terrors_exiles_permanently_and_records_nothing() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    stock_library(&mut state, &reg, P0, 6);
    stock_library(&mut state, &reg, P1, 6);

    let bears = spell_in_hand(&mut state, &reg, "Grizzly Bears", P1);
    spell_in_hand(&mut state, &reg, "Lightning Bolt", P1);
    let nt = castable_spell(&mut state, &reg, "Night Terrors", P0);
    state = cast_and_resolve(&state, &reg, nt, vec![Target::Player(P1)]);
    state = answer_choice_with(&state, &reg, bears);

    assert!(state.get_object(nt).unwrap().card_state.is_empty(),
        "nothing is remembered about the exiled card — a returning effect is what \
         needs a record, and this spell has none");

    // A full turn's worth of triggers and state-based actions, with the spell
    // itself now in the graveyard: nothing returns the card.
    advance_to_next_turn(&mut state, &reg);
    assert_eq!(state.get_object(bears).unwrap().zone, Zone::Exile,
        "the card stays exiled");
}
