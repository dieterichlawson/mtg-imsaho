//! A combat prompt with one legal answer is answered, not presented.
//!
//! The engine asks the active player to declare attackers every combat and
//! the defender to declare blocks whenever anything is attacking, which is
//! right — CR 508.1 and 509.1 make both turn-based actions, and declaring
//! none is a declaration. With nothing eligible, though, the empty
//! declaration is the *only* answer, and three of the four seats had
//! quietly worked that out for themselves while the fourth had not: the
//! page drew "DECLARE ATTACKERS / CLICK CREATURES TO ATTACK WITH, THEN
//! CONFIRM" over a board with nothing to click, once per turn, and 8 of 11
//! declare-blockers prompts in three measured games were screens with
//! nothing on them (issue #517).
//!
//! The rule now lives once, in `mtg_player::forced_combat_answer`, and
//! every seat's combat entry point asks it. These tests hold all four to
//! it, the page included — a seat that forgets blocks on a decision the
//! person cannot answer.

use std::sync::mpsc;
use std::time::Duration;

use mtg_engine::actions::{Action, CombatPrompt};
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig, LegalActions};
use mtg_engine::ids::{ObjectId, PlayerId};
use mtg_engine::view::GameView;
use mtg_player::{forced_combat_answer, Player};

const P0: PlayerId = PlayerId(0);
const P1: PlayerId = PlayerId(1);

fn attackers(eligible: Vec<ObjectId>) -> CombatPrompt {
    CombatPrompt::ChooseAttackers {
        eligible,
        must_attack: vec![],
        defending_player: P1,
        defending_planeswalkers: vec![],
    }
}

fn blockers(eligible_blockers: Vec<ObjectId>, attacking: Vec<ObjectId>) -> CombatPrompt {
    let legal_blocks = eligible_blockers.iter()
        .map(|&b| (b, attacking.clone()))
        .collect();
    CombatPrompt::ChooseBlockers {
        eligible_blockers,
        attackers: attacking,
        legal_blocks,
        min_blockers: std::collections::HashMap::new(),
    }
}

/// A real view, for the seats that take one. The forced answers never read
/// it — that is the point — but the signatures want one.
fn a_view() -> (GameView, CardRegistry) {
    let registry = CardRegistry::with_all_cards();
    let deck = Decklist { entries: vec![("Plains".into(), 40)] };
    let config = GameConfig {
        player_names: vec!["a".into(), "b".into()],
        decklists: vec![deck.clone(), deck],
        starting_life: 20,
        starting_player: Some(P0),
        rng_seed: Some(1),
    };
    let state = engine::setup_game(&config, &registry);
    let view = GameView::for_player(&state, P0, &registry);
    (view, registry)
}

/// The environment the two constructing seats want, set once for the whole
/// binary so two test threads never race to set it.
fn seat_env() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // `GuiPlayer::new` wants the page's directory; tests run from the
        // crate root, so point it at the checkout rather than the relative
        // default.
        std::env::set_var(mtg_player::gui::WEB_DIR_ENV,
            concat!(env!("CARGO_MANIFEST_DIR"), "/../mtg-gui"));
        // `LlmPlayer::new` builds a backend eagerly. No request is made by
        // any test here — that is what they assert — but the constructor
        // still wants a key to exist.
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            std::env::set_var("ANTHROPIC_API_KEY", "dummy");
        }
    });
}

fn empty_legal() -> LegalActions {
    LegalActions {
        actions: vec![],
        combat_prompt: None,
        castable_spells: vec![],
        activatable_abilities: vec![],
        context: None,
        resolution_prompt: None,
        set_prompt: None,
    }
}

// ---------------------------------------------------------------------------
// The rule
// ---------------------------------------------------------------------------

/// Nothing eligible on either side of combat means one answer.
#[test]
fn an_empty_combat_prompt_has_exactly_one_answer() {
    assert!(matches!(forced_combat_answer(&attackers(vec![])),
        Some(Action::DeclareAttackers { ref attackers, ref planeswalker_attacks })
            if attackers.is_empty() && planeswalker_attacks.is_empty()));

    assert!(matches!(forced_combat_answer(&blockers(vec![], vec![ObjectId(61)])),
        Some(Action::DeclareBlockers { ref assignments }) if assignments.is_empty()));

    // No attacker to block is the same non-question from the other side.
    assert!(forced_combat_answer(&blockers(vec![ObjectId(30)], vec![])).is_some());
}

/// And a prompt with something to click is left alone. A rule that answered
/// every combat prompt would pass every other test in this file while
/// taking the game away from the player.
#[test]
fn a_prompt_with_something_eligible_is_the_players() {
    assert!(forced_combat_answer(&attackers(vec![ObjectId(7)])).is_none(),
        "one eligible attacker is a decision");
    assert!(forced_combat_answer(&blockers(vec![ObjectId(30)], vec![ObjectId(61)])).is_none(),
        "so is one eligible blocker against one attacker");
}

/// A creature that *must* attack is a decision even though the player has
/// no say over whether it attacks: it may still be sent at a planeswalker
/// rather than at the player (CR 508.1a).
#[test]
fn a_required_attack_is_not_a_forced_answer() {
    let prompt = CombatPrompt::ChooseAttackers {
        eligible: vec![ObjectId(7)],
        must_attack: vec![ObjectId(7)],
        defending_player: P1,
        defending_planeswalkers: vec![ObjectId(50)],
    };
    assert!(forced_combat_answer(&prompt).is_none());
}

// ---------------------------------------------------------------------------
// Every seat asks it
// ---------------------------------------------------------------------------

/// The CLI seat answers without drawing a screen. It has no terminal in a
/// test, so a seat that tried to render would not return at all.
#[test]
fn the_cli_seat_answers_a_forced_prompt_without_a_screen() {
    let (view, _reg) = a_view();
    let mut seat = mtg_player::cli::CliPlayer::new("cli");
    assert!(matches!(seat.choose_combat(&view, &attackers(vec![])),
        Action::DeclareAttackers { .. }));
    assert!(matches!(seat.choose_combat(&view, &blockers(vec![], vec![ObjectId(61)])),
        Action::DeclareBlockers { .. }));
}

/// The LLM seat answers without a round trip. There is no API key and no
/// backend here, so a seat that asked the model would fail rather than
/// return the empty declaration.
#[test]
fn the_llm_seat_answers_a_forced_prompt_without_asking_the_model() {
    seat_env();
    let (view, _reg) = a_view();
    let mut seat = mtg_player::llm::LlmPlayer::new("llm");
    assert!(matches!(seat.choose_combat(&view, &attackers(vec![])),
        Action::DeclareAttackers { .. }));
    assert!(matches!(seat.choose_combat(&view, &blockers(vec![], vec![ObjectId(61)])),
        Action::DeclareBlockers { .. }));
}

/// The random seat, which never had the rule written down, produces the
/// same answer by rolling over an empty list. Pinned here so the other
/// three are not the only ones holding the line.
#[test]
fn the_random_seat_answers_a_forced_prompt_the_same_way() {
    let mut seat = mtg_player::random::RandomPlayer::with_seed("r", 1);
    assert!(matches!(seat.choose_combat(&attackers(vec![])),
        Action::DeclareAttackers { ref attackers, .. } if attackers.is_empty()));
    assert!(matches!(seat.choose_combat(&blockers(vec![], vec![ObjectId(61)])),
        Action::DeclareBlockers { ref assignments } if assignments.is_empty()));
}

/// Real games, as the control the unit tests above cannot be: the rule has
/// to fire often (it is the normal state of the first turns and of every
/// board after a wipe) and it has to leave real combat alone. A rule that
/// answered every combat prompt would suppress the game itself, and the
/// counts here are what would notice.
#[test]
fn seeded_games_have_both_forced_and_real_combat_prompts() {
    let registry = CardRegistry::with_all_cards();
    let deck = |path: &str| -> Decklist {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
        Decklist {
            entries: text.lines().map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .filter_map(|l| {
                    let (n, name) = l.split_once(' ')?;
                    Some((name.trim().to_string(), n.parse::<u32>().ok()?))
                })
                .collect(),
        }
    };
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../decks/");
    let (mut forced, mut real) = (0u32, 0u32);

    for seed in 1..=6u64 {
        let config = GameConfig {
            player_names: vec!["a".into(), "b".into()],
            decklists: vec![deck(&format!("{root}ub-zombies.txt")), deck(&format!("{root}gw-humans.txt"))],
            starting_life: 20,
            starting_player: Some(P0),
            rng_seed: Some(seed),
        };
        let mut state = engine::setup_game(&config, &registry);
        let mut seats = [
            mtg_player::random::RandomPlayer::with_seed("a", seed + 1),
            mtg_player::random::RandomPlayer::with_seed("b", seed + 2),
        ];
        let mut decisions = 0u32;
        engine::run_game_loop(&mut state, &registry, |gs, acting, legal| {
            decisions += 1;
            if decisions > 6000 {
                return Action::AbandonGame;
            }
            match &legal.combat_prompt {
                Some(p) => {
                    if forced_combat_answer(p).is_some() { forced += 1; } else { real += 1; }
                    seats[acting.0 as usize].choose_combat(p)
                }
                None => {
                    let view = GameView::for_player(gs, acting, &registry);
                    seats[acting.0 as usize].choose_action(&view, legal)
                }
            }
        });
    }

    assert!(forced > 0,
        "six games produced no combat prompt with one answer — the rule this file is \
         about would be dead code");
    assert!(real > 0,
        "six games produced no combat prompt worth asking about: the rule is \
         swallowing real combat, which is far worse than the bug it fixes");
}

/// The seat this was filed against. `GuiPlayer::choose_combat` used to send
/// the prompt to the page and block on `answers.recv()` until a person
/// clicked something — with no browser attached, for ever. Answering it in
/// the seat is what makes this return at all, so the timeout is the
/// assertion.
#[test]
fn the_gui_seat_answers_a_forced_prompt_without_putting_it_to_the_page() {
    seat_env();
    let (view, _reg) = a_view();
    let legal = empty_legal();
    let Ok(mut seat) = mtg_player::gui::GuiPlayer::new("gui", None) else {
        // No free port in the seat's range: say so rather than passing
        // quietly, but do not fail a whole suite over a busy machine.
        eprintln!("skipped: no free port for the GUI seat");
        return;
    };

    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let a = seat.choose_combat(&view, &legal, &attackers(vec![]));
        let b = seat.choose_combat(&view, &legal, &blockers(vec![], vec![ObjectId(61)]));
        let _ = tx.send((format!("{a:?}"), format!("{b:?}")));
        seat
    });

    let (a, b) = rx.recv_timeout(Duration::from_secs(10)).expect(
        "the GUI seat answered a prompt with one answer itself; if this timed out \
         it put the prompt to a page nobody is looking at and waited for a click");
    assert!(a.starts_with("DeclareAttackers"), "got {a}");
    assert!(b.starts_with("DeclareBlockers"), "got {b}");
    drop(handle);
}
