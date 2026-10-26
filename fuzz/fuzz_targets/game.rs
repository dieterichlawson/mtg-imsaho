//! The whole engine, as one property: from any state a game can reach, no
//! legal action makes the engine panic or break an invariant.
//!
//! This is the fuzz campaign's oracle (`scripts/fuzz.sh`, every
//! `mtg_engine::invariants` check at every decision) turned into a
//! coverage-guided target, written to the rules of Theorem's verification
//! skill, after Anthropic's 2026-10-22 post on proving software with LLMs
//! (`docs/plans/formal-verification.md`, tier 2):
//!
//! - **The input drives the API, and decodes totally.** Two bytes pick the
//!   decks, eight seed the shuffles, and every choice either seat makes is
//!   drawn from the rest by the random seat (`RandomPlayer`), which answers
//!   every kind of question the engine asks. Any byte string is a game.
//!   When the input runs out the game is abandoned at the next decision,
//!   the harness's own stop (`AbandonGame`), so a longer input is a longer
//!   game and not a different one.
//! - **Properties abort.** A violated invariant or a stalled game is a
//!   `panic!`, which libFuzzer records with the input that caused it. A
//!   panic inside the engine is a finding too, as is an integer overflow:
//!   this builds with overflow checks.
//! - **Bounded by the input, and by a decision ceiling** for an input whose
//!   choices keep a game going.
//! - **The input is the only randomness**: the engine's seed and both
//!   seats' generators come from it; nothing reads a clock or the OS.
//!
//! Run it with `scripts/fuzz-engine.sh`.

#![no_main]

use std::cell::RefCell;
use std::rc::Rc;

use libfuzzer_sys::fuzz_target;
use mtg_engine::actions::Action;
use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig, LegalActions};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;
use mtg_player::random::RandomPlayer;
use mtg_player::Player;

/// The coverage decks: between them, every castable card the engine
/// implements (`mtg-engine/tests/deck_coverage.rs`).
const DECKS: [&str; 10] = [
    include_str!("../../decks/coverage/bg-coverage.txt"),
    include_str!("../../decks/coverage/br-coverage.txt"),
    include_str!("../../decks/coverage/rg-coverage.txt"),
    include_str!("../../decks/coverage/ub-coverage.txt"),
    include_str!("../../decks/coverage/ug-coverage.txt"),
    include_str!("../../decks/coverage/ur-coverage.txt"),
    include_str!("../../decks/coverage/wb-coverage.txt"),
    include_str!("../../decks/coverage/wg-coverage.txt"),
    include_str!("../../decks/coverage/wr-coverage.txt"),
    include_str!("../../decks/coverage/wu-coverage.txt"),
];

/// A game the input has not ended by then is ended here. Coverage games run
/// to a few hundred decisions.
const MAX_DECISIONS: u64 = 3_000;

fn decklist(text: &str) -> Decklist {
    let entries = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (count, name) = l.split_once(' ').expect("a deck line is COUNT NAME");
            (name.trim().to_string(), count.parse().expect("a deck line starts with a count"))
        })
        .collect();
    Decklist { entries }
}

/// The fuzzer's input as a stream of random bytes, shared by both seats.
/// Past its end it yields zeros (any byte string decodes to a game) and
/// says it has run out, which ends the game at the next decision.
struct Input {
    bytes: Vec<u8>,
    at: usize,
}

impl Input {
    fn byte(&mut self) -> u8 {
        let b = self.bytes.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        b
    }

    fn exhausted(&self) -> bool {
        self.at >= self.bytes.len()
    }
}

/// One seat's handle on the shared input.
struct InputRng(Rc<RefCell<Input>>);

impl rand_core::RngCore for InputRng {
    fn next_u32(&mut self) -> u32 {
        let mut input = self.0.borrow_mut();
        u32::from_le_bytes([input.byte(), input.byte(), input.byte(), input.byte()])
    }

    fn next_u64(&mut self) -> u64 {
        u64::from(self.next_u32()) | (u64::from(self.next_u32()) << 32)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        let mut input = self.0.borrow_mut();
        for b in dest {
            *b = input.byte();
        }
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

fn registry() -> &'static CardRegistry {
    static REGISTRY: std::sync::OnceLock<CardRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(CardRegistry::with_all_cards)
}

/// Every invariant the fuzz campaign checks at a decision, as the runner
/// checks them under `--check-invariants`.
fn violations(
    state: &GameState,
    acting: PlayerId,
    legal: &LegalActions,
    last: Option<&(GameState, Action)>,
    nontoken_baseline: &mut Option<Vec<usize>>,
    decision: u64,
) -> Vec<String> {
    let registry = registry();
    let mut v = if legal.resolution_prompt.is_some() {
        mtg_engine::invariants::check_core(state, registry)
    } else {
        mtg_engine::invariants::check_settled(state, registry)
    };
    if let Some((prev, act)) = last {
        v.extend(mtg_engine::invariants::check_transition(prev, Some(act), state, registry));
    }
    v.extend(mtg_engine::invariants::check_legal(state, acting, legal, registry));

    // No card enters or leaves the game (CR 400.1, 108.3).
    let mut counts = vec![0usize; state.players.len()];
    for obj in state.objects.values() {
        if !obj.is_token {
            counts[obj.owner.0 as usize] += 1;
        }
    }
    match nontoken_baseline {
        None => *nontoken_baseline = Some(counts),
        Some(base) if *base != counts => v.push(format!("non-token card counts changed: {base:?} -> {counts:?}")),
        Some(_) => {}
    }

    if legal.offers_nothing() {
        v.push("a decision with no legal actions and no prompt".to_string());
    }

    // A save is the state: it survives a round trip with nothing lost.
    if decision % 16 == 0 {
        let v1 = serde_json::to_value(state).expect("a state serializes");
        let reloaded: GameState = serde_json::from_value(v1.clone()).expect("a state deserializes from its own save");
        if serde_json::to_value(&reloaded).expect("a reloaded state serializes") != v1 {
            v.push("the state does not survive a serialization round trip".to_string());
        }
    }
    v
}

fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
        return;
    }
    let registry = registry();
    let deck1 = DECKS[usize::from(data[0]) % DECKS.len()];
    let deck2 = DECKS[usize::from(data[1]) % DECKS.len()];
    let seed = u64::from_le_bytes(data[2..10].try_into().expect("eight bytes"));
    let input = Rc::new(RefCell::new(Input { bytes: data[10..].to_vec(), at: 0 }));

    let config = GameConfig {
        player_names: vec!["p0".to_string(), "p1".to_string()],
        decklists: vec![decklist(deck1), decklist(deck2)],
        starting_life: 20,
        starting_player: None,
        rng_seed: Some(seed),
    };
    let mut state = engine::setup_game(&config, registry);
    state.observe_every_submit = true;

    let mut seats = [
        RandomPlayer::with_rng("p0", InputRng(Rc::clone(&input))),
        RandomPlayer::with_rng("p1", InputRng(Rc::clone(&input))),
    ];
    let mut decisions: u64 = 0;
    let mut last: Option<(GameState, Action)> = None;
    let mut nontoken_baseline = None;
    let mut watchdog = mtg_player::watchdog::ProgressWatchdog::new();

    engine::run_game_loop(&mut state, registry, &mut |state: &GameState, acting: PlayerId, legal: &LegalActions| {
        decisions += 1;
        if watchdog.observe(state) {
            panic!("{}", mtg_player::watchdog::stall_report(state, acting, legal, &format!("p{}", acting.0)));
        }
        let found = violations(state, acting, legal, last.as_ref(), &mut nontoken_baseline, decisions);
        assert!(
            found.is_empty(),
            "invariant violation at decision {decisions} (turn {}, {:?}):\n  {}",
            state.turn_number,
            state.step,
            found.join("\n  ")
        );
        if input.borrow().exhausted() || decisions >= MAX_DECISIONS {
            return mtg_player::watchdog::ceiling_move();
        }
        let seat = &mut seats[acting.0 as usize];
        let chosen = if let Some(prompt) = &legal.combat_prompt {
            seat.choose_combat(prompt)
        } else {
            let view = GameView::for_player(state, acting, registry);
            seat.choose_action(&view, legal)
        };
        last = Some((state.clone(), chosen.clone()));
        chosen
    });
});
