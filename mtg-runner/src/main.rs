use std::env;

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::view::GameView;

use mtg_player::Player;
use mtg_player::cli::CliPlayer;
use mtg_player::llm::LlmPlayer;
use mtg_player::random::RandomPlayer;

enum PlayerKind {
    Cli(CliPlayer),
    Llm(LlmPlayer),
    Random(RandomPlayer),
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse player specs: --p1 <spec> --p2 <spec>
    // Spec formats:
    //   cli                          — human CLI player
    //   random                       — random AI
    //   claude                       — Claude with default model (sonnet)
    //   claude:claude-haiku-4-5-20251001  — Claude with specific model
    //   gemini                       — Gemini with default model (2.0-flash)
    //   gemini:gemini-2.5-flash      — Gemini with specific model
    let p1_spec = args.iter().position(|a| a == "--p1")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("cli");

    let p2_spec = args.iter().position(|a| a == "--p2")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("random");

    let log_file = args.iter().position(|a| a == "--log")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    let matchup = args.iter().position(|a| a == "--matchup")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("rg-wb");

    let registry = CardRegistry::with_all_cards();

    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");

    let config = match matchup {
        "rg-uw" => {
            if !quiet {
                println!("MTG Engine — {} (Red/Green) vs {} (Blue/White)", p1_spec, p2_spec);
                println!("R/G: Goblin Piker, Grizzly Bears, Kalonian Tusker, Lightning Bolt, Giant Growth");
                println!("U/W: Coral Merfolk, Savannah Lions, Counterspell, Swords to Plowshares, Divination");
                println!();
            }
            GameConfig {
                player_names: vec!["Red/Green".into(), "Blue/White".into()],
                decklists: vec![
                    deck_red_green(),
                    deck_blue_white(),
                ],
                starting_life: 20,
            }
        }
        _ => {
            if !quiet {
                println!("MTG Engine — {} (Red/Green) vs {} (White/Black)", p1_spec, p2_spec);
                println!("R/G: Goblin Piker, Grizzly Bears, Kalonian Tusker, Lightning Bolt, Giant Growth");
                println!("W/B: Savannah Lions, Walking Corpse, Swords to Plowshares, Doom Blade, Holy Strength, Pacifism");
                println!();
            }
            GameConfig {
                player_names: vec!["Red/Green".into(), "White/Black".into()],
                decklists: vec![
                    deck_red_green(),
                    deck_white_black(),
                ],
                starting_life: 20,
            }
        }
    };

    let mut state = engine::setup_game(&config, &registry);

    let mut p1 = make_player(p1_spec, "P1", log_file);
    let mut p2 = make_player(p2_spec, "P2", log_file);

    let has_human = matches!(p1, PlayerKind::Cli(_)) || matches!(p2, PlayerKind::Cli(_));

    let mut action_count: u64 = 0;
    let max_actions: u64 = 50_000;

    engine::run_game_loop(&mut state, &registry, |game_state, acting_player, legal| {
        action_count += 1;

        if action_count >= max_actions {
            if let Some(concede_idx) = legal.actions.iter().position(|a| matches!(a, mtg_engine::actions::Action::Concede)) {
                return legal.actions[concede_idx].clone();
            }
        }

        let view = GameView::for_player(game_state, acting_player, &CardRegistry::with_all_cards());

        let player = if acting_player == PlayerId(0) { &mut p1 } else { &mut p2 };

        // Show thinking spinner only if a human is playing — render from the
        // human's perspective so they see the board while the AI thinks.
        let _spinner = if has_human {
            let is_ai = matches!(player, PlayerKind::Llm(_));
            let will_call_api = is_ai && (
                legal.combat_prompt.is_some() ||
                !legal.actions.iter().all(|a| matches!(a,
                    mtg_engine::actions::Action::PassPriority | mtg_engine::actions::Action::Concede
                ))
            );
            if will_call_api {
                let human_id = if acting_player == PlayerId(0) { PlayerId(1) } else { PlayerId(0) };
                let human_view = GameView::for_player(game_state, human_id, &CardRegistry::with_all_cards());
                Some(mtg_player::cli::CliPlayer::start_thinking(&human_view))
            } else {
                None
            }
        } else {
            None
        };

        if let Some(prompt) = &legal.combat_prompt {
            return choose_combat(player, &view, prompt);
        }

        choose_action(player, &view, &legal.actions)
    });

    match &state.result {
        Some(mtg_engine::state::GameResult::Winner(id)) => {
            let name = &config.player_names[id.0 as usize];
            println!("\nGame over! {} wins!", name);
        }
        Some(mtg_engine::state::GameResult::Draw) => {
            println!("\nGame over! It's a draw!");
        }
        None => {
            println!("\nGame ended without a result.");
        }
    }

    println!("Total actions: {}", action_count);
    println!("Final turn: {}", state.turn_number);
}

fn make_player(spec: &str, name: &str, log_file: Option<&str>) -> PlayerKind {
    let (kind, model) = match spec.split_once(':') {
        Some((k, m)) => (k, Some(m)),
        None => (spec, None),
    };

    match kind {
        "cli" => PlayerKind::Cli(CliPlayer::new(name)),
        "ai" | "llm" | "claude" => {
            let mut player = LlmPlayer::new(name);
            if let Some(m) = model {
                player = player.with_model(m);
            }
            if let Some(path) = log_file {
                player = player.with_log(path);
            }
            PlayerKind::Llm(player)
        }
        "gemini" => {
            let mut player = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                player = player.with_model(m);
            }
            if let Some(path) = log_file {
                player = player.with_log(path);
            }
            PlayerKind::Llm(player)
        }
        "random" => PlayerKind::Random(RandomPlayer::new(name)),
        other => {
            eprintln!("Unknown player type '{}', using random", other);
            PlayerKind::Random(RandomPlayer::new(name))
        }
    }
}

fn choose_action(player: &mut PlayerKind, view: &GameView, actions: &[mtg_engine::actions::Action]) -> mtg_engine::actions::Action {
    match player {
        PlayerKind::Cli(p) => p.choose_action(view, actions),
        PlayerKind::Llm(p) => p.choose_action(view, actions),
        PlayerKind::Random(p) => p.choose_action(view, actions),
    }
}

fn choose_combat(player: &mut PlayerKind, view: &GameView, prompt: &mtg_engine::actions::CombatPrompt) -> mtg_engine::actions::Action {
    match player {
        PlayerKind::Cli(p) => p.choose_combat(view, prompt),
        PlayerKind::Llm(p) => p.choose_combat(view, prompt),
        PlayerKind::Random(p) => p.choose_combat(prompt),
    }
}

fn deck_red_green() -> Decklist {
    Decklist {
        entries: vec![
            ("Mountain".into(), 10),
            ("Forest".into(), 10),
            ("Goblin Piker".into(), 4),
            ("Grizzly Bears".into(), 4),
            ("Kalonian Tusker".into(), 4),
            ("Lightning Bolt".into(), 4),
            ("Giant Growth".into(), 4),
        ],
    }
}

fn deck_white_black() -> Decklist {
    Decklist {
        entries: vec![
            ("Plains".into(), 10),
            ("Swamp".into(), 10),
            ("Savannah Lions".into(), 4),
            ("Walking Corpse".into(), 4),
            ("Swords to Plowshares".into(), 4),
            ("Doom Blade".into(), 4),
            ("Holy Strength".into(), 2),
            ("Pacifism".into(), 2),
        ],
    }
}

fn deck_blue_white() -> Decklist {
    Decklist {
        entries: vec![
            ("Island".into(), 12),
            ("Plains".into(), 8),
            ("Coral Merfolk".into(), 4),
            ("Savannah Lions".into(), 4),
            ("Counterspell".into(), 4),
            ("Swords to Plowshares".into(), 4),
            ("Divination".into(), 4),
        ],
    }
}
