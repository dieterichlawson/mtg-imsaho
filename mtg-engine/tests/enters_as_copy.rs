//! CR 614.12b / 706.9a: "you may have this enter as a copy of any creature on
//! the battlefield" is a choice made *as the permanent enters*, so everything
//! that follows from the copy — its copiable values, the replacement effects
//! printed on the copied card, the abilities that trigger on it entering —
//! happens as part of entering, with no window in between.

mod common;

use common::*;
use mtg_engine::actions::{Action, ResolvedChoice, Target};
use mtg_engine::state::{AwaitingAction, EnterAsCopyChoice, ResolutionChoiceKind};
use mtg_engine::types::*;

/// The consequence the old ETB-trigger implementation could not reach: the
/// copied card's own "enters tapped" replacement (CR 614.1c) applies, because
/// the permanent already *is* that card as it enters.
#[test]
fn a_copy_of_a_creature_that_enters_tapped_enters_tapped() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P1);

    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(grimgrin));

    assert_eq!(state.name_of(twin, &reg), "Grimgrin, Corpse-Born");
    assert!(state.get_object(twin).unwrap().tapped,
        "the copied card's 'enters tapped' applies to the copy (CR 614.1c)");
}

/// A declined copy is a printed 0/0 and enters untapped — the same card,
/// the other answer.
#[test]
fn a_declined_copy_enters_as_the_printed_card() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P1);

    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, None);

    assert_eq!(state.name_of(twin, &reg), "Evil Twin");
    assert!(!state.get_object(twin).unwrap().tapped);
    assert_eq!(state.effective_toughness(twin, &reg), Some(0),
        "the printed body is a 0/0 (CR 704.5f will have it)");
}

/// CR 706.2: the copiable values of a permanent that is itself a copy are the
/// values it copied. Copying a copy therefore reaches the original card, not
/// the printed card underneath it.
#[test]
fn copying_a_copy_takes_what_that_copy_copied() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let first = enters_as_copy_of(&mut state, &reg, "Evil Twin", P1, Some(bear));
    assert_eq!(state.name_of(first, &reg), "Grizzly Bears", "test precondition");

    let second = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(first));

    assert_eq!(state.name_of(second, &reg), "Grizzly Bears",
        "a copy of a copy is a copy of what the first one copied (CR 706.2)");
    assert_eq!(state.effective_power(second, &reg), Some(2));
}

/// The choice is not tied to casting: a permanent put onto the battlefield
/// from anywhere (Grimoire of the Dead reanimating an Evil Twin) is asked
/// too, and does not arrive until it is answered.
#[test]
fn an_evil_twin_reanimated_from_the_graveyard_is_asked_as_well() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card_id = reg.get_id_by_name("Evil Twin").unwrap();
    let twin = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    state.get_object_mut(twin).unwrap().name = "Evil Twin".into();

    state.move_object(twin, Zone::Battlefield, &reg);
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "reanimation waits on the choice too");
    mtg_engine::replacement::process_pending_entry_choices(&mut state, &reg);
    let asked = matches!(&state.awaiting_action, Some(AwaitingAction::ResolutionChoice {
        player,
        choice: ResolutionChoiceKind::ChooseTarget { options, .. },
        ..
    }) if *player == P0 && options.contains(&Target::Object(bear)));
    assert!(asked, "got {:?}", state.awaiting_action);

    let state = mtg_engine::engine::submit_action(
        &state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bear))) },
        &reg,
    );
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.name_of(twin, &reg), "Grizzly Bears");
}

/// Two of them entering at once each get their own answer — one prompt at a
/// time, and neither entry happens before its own choice is made.
#[test]
fn two_entering_copies_are_asked_one_at_a_time() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card_id = reg.get_id_by_name("Evil Twin").unwrap();
    let a = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    let b = state.create_object(card_id, P0, Zone::Graveyard, Some(0), Some(0));
    for id in [a, b] {
        state.get_object_mut(id).unwrap().name = "Evil Twin".into();
        state.move_object(id, Zone::Battlefield, &reg);
    }

    assert_eq!(state.pending_entry_choices, vec![a, b], "both are queued, neither entered");

    let mut state = state;
    for id in [a, b] {
        mtg_engine::replacement::process_pending_entry_choices(&mut state, &reg);
        assert!(state.awaiting_action.is_some(), "the queue asks for #{}", id.0);
        state = mtg_engine::engine::submit_action(
            &state,
            &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(bear))) },
            &reg,
        );
        assert_eq!(state.get_object(id).unwrap().zone, Zone::Battlefield);
    }
    assert!(state.pending_entry_choices.is_empty());
    assert_eq!(state.name_of(a, &reg), "Grizzly Bears");
    assert_eq!(state.name_of(b, &reg), "Grizzly Bears");
}

/// CR 400.7: a permanent that leaves the battlefield is a new object, and a
/// new object chooses again. The recorded answer must not follow the card
/// into the graveyard and back.
#[test]
fn the_recorded_choice_does_not_survive_a_zone_change() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(bear));

    state.move_object(twin, Zone::Graveyard, &reg);

    assert_eq!(state.get_object(twin).unwrap().entering_copy_choice, EnterAsCopyChoice::Unasked,
        "the answer is forgotten with the object it was made for");
    state.move_object(twin, Zone::Battlefield, &reg);
    assert_eq!(state.get_object(twin).unwrap().zone, Zone::Graveyard,
        "so coming back asks again rather than silently re-using the old answer");
}

/// CR 706.2 / 613.2: a copy has the copied card's abilities, and a static
/// ability functions wherever its text says it does. Grimgrin's is
/// "doesn't untap during your untap step", so a copy of Grimgrin sits out
/// its controller's untap step exactly as the printed card does.
///
/// The copy's static abilities used to be dropped on the floor: the object's
/// instance effect list was written as an empty vector and read as an
/// override, so a copy answered "no continuous effects" whatever the copied
/// card declares. A copied Grimgrin untapped and could attack every turn.
#[test]
fn a_copy_does_not_untap_when_the_copied_card_says_it_does_not() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P1);
    stock_library(&mut state, &reg, P0, 5);
    stock_library(&mut state, &reg, P1, 5);
    let grimgrin = named_permanent(&mut state, &reg, "Grimgrin, Corpse-Born", P1);

    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(grimgrin));
    assert!(state.get_object(twin).unwrap().tapped, "test precondition: it enters tapped");

    advance_to_next_turn(&mut state, &reg);

    assert_eq!(state.active_player, P0, "test precondition: it is the copy controller's turn");
    assert!(state.get_object(twin).unwrap().tapped,
        "a copy of Grimgrin doesn't untap during its controller's untap step (CR 706.2, 613.2)");
}

/// The same mechanism from the other side: a static ability of the copied
/// card that reaches *other* permanents. A copy of Mayor of Avabruck gives
/// the other Humans its controller controls +1/+1, because the copy has the
/// Mayor's abilities (CR 706.2).
#[test]
fn a_copy_grants_the_copied_cards_anthem_to_other_permanents() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let mayor = named_permanent(&mut state, &reg, "Mayor of Avabruck", P1);
    let human = named_permanent(&mut state, &reg, "Elite Inquisitor", P0);
    assert_eq!(state.effective_power(human, &reg), Some(2),
        "test precondition: the opponent's Mayor does not pump p0's Humans");

    enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(mayor));

    assert_eq!(state.effective_power(human, &reg), Some(3),
        "the copy has the Mayor's anthem and p0's other Human gets +1/+1 (CR 706.2)");
}

// ── One event, several cards: Grimoire of the Dead (#1064-#1067) ──────────

/// A card of `name` owned by `owner` in their graveyard.
fn in_graveyard(state: &mut mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry,
                name: &str, owner: mtg_engine::ids::PlayerId) -> mtg_engine::ids::ObjectId {
    let card = reg.get_id_by_name(name).unwrap();
    let data = reg.card_data(card).unwrap();
    let id = state.create_object(card, owner, Zone::Graveyard, data.power, data.toughness);
    state.get_object_mut(id).unwrap().name = name.into();
    id
}

/// p0's Grimoire of the Dead with its three study counters, activated and
/// resolved, with the first enters-as-copy question raised.
fn resolve_grimoire(mut state: mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry)
    -> mtg_engine::state::GameState {
    let grimoire = named_permanent(&mut state, reg, "Grimoire of the Dead", P0);
    state.add_counters(grimoire, CounterType::Study, 3);
    let mut state = activate(&state, reg, grimoire, 1, vec![]);
    mtg_engine::replacement::process_pending_entry_choices(&mut state, reg);
    state
}

fn answer(state: &mtg_engine::state::GameState, reg: &mtg_engine::cards::CardRegistry,
          copy: mtg_engine::ids::ObjectId) -> mtg_engine::state::GameState {
    let mut state = mtg_engine::engine::submit_action(
        state,
        &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(Some(Target::Object(copy))) },
        reg,
    );
    // What the game loop does next: the queue's next question, and only
    // once it is empty the state-based actions (`run_game_loop_inner`).
    mtg_engine::replacement::process_pending_entry_choices(&mut state, reg);
    if state.pending_entry_choices.is_empty() {
        while mtg_engine::sba::check_state_based_actions(&mut state, reg) {}
    }
    state
}

fn asked(state: &mtg_engine::state::GameState) -> Option<mtg_engine::ids::PlayerId> {
    match &state.awaiting_action {
        Some(AwaitingAction::ResolutionChoice { player, .. }) => Some(*player),
        _ => None,
    }
}

/// CR 614.12b, 707: the Twin copies what is on the battlefield as it enters,
/// and the cards entering with it are not there yet. The engine moved every
/// other card first and then asked, so the Twin was offered the Festerhide
/// Boar returning beside it (#1065).
#[test]
fn a_twin_returned_with_other_creatures_cannot_copy_them() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = in_graveyard(&mut state, &reg, "Evil Twin", P0);
    let boar = in_graveyard(&mut state, &reg, "Festerhide Boar", P0);

    let state = resolve_grimoire(state, &reg);

    let offered = pending_choice_options(&state);
    assert_eq!(offered, vec![Target::Object(bear)],
        "only what was already on the battlefield is a copy to choose");
    assert_eq!(state.get_object(boar).unwrap().zone, Zone::Graveyard,
        "the batch enters together, after the choice");
    let state = answer(&state, &reg, bear);
    assert_eq!(state.get_object(boar).unwrap().zone, Zone::Battlefield);
    assert_eq!(state.name_of(twin, &reg), "Grizzly Bears");
}

/// CR 109.5, 614.12b: the choice belongs to the player the permanent enters
/// under. p0's Grimoire returning p1's Twin asked p1 (#1066).
#[test]
fn an_opponents_twin_returned_under_your_control_asks_you() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = in_graveyard(&mut state, &reg, "Evil Twin", P1);

    let state = resolve_grimoire(state, &reg);

    assert_eq!(asked(&state), Some(P0), "got {:?}", state.awaiting_action);
    let state = answer(&state, &reg, bear);
    assert_eq!(state.get_object(twin).unwrap().controller, P0);
}

/// CR 613.1a, 613.1d-e: Grimoire's "black Zombies" applies on top of the
/// copy. The deferred Twin skipped the stamp, and the count line counted the
/// two Twins still choosing as returned (#1067).
#[test]
fn a_twin_returned_by_grimoire_is_a_black_zombie_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = in_graveyard(&mut state, &reg, "Evil Twin", P0);
    let boar = in_graveyard(&mut state, &reg, "Festerhide Boar", P1);

    let mut state = resolve_grimoire(state, &reg);
    let before: Vec<String> = state.game_log.iter().map(|e| e.message.clone()).collect();
    assert!(!before.iter().any(|l| l.contains("creatures returned")),
        "nothing has returned while the Twin chooses: {before:?}");
    state.game_log.clear();
    let state = answer(&state, &reg, bear);

    for id in [twin, boar] {
        assert!(state.has_subtype(id, "Zombie", &reg), "#{} is a Zombie", id.0);
        assert!(state.colors_of(id, &reg).contains(&Color::Black), "#{} is black", id.0);
    }
    assert!(state.has_subtype(twin, "Bear", &reg), "and still the Bear it copied");
    let lines: Vec<String> = state.game_log.iter().map(|e| e.message.clone()).collect();
    assert!(lines.iter().any(|l| l == "Grimoire of the Dead: 2 creatures returned from all graveyards"),
        "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("Grimoire of the Dead: Evil Twin (#") && l.ends_with(") returned as a black Zombie")),
        "{lines:?}");
}

/// CR 704.3: no state-based action runs inside a resolution. With two Twins
/// in the batch, SBAs ran between their answers: a Boneyard Wurm that the
/// first Twin's leaving had made 0/0 died, and the second Twin's copy of a
/// Festerhide Boar entered with morbid's counters (#1064).
#[test]
fn two_twins_in_one_batch_see_no_state_based_action_between_answers() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let wurm = named_permanent(&mut state, &reg, "Boneyard Wurm", P0);
    let traveler = named_permanent(&mut state, &reg, "Doomed Traveler", P1);
    let boar = named_permanent(&mut state, &reg, "Festerhide Boar", P1);
    let first = in_graveyard(&mut state, &reg, "Evil Twin", P0);
    let second = in_graveyard(&mut state, &reg, "Evil Twin", P1);

    let state = resolve_grimoire(state, &reg);
    let state = answer(&state, &reg, traveler);

    assert_eq!(state.get_object(wurm).unwrap().zone, Zone::Battlefield,
        "the resolution is not over, so nothing has died");
    assert_eq!(state.get_object(first).unwrap().zone, Zone::Graveyard,
        "and nothing has entered: the batch waits on the second answer");
    assert!(pending_choice_options(&state).contains(&Target::Object(wurm)));

    let state = answer(&state, &reg, boar);
    assert_eq!(state.name_of(second, &reg), "Festerhide Boar");
    assert!(state.get_object(second).unwrap().counters.is_empty(),
        "no creature had died as it entered, so morbid adds nothing (CR 603.2)");
    assert_eq!(state.get_object(wurm).unwrap().zone, Zone::Graveyard,
        "once the batch is in, the 0/0 Wurm dies to SBA");
}

// ── A token copy of the card itself (#1068) ───────────────────────────────

/// CR 707.2, 111.1, 614.12b: a token copy of an Evil Twin *card* (Back from
/// the Brink exiling one) has the Twin's replacement among its copiable
/// values, and it applies as the token is created. It was skipped: the
/// token entered as a printed 0/0 and died (#1068).
#[test]
fn a_token_copy_of_an_evil_twin_card_is_asked_what_to_copy() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card = in_graveyard(&mut state, &reg, "Evil Twin", P0);
    state.move_object(card, Zone::Exile, &reg);
    let before = state.objects.len();

    let token = state.create_token_copy(card, P0, &reg);

    assert_eq!(token, mtg_engine::ids::ObjectId(0), "no token exists before the answer");
    assert_eq!(state.objects.len(), before);
    mtg_engine::replacement::process_pending_entry_choices(&mut state, &reg);
    assert_eq!(asked(&state), Some(P0), "the creator is asked: {:?}", state.awaiting_action);
    assert_eq!(pending_choice_options(&state), vec![Target::Object(bear)]);

    let state = answer(&state, &reg, bear);
    let tokens: Vec<_> = state.all_objects_in_zone(Zone::Battlefield).into_iter()
        .filter(|o| o.is_token).map(|o| o.id).collect();
    assert_eq!(tokens.len(), 1, "the answer creates the token");
    assert_eq!(state.name_of(tokens[0], &reg), "Grizzly Bears");
    assert!(mtg_engine::cards::ability_granting_grantor(&state, tokens[0], &reg).is_some(),
        "with the Twin's \"except it has\" ability (CR 706.9a)");
    assert!(state.pending_token_copies.is_empty());
}

/// Declined, it is the printed 0/0 the rules give, and SBAs have it.
#[test]
fn a_declined_token_copy_of_an_evil_twin_card_is_a_0_0() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let card = in_graveyard(&mut state, &reg, "Evil Twin", P0);
    state.create_token_copy(card, P0, &reg);
    mtg_engine::replacement::process_pending_entry_choices(&mut state, &reg);

    let mut state = mtg_engine::engine::submit_action(
        &state, &Action::ResolveChoice { choice: ResolvedChoice::ChosenTarget(None) }, &reg);

    let token = state.all_objects_in_zone(Zone::Battlefield).into_iter()
        .find(|o| o.is_token).map(|o| o.id).expect("declining still creates the token");
    assert_eq!(state.name_of(token, &reg), "Evil Twin");
    while mtg_engine::sba::check_state_based_actions(&mut state, &reg) {}
    assert!(state.get_object(token).is_none_or(|o| o.zone != Zone::Battlefield));
}

/// A copy of a Twin that already copied something copies what it copied
/// (CR 707.2), so nothing is asked.
#[test]
fn a_token_copy_of_a_twin_clone_is_not_asked() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let twin = enters_as_copy_of(&mut state, &reg, "Evil Twin", P0, Some(bear));

    let token = state.create_token_copy(twin, P0, &reg);

    assert_eq!(state.name_of(token, &reg), "Grizzly Bears");
    assert!(state.pending_token_copies.is_empty());
}
