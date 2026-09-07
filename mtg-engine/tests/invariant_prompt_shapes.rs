//! Self-tests for the prompt-coherence invariants
//! (`mtg_engine::invariants`'s `prompts` family): a prompt the game raises
//! is answerable, addressed to the right player, and asks about things that
//! are there.
//!
//! Same contract as the other invariant self-tests — the checker is the
//! fuzzer's only pair of eyes, so every clause needs a prompt that violates
//! it, and every conditional clause a neighbouring prompt that does not.

mod common;
use common::*;
use mtg_engine::actions::Target;
use mtg_engine::cards::CardRegistry;
use mtg_engine::ids::ObjectId;
use mtg_engine::invariants::check_core;
use mtg_engine::state::{AwaitingAction, PendingEffect, ResolutionChoiceKind, StackEntry};
use mtg_engine::types::*;

fn base() -> (GameState, CardRegistry) {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    state.turn_number = 3;
    (state, reg)
}

/// The opening-hand phase as the game really reaches it: turn one, untap
/// step, everything still in libraries and hands.
fn opening_hands(reg: &CardRegistry) -> GameState {
    let mut state = game_at_step(Step::Untap, P0);
    state.turn_number = 1;
    state.is_first_turn = true;
    state.priority_player = None;
    for p in [P0, P1] {
        for id in stock_library(&mut state, reg, p, 20) {
            state.get_object_mut(id).unwrap().name = "Forest".into();
        }
        for _ in 0..7 {
            spell_in_hand(&mut state, reg, "Moment of Heroism", p);
        }
    }
    state
}

#[track_caller]
fn flags(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(v.iter().any(|m| m.contains(needle)),
        "expected a violation containing {needle:?}, got: {v:?}");
}

#[track_caller]
fn quiet_about(state: &GameState, reg: &CardRegistry, needle: &str) {
    let v = check_core(state, reg);
    assert!(!v.iter().any(|m| m.contains(needle)),
        "expected no violation containing {needle:?}, got: {v:?}");
}

/// CR 500.2/703.3: every turn-based-action prompt is raised on an empty
/// stack, with no triggers waiting, nothing half-resolved, and no mana
/// floating.
#[test]
fn a_turn_based_prompt_is_raised_on_a_quiet_game() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let card_id = state.get_object(bear).unwrap().card_id;
    state.step = Step::DeclareAttackers;
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);
    assert_eq!(check_core(&state, &reg), Vec::<String>::new());

    let mut s = state.clone();
    s.stack.push(StackEntry::Spell(bear));
    flags(&s, &reg, "with 1 entries on the stack (CR 500.2)");

    let mut s = state.clone();
    s.pending_triggers.push(mtg_engine::triggers::PendingTrigger::new(
        mtg_engine::triggers::TriggerSource::new(bear, card_id, P0, "a triggered ability"),
        mtg_engine::triggers::TriggerEvent::Upkeep));
    flags(&s, &reg, "attackers prompt with triggers still queued");

    let mut s = state.clone();
    s.resolving_spell = Some(bear);
    flags(&s, &reg, "with a cast or resolution in progress");

    let mut s = state.clone();
    add_mana(&mut s, P0, &[(ManaType::Green, 1)]);
    flags(&s, &reg, "has mana floating (CR 500.5)");

    // CR 508.1: and the attackers prompt in particular finds no combat
    // state at all, and nothing left over from an earlier one.
    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags(&s, &reg, "with leftovers from a previous combat");
    let mut s = state.clone();
    s.priority_player = Some(P1);
    flags(&s, &reg, "but priority is Some(PlayerId(1)), not the active player's");
}

/// CR 508.8/509.1: the blockers prompt is raised on a combat that has
/// attackers, in the blockers step, for the defending player.
#[test]
fn the_blockers_prompt_is_raised_on_a_declared_attack() {
    let (mut state, reg) = base();
    let attacker = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let blocker = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    state.step = Step::DeclareAttackers;
    submit_declare_attackers(&mut state, &[(attacker, P1)], &reg);
    state.events.clear();
    state.trigger_event_index = 0;
    state.step = Step::DeclareBlockers;
    state.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
    state.priority_player = Some(P1);
    assert_eq!(check_core(&state, &reg), Vec::<String>::new());

    let mut s = state.clone();
    s.step = Step::CombatDamage;
    flags(&s, &reg, "blockers prompt in CombatDamage (CR 509.1)");

    let mut s = state.clone();
    s.priority_player = Some(P0);
    flags(&s, &reg, "but priority is Some(PlayerId(0)), not the defender's");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().any_attackers_declared = false;
    flags(&s, &reg, "with no attackers declared (CR 508.8)");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().attackers.insert(attacker, P0);
    flags(&s, &reg, "but an attacker attacks someone other than p1");

    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocked_attackers.insert(attacker);
    flags(&s, &reg, "but blocks or damage are already recorded");
    let mut s = state.clone();
    s.combat.as_mut().unwrap().blocker_assignments.insert(attacker, vec![blocker]);
    flags(&s, &reg, "but blocks or damage are already recorded");
    let mut s = state.clone();
    s.combat.as_mut().unwrap().dealt_first_strike.insert(attacker);
    flags(&s, &reg, "but blocks or damage are already recorded");

    let mut s = state.clone();
    s.combat_damage_step_pending = true;
    flags(&s, &reg, "with a second damage step pending");
}

/// CR 514.1: the cleanup discard is asked of the active player, in their
/// cleanup, for exactly the cards over seven.
#[test]
fn the_cleanup_discard_prompt_asks_for_the_cards_over_seven() {
    let (mut state, reg) = base();
    for _ in 0..9 {
        spell_in_hand(&mut state, &reg, "Moment of Heroism", P0);
    }
    state.step = Step::Cleanup;
    state.priority_player = Some(P0);
    state.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 2 });
    assert_eq!(check_core(&state, &reg), Vec::<String>::new());

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P0, discard_count: 1 });
    flags(&s, &reg, "asks for 1 discards from a hand of 9 (CR 514.1)");

    let mut s = state.clone();
    s.step = Step::EndStep;
    flags(&s, &reg, "discard-to-hand-size prompt in EndStep (CR 514.1)");

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::DiscardToHandSize { player: P1, discard_count: 2 });
    flags(&s, &reg, "it is p0's cleanup");

    let mut s = state.clone();
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags(&s, &reg, "with combat state present");
}

/// CR 103: the opening-hand phase is turn one before anything has
/// happened, and the bottoming prompt asks for one card per mulligan.
#[test]
fn the_mulligan_phase_is_turn_one_before_anything_happened() {
    let reg = registry();
    let state = opening_hands(&reg);

    let mut s = state.clone();
    s.awaiting_action = Some(AwaitingAction::MulliganDecision { player: P0 });
    assert_eq!(check_core(&s, &reg), Vec::<String>::new());
    let keeping = s.clone();

    let mut s = keeping.clone();
    s.turn_number = 2;
    s.is_first_turn = false;
    flags(&s, &reg, "mulligan phase on turn 2");

    let mut s = keeping.clone();
    s.priority_player = Some(P0);
    flags(&s, &reg, "mulligan phase with a priority holder");

    let mut s = keeping.clone();
    s.combat = Some(mtg_engine::state::CombatState::new());
    flags(&s, &reg, "with a stack, combat, or a result");

    let mut s = keeping.clone();
    s.until_end_of_turn.push(mtg_engine::state::TemporaryEffect::GrantKeyword {
        target: ObjectId(1), keyword: Keyword::Flying });
    flags(&s, &reg, "mulligan phase with effects in force");

    // Nothing is on the battlefield yet, and nothing is a token.
    let mut s = keeping.clone();
    named_permanent(&mut s, &reg, "Grizzly Bears", P0);
    flags(&s, &reg, "is a card in Battlefield");

    let mut s = keeping.clone();
    s.get_player_mut(P0).land_plays_remaining = 0;
    flags(&s, &reg, "already has turn state");

    let mut s = keeping.clone();
    s.pending_mulligan_bottoms.push((P0, 8));
    flags(&s, &reg, "queued bottoming of 8 for p0");

    // A hand that is not seven cards is not a hand a mulligan decision is
    // made on (CR 103.4 redraws to seven every time).
    let mut s = keeping.clone();
    spell_in_hand(&mut s, &reg, "Moment of Heroism", P0);
    flags(&s, &reg, "mulligan prompt for p0 holding 8 cards (CR 103.5)");

    // A player who kept is not asked again.
    let mut s = keeping.clone();
    s.get_player_mut(P0).mulligan_kept = true;
    flags(&s, &reg, "who already kept");

    // CR 103.4: bottoming is one card per mulligan, after everyone kept.
    let mut s = state.clone();
    s.get_player_mut(P0).mulligan_count = 2;
    s.get_player_mut(P0).mulligan_kept = true;
    s.get_player_mut(P1).mulligan_kept = true;
    s.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 2 });
    assert_eq!(check_core(&s, &reg), Vec::<String>::new());

    let mut bad = s.clone();
    bad.awaiting_action = Some(AwaitingAction::BottomAfterMulligan { player: P0, count: 1 });
    flags(&bad, &reg, "bottom 1 after 2 mulligans");

    let mut bad = s.clone();
    bad.get_player_mut(P1).mulligan_kept = false;
    flags(&bad, &reg, "bottoming started before every player kept (CR 103.5)");
}

/// CR 608.2: a resolution prompt asks about things that are there, once
/// each, of the player who has to answer.
#[test]
fn a_resolution_prompt_offers_real_things_once_each() {
    let (mut state, reg) = base();
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    let other = named_permanent(&mut state, &reg, "Grizzly Bears", P1);
    let ghost = PlayerId(u8::try_from(state.players.len()).unwrap());
    let prompt = |choice: ResolutionChoiceKind| AwaitingAction::ResolutionChoice {
        player: P0, source: bear, choice };

    let target = |options: Vec<Target>| ResolutionChoiceKind::ChooseTarget {
        description: "d".into(), options, optional: false,
        effect: PendingEffect::DestroyCreature { source_name: "x".into() } };

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Object(other)])));
    quiet_about(&s, &reg, "offers");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Object(ObjectId(4242))])));
    flags(&s, &reg, "offers missing #4242");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Player(ghost)])));
    flags(&s, &reg, "offers p2 who is not a player");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Illegal])));
    flags(&s, &reg, "offers an Illegal target");

    let mut s = state.clone();
    s.awaiting_action = Some(prompt(target(vec![Target::Object(other), Target::Object(other)])));
    flags(&s, &reg, "twice");

    // CR 608.2: the prompt's source and the choice's source are the same
    // object.
    let mut s = state.clone();
    s.awaiting_action = Some(prompt(ResolutionChoiceKind::ChoosePile {
        description: "d".into(), pile_1: vec![bear], pile_2: vec![other], source_id: other }));
    flags(&s, &reg, "carries a choice for #");
}
