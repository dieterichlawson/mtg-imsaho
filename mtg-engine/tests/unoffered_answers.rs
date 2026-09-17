//! An answer the prompt never offered is not played.
//!
//! The loop computed `legal` to build the prompt and then never consulted it
//! again, so whatever a seat sent was executed. A `PassPriority` sent at a
//! declare-attackers prompt left the declaration outstanding — right — and
//! moved priority to the non-active player anyway, which CR 508.1 says never
//! happens: declaring attackers is a turn-based action of the active player
//! and no player has priority until it is done. The same pass at a
//! `ResolutionChoice` was executed while the choice stood, so the step
//! machinery advanced past it and carried queued triggers into the
//! declare-attackers step. Both are states `invariants/prompts.rs` exists to
//! say the rules never produce, and `--check-invariants` exits 2 on them
//! (issue #514).
//!
//! No shipped seat reaches this by itself — the page's widgets, the CLI's
//! combat screen and the LLM seat's substitution all answer the prompt they
//! were given — so the fuzzer cannot find it either: the random seat only
//! picks from `legal`. It is reachable from anything speaking the socket
//! protocol, which is why the guard belongs in the engine rather than in a
//! seat.

mod common;
use common::*;

use mtg_engine::actions::{Action, ResolvedChoice};
use mtg_engine::engine::LegalActions;
use mtg_engine::ids::ObjectId;
use mtg_engine::invariants::check_core;
use mtg_engine::state::{AwaitingAction, GameState, ResolutionChoiceKind};
use mtg_engine::types::*;

/// What the engine looked like the second time it asked the same prompt.
struct ReAsk {
    priority: Option<PlayerId>,
    awaiting: Option<AwaitingAction>,
    step: Step,
    violations: Vec<String>,
}

// ---------------------------------------------------------------------------
// The shape test, on its own
// ---------------------------------------------------------------------------

/// A priority menu admits its own rows and nothing else. `PassPriority` is a
/// row of one, which is exactly why sending it at a prompt that does not
/// list it slipped through.
#[test]
fn a_priority_menu_admits_its_rows_and_refuses_the_rest() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);

    assert!(legal.permits(&Action::PassPriority), "a pass is on a priority menu");
    assert!(!legal.permits(&Action::DeclareAttackers {
        attackers: vec![], planeswalker_attacks: vec![] }),
        "a declaration is not — there is no combat prompt here");
    assert!(!legal.permits(&Action::ResolveChoice {
        choice: ResolvedChoice::YesNoDecision(true) }),
        "and neither is an answer to a choice nobody asked for");
    assert!(!legal.permits(&Action::PlayLand { object_id: ObjectId(9999) }),
        "a land that is not in hand is not a row of this menu");
}

/// Each prompt admits the answer it asked for. The structured ones are
/// admitted by kind — the seat builds those from the prompt rather than
/// picking a row, and `submit_action` checks the contents.
#[test]
fn every_prompt_admits_its_own_answer_and_refuses_a_pass() {
    let reg = registry();

    let attackers = {
        let mut s = game_at_step(Step::DeclareAttackers, P0);
        ready_creature(&mut s, P0, 2, 2);
        s.awaiting_action = Some(AwaitingAction::DeclareAttackers);
        s.priority_player = Some(P0);
        mtg_engine::engine::legal_actions(&s, &reg)
    };
    assert!(attackers.permits(&Action::DeclareAttackers {
        attackers: vec![], planeswalker_attacks: vec![] }));
    assert!(!attackers.permits(&Action::PassPriority),
        "CR 508.1: no player has priority until the declaration is made");

    let blockers = {
        let mut s = game_at_step(Step::DeclareBlockers, P0);
        ready_creature(&mut s, P1, 2, 2);
        s.awaiting_action = Some(AwaitingAction::DeclareBlockers { defending_player: P1 });
        s.priority_player = Some(P1);
        mtg_engine::engine::legal_actions(&s, &reg)
    };
    assert!(blockers.permits(&Action::DeclareBlockers { assignments: vec![] }));
    assert!(!blockers.permits(&Action::PassPriority));
    assert!(!blockers.permits(&Action::DeclareAttackers {
        attackers: vec![], planeswalker_attacks: vec![] }),
        "the other half of the combat prompt is still the wrong answer");

    let discard = {
        let mut s = game_at_step(Step::Cleanup, P0);
        spell_in_hand(&mut s, &reg, "Grizzly Bears", P0);
        s.awaiting_action = Some(AwaitingAction::DiscardToHandSize {
            player: P0, discard_count: 1 });
        mtg_engine::engine::legal_actions(&s, &reg)
    };
    assert!(discard.permits(&Action::DiscardCards { cards: vec![] }));
    assert!(!discard.permits(&Action::BottomCards { cards: vec![] }),
        "a bottoming is not a discard, though both are a set of cards from hand");
    assert!(!discard.permits(&Action::PassPriority));

    let choice = {
        let mut s = game_at_step(Step::PrecombatMain, P0);
        let bear = named_permanent(&mut s, &reg, "Grizzly Bears", P0);
        s.awaiting_action = Some(AwaitingAction::ResolutionChoice {
            player: P0,
            source: bear,
            choice: ResolutionChoiceKind::YesNo {
                description: "?".into(), source_card: bear },
        });
        mtg_engine::engine::legal_actions(&s, &reg)
    };
    assert!(choice.permits(&Action::ResolveChoice {
        choice: ResolvedChoice::YesNoDecision(false) }));
    assert!(!choice.permits(&Action::PassPriority));
}

/// CR 104.3a: a player may concede at any time, so no prompt refuses it —
/// including the prompts that list no actions at all. The gate has to make
/// that exception explicitly, because no menu carries `Concede` beside a
/// prompt for it to be found in.
#[test]
fn a_concede_is_admitted_at_every_prompt() {
    let reg = registry();
    let mut state = game_at_step(Step::DeclareAttackers, P0);
    ready_creature(&mut state, P0, 2, 2);
    state.awaiting_action = Some(AwaitingAction::DeclareAttackers);
    state.priority_player = Some(P0);
    let legal = mtg_engine::engine::legal_actions(&state, &reg);
    assert!(legal.actions.is_empty(), "test precondition: this prompt lists no rows");
    assert!(legal.permits(&Action::Concede));
    assert!(legal.permits(&Action::AbandonGame), "nor is the harness stopping a game action");
}

// ---------------------------------------------------------------------------
// The loop, which is where it mattered
// ---------------------------------------------------------------------------

/// Run `state` to the first prompt `at` matches, answer it once with
/// `unoffered`, record what the engine looks like when it asks again, and
/// then answer it properly with `answer` so the game can finish.
fn inject_once(
    state: &mut GameState,
    reg: &mtg_engine::cards::CardRegistry,
    at: impl Fn(&GameState, &LegalActions) -> bool,
    unoffered: Action,
    answer: Action,
) -> Option<ReAsk> {
    let mut injected = false;
    let mut seen: Option<ReAsk> = None;
    mtg_engine::engine::run_game_loop(state, reg, |gs, _acting, legal| {
        if at(gs, legal) {
            if !injected {
                injected = true;
                return unoffered.clone();
            }
            if seen.is_none() {
                seen = Some(ReAsk {
                    priority: gs.priority_player,
                    awaiting: gs.awaiting_action.clone(),
                    step: gs.step,
                    violations: check_core(gs, reg),
                });
            }
            return answer.clone();
        }
        Action::Concede
    });
    assert!(injected, "the prompt under test was never reached");
    seen
}

/// The prompt clauses of `invariants/prompts.rs` — the ones the issue's
/// `--check-invariants` run stopped on. A state a test builds by hand
/// carries unrelated bookkeeping noise (a vanilla creature with no registry
/// entry, `is_first_turn` on a state that did not start a game), and
/// asserting on the whole list would measure that instead.
fn about_the_prompt(violations: &[String]) -> Vec<&String> {
    violations.iter().filter(|v| v.contains("prompt")).collect()
}

/// The reported case. A pass at the attackers prompt is refused, so priority
/// does not move off the active player and the declaration is asked for
/// again in a state the invariants accept.
#[test]
fn a_pass_at_the_attackers_prompt_leaves_priority_with_the_active_player() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P0, 2, 2);
    state.priority_player = Some(P0);

    let again = inject_once(&mut state, &reg,
        |_, legal| matches!(legal.combat_prompt,
            Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { .. })),
        Action::PassPriority,
        Action::DeclareAttackers { attackers: vec![], planeswalker_attacks: vec![] })
        .expect("the attackers prompt is asked again after the refused pass");

    assert_eq!(again.priority, Some(P0),
        "CR 508.1: priority is the active player's, not the opponent's");
    assert!(matches!(again.awaiting, Some(AwaitingAction::DeclareAttackers)),
        "the declaration is still outstanding, got {:?}", again.awaiting);
    assert!(about_the_prompt(&again.violations).is_empty(),
        "the state the re-ask happens in: {:?}", about_the_prompt(&again.violations));
}

/// The wider case from the same hole: a pass at a mid-resolution choice used
/// to be executed while the choice stood, which let the step machinery walk
/// past an outstanding prompt with its triggers still queued.
#[test]
fn a_pass_at_a_resolution_choice_leaves_the_choice_outstanding() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = named_permanent(&mut state, &reg, "Grizzly Bears", P0);
    state.priority_player = Some(P0);
    state.awaiting_action = Some(AwaitingAction::ResolutionChoice {
        player: P0,
        source: bear,
        choice: ResolutionChoiceKind::YesNo {
            description: "keep the choice outstanding?".into(), source_card: bear },
    });

    let again = inject_once(&mut state, &reg,
        |_, legal| legal.resolution_prompt.is_some(),
        Action::PassPriority,
        Action::ResolveChoice { choice: ResolvedChoice::YesNoDecision(false) })
        .expect("the choice is asked again after the refused pass");

    assert!(matches!(again.awaiting, Some(AwaitingAction::ResolutionChoice { .. })),
        "the choice is still the thing being waited on, got {:?}", again.awaiting);
    assert_eq!(again.priority, Some(P0),
        "the pass was not executed, so priority did not move — which is what \
         let the step machinery walk past the outstanding choice");
    assert_eq!(again.step, Step::PrecombatMain,
        "and the step did not advance past an unanswered prompt");
    assert!(about_the_prompt(&again.violations).is_empty(),
        "{:?}", about_the_prompt(&again.violations));
}

/// The other half, which is the one a fix can quietly break: an answer the
/// prompt *does* offer is still played. A gate that refused everything would
/// pass every test above.
#[test]
fn the_answer_the_prompt_asked_for_is_still_played() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    let bear = ready_creature(&mut state, P0, 2, 2);
    state.priority_player = Some(P0);

    let mut declared = false;
    mtg_engine::engine::run_game_loop(&mut state, &reg, |gs, _acting, legal| {
        // Far enough: the attack has resolved and the turn has turned over.
        if gs.turn_number >= 2 {
            return Action::Concede;
        }
        if matches!(legal.combat_prompt,
            Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { .. })) && !declared
        {
            declared = true;
            return Action::DeclareAttackers {
                attackers: vec![(bear, P1)], planeswalker_attacks: vec![] };
        }
        if legal.actions.iter().any(|a| matches!(a, Action::PassPriority)) {
            return Action::PassPriority;
        }
        if legal.combat_prompt.is_some() {
            return Action::DeclareBlockers { assignments: vec![] };
        }
        Action::Concede
    });

    assert!(declared, "the attackers prompt was reached");
    assert_eq!(state.get_player(P1).life, 18,
        "the 2/2 attacked and connected — the gate did not eat a legal declaration");
}

/// A seat that will only ever answer with something unoffered does not spin
/// the engine silently. The decision is handed back — so the caller's
/// progress watchdog, which counts decisions, sees each one — and the
/// refusal is written to the game log, which is the only channel a seat
/// that sent an unusable answer has to find that out.
#[test]
fn a_refused_answer_is_asked_again_and_said_out_loud() {
    let reg = registry();
    let mut state = game_at_step(Step::PrecombatMain, P0);
    ready_creature(&mut state, P0, 2, 2);
    state.priority_player = Some(P0);

    let passes = |gs: &GameState| gs.game_log.iter()
        .filter(|e| e.message.contains("passes priority")).count();

    let mut asks = 0;
    // The passes already played to reach combat, which are legitimate; what
    // matters is that the four refused ones add nothing to them.
    let mut passes_before = 0;
    let mut passes_after = 0;
    mtg_engine::engine::run_game_loop(&mut state, &reg, |gs, _, legal| {
        if matches!(legal.combat_prompt,
            Some(mtg_engine::actions::CombatPrompt::ChooseAttackers { .. }))
        {
            asks += 1;
            if asks == 1 {
                passes_before = passes(gs);
            }
            if asks < 5 {
                return Action::PassPriority;
            }
            passes_after = passes(gs);
        }
        Action::Concede
    });

    assert_eq!(asks, 5,
        "each refused answer comes back to the seat as the same decision");
    assert_eq!(passes_after, passes_before,
        "and none of the four was played — the log gained no pass");

    let refusals = state.game_log.iter()
        .filter(|e| e.message.contains("was not one this prompt offers"))
        .count();
    assert_eq!(refusals, 4, "every refusal is recorded where the seat can read it");
}
