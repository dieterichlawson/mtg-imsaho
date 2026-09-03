//! Seeded random games as a correctness oracle.
//!
//! Two properties that need no judgment about what any card does:
//!
//! - **Replay determinism**: the same seed plays the same game, move for
//!   move. A divergence means something order-sensitive iterated a
//!   `HashMap`/`HashSet` (their order is seeded per process), which also
//!   means a real game's saves and replays are unsound.
//! - **State invariants**: `mtg_engine::invariants` holds at every decision
//!   point, and every game reaches a result.
//!
//! The runner exposes the same checks at scale (`--seed`,
//! `--check-invariants`, driven by `scripts/fuzz.sh`); this test keeps a
//! small battery in the suite so a regression is caught by `cargo test`.

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig, LegalActions};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::{GameState, LogLevel};
use mtg_engine::view::GameView;

use mtg_player::Player;
use mtg_player::random::RandomPlayer;

fn deck(entries: &[(&str, u32)]) -> Decklist {
    Decklist { entries: entries.iter().map(|(n, c)| ((*n).to_string(), *c)).collect() }
}

/// A deck that exercises tokens, DFC werewolves, curses, equipment, random
/// graveyard returns, and a planeswalker — the mechanics most likely to
/// carry ordering bugs.
fn deck_a() -> Decklist {
    deck(&[
        ("Forest", 9), ("Plains", 8),
        ("Villagers of Estwald", 3),
        ("Mayor of Avabruck", 2),
        ("Gatstaf Shepherd", 2),
        ("Avacyn's Pilgrim", 3),
        ("Doomed Traveler", 3),
        ("Mausoleum Guard", 2),
        ("Elder Cathar", 2),
        ("Woodland Sleuth", 2),
        ("Travel Preparations", 2),
        ("Rebuke", 2),
    ])
}

fn deck_b() -> Decklist {
    deck(&[
        ("Swamp", 9), ("Island", 8),
        ("Diregraf Ghoul", 3),
        ("Screeching Bat", 2),
        ("Vampire Interloper", 2),
        ("Stitcher's Apprentice", 2),
        ("Abattoir Ghoul", 2),
        ("Falkenrath Noble", 2),
        ("Liliana of the Veil", 1),
        ("Curse of Death's Hold", 1),
        ("Silver-Inlaid Dagger", 2),
        ("Moan of the Unhallowed", 2),
        ("Victim of Night", 2),
        ("Forbidden Alchemy", 2),
        // A structured (non-enumerated) cost prompt, so the game loop's
        // prompt delivery stays exercised by the in-suite battery.
        ("Corpse Lunge", 2),
    ])
}

struct GameOutcome {
    state: GameState,
    actions: u64,
    violations: Vec<String>,
}

/// Play one full seeded random-vs-random game, checking invariants at every
/// decision point. `max_actions` concedes to guarantee termination; a healthy
/// random game ends well under 1000 actions.
fn play(seed: u64, registry: &CardRegistry) -> GameOutcome {
    let config = GameConfig {
        player_names: vec!["A".into(), "B".into()],
        decklists: vec![deck_a(), deck_b()],
        starting_life: 20,
        starting_player: None,
        rng_seed: Some(seed),
    };
    let mut state = engine::setup_game(&config, registry);

    let mut players = [
        RandomPlayer::with_seed("A", seed.wrapping_add(1)),
        RandomPlayer::with_seed("B", seed.wrapping_add(2)),
    ];

    let mut actions: u64 = 0;
    let max_actions: u64 = 5_000;
    let mut violations: Vec<String> = Vec::new();
    let mut baseline: Option<Vec<usize>> = None;
    let mut last_decision: Option<(GameState, mtg_engine::actions::Action)> = None;

    let mut callback = |gs: &GameState, acting: PlayerId, legal: &LegalActions| {
        actions += 1;

        if violations.is_empty() {
            let mut found = if legal.resolution_prompt.is_some() {
                mtg_engine::invariants::check_core(gs, registry)
            } else {
                mtg_engine::invariants::check_settled(gs, registry)
            };
            if let Some((prev, act)) = &last_decision {
                found.extend(mtg_engine::invariants::check_transition(prev, Some(act), gs, registry));
            }
            let mut counts = vec![0usize; gs.players.len()];
            for obj in gs.objects.values() {
                if !obj.is_token {
                    counts[obj.owner.0 as usize] += 1;
                }
            }
            match &baseline {
                None => baseline = Some(counts),
                Some(base) if *base != counts => {
                    found.push(format!("non-token card counts changed: {base:?} -> {counts:?}"));
                }
                Some(_) => {}
            }
            if legal.actions.is_empty()
                && legal.combat_prompt.is_none()
                && legal.resolution_prompt.is_none()
            {
                found.push("no legal actions and no prompt: the game is stuck".into());
            }
            // Save/resume soundness: the state survives a serialization
            // round-trip with nothing lost. Compared as JSON values, not
            // bytes — HashMap-keyed fields serialize in per-instance order.
            // (Strided: it serializes everything.)
            if actions % 32 == 0 {
                let v1 = serde_json::to_value(gs).expect("state serializes");
                let reloaded: GameState =
                    serde_json::from_value(v1.clone()).expect("save deserializes");
                let v2 = serde_json::to_value(&reloaded).expect("reloaded state serializes");
                if v1 != v2 {
                    found.push("state does not survive a serialization round-trip".into());
                }
            }
            for msg in found {
                violations.push(format!(
                    "seed {seed}, action {actions}, turn {}, step {:?}: {msg}",
                    gs.turn_number, gs.step
                ));
            }
        }

        if actions >= max_actions {
            if let Some(a) = legal.actions.iter().find(|a| matches!(a, mtg_engine::actions::Action::Concede)) {
                return a.clone();
            }
        }

        let player = &mut players[acting.0 as usize];
        let chosen = if let Some(prompt) = &legal.combat_prompt {
            player.choose_combat(prompt)
        } else {
            let view = GameView::for_player(gs, acting, registry);
            player.choose_action(&view, legal)
        };
        last_decision = Some((gs.clone(), chosen.clone()));
        chosen
    };

    engine::run_game_loop(&mut state, registry, &mut callback);
    GameOutcome { state, actions, violations }
}

/// The engine's own log of the game, one line per entry — the trace two
/// replays of one seed are compared by.
fn trace(outcome: &GameOutcome) -> Vec<String> {
    outcome.state.game_log.iter()
        .filter(|e| e.level >= LogLevel::Info)
        .map(|e| e.message.clone())
        .collect()
}

#[test]
fn the_same_seed_replays_the_same_game() {
    let registry = CardRegistry::with_all_cards();
    for seed in [7u64, 42, 1234] {
        let a = play(seed, &registry);
        let b = play(seed, &registry);
        let (ta, tb) = (trace(&a), trace(&b));
        if ta != tb {
            let first = ta.iter().zip(tb.iter()).position(|(x, y)| x != y)
                .unwrap_or_else(|| ta.len().min(tb.len()));
            let context = |t: &[String]| t.iter()
                .skip(first.saturating_sub(3)).take(6)
                .cloned().collect::<Vec<_>>().join("\n    ");
            panic!(
                "seed {seed} diverged at log line {first} \
                 (run 1: {} lines / {} actions, run 2: {} lines / {} actions)\n\
                 run 1:\n    {}\nrun 2:\n    {}",
                ta.len(), a.actions, tb.len(), b.actions,
                context(&ta), context(&tb),
            );
        }
        assert_eq!(a.state.result, b.state.result, "seed {seed}: same log, different result");
    }
}

#[test]
fn seeded_random_games_hold_the_state_invariants() {
    let registry = CardRegistry::with_all_cards();
    for seed in 1u64..=8 {
        let outcome = play(seed, &registry);
        assert!(
            outcome.violations.is_empty(),
            "invariant violations:\n{}",
            outcome.violations.join("\n")
        );
        assert!(
            outcome.state.result.is_some(),
            "seed {seed}: game ended without a result after {} actions",
            outcome.actions
        );
    }
}
