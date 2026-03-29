use std::env;
use std::fs;

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;

use mtg_player::Player;
use mtg_player::cli::CliPlayer;
use mtg_player::llm::LlmPlayer;
use mtg_player::random::RandomPlayer;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SaveData {
    state: GameState,
    player_names: Vec<String>,
}

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

    // Deck specs: --deck1 <name-or-file> --deck2 <name-or-file>
    // Built-in deck names: red-green, white-black, blue-white,
    //   black-aggro, innistrad-white, innistrad-blue
    // Or a path to a deck file (one "COUNT CARD NAME" per line).
    let deck1_spec = args.iter().position(|a| a == "--deck1")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("red-green");

    let deck2_spec = args.iter().position(|a| a == "--deck2")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("white-black");

    let save_file = args.iter().position(|a| a == "--save")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let resume_file = args.iter().position(|a| a == "--resume")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let registry = CardRegistry::with_all_cards();

    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");

    let (player_names, mut state) = if let Some(ref path) = resume_file {
        let data = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read save file '{}': {}", path, e));
        let save: SaveData = serde_json::from_str(&data)
            .unwrap_or_else(|e| panic!("Failed to parse save file '{}': {}", path, e));
        if !quiet {
            println!("MTG Engine — resuming from {} (turn {}, {} vs {})",
                path, save.state.turn_number, save.player_names[0], save.player_names[1]);
            println!();
        }
        (save.player_names, save.state)
    } else {
        let deck1 = load_deck(deck1_spec, &registry);
        let deck2 = load_deck(deck2_spec, &registry);
        let name1 = deck_display_name(deck1_spec);
        let name2 = deck_display_name(deck2_spec);

        if !quiet {
            println!("MTG Engine — {} ({}) vs {} ({})", p1_spec, name1, p2_spec, name2);
            println!();
        }

        let config = GameConfig {
            player_names: vec![name1.clone(), name2.clone()],
            decklists: vec![deck1, deck2],
            starting_life: 20,
        };
        let player_names = config.player_names.clone();
        let state = engine::setup_game(&config, &registry);
        (player_names, state)
    };

    let mut p1 = make_player(p1_spec, "P1", log_file);
    let mut p2 = make_player(p2_spec, "P2", log_file);

    let has_human = matches!(p1, PlayerKind::Cli(_)) || matches!(p2, PlayerKind::Cli(_));

    let mut action_count: u64 = 0;
    let max_actions: u64 = 50_000;

    let save_file_ref = save_file.clone();
    let player_names_ref = player_names.clone();

    let mut game_callback = |game_state: &GameState, acting_player: PlayerId, legal: &engine::LegalActions| -> mtg_engine::actions::Action {
        action_count += 1;

        // Save state before each decision point.
        if let Some(ref path) = save_file_ref {
            let save = SaveData {
                state: game_state.clone(),
                player_names: player_names_ref.clone(),
            };
            let json = serde_json::to_string(&save).expect("Failed to serialize game state");
            fs::write(path, json).expect("Failed to write save file");
        }

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

        choose_action(player, &view, legal)
    };

    if resume_file.is_some() {
        engine::resume_game_loop(&mut state, &registry, &mut game_callback);
    } else {
        engine::run_game_loop(&mut state, &registry, &mut game_callback);
    }

    // Clean up save file when game completes.
    if let Some(ref path) = save_file {
        let _ = fs::remove_file(path);
    }

    match &state.result {
        Some(mtg_engine::state::GameResult::Winner(id)) => {
            let name = &player_names[id.0 as usize];
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

fn choose_action(player: &mut PlayerKind, view: &GameView, legal: &engine::LegalActions) -> mtg_engine::actions::Action {
    match player {
        PlayerKind::Cli(p) => p.choose_action(view, legal),
        PlayerKind::Llm(p) => p.choose_action(view, legal),
        PlayerKind::Random(p) => p.choose_action(view, legal),
    }
}

fn choose_combat(player: &mut PlayerKind, view: &GameView, prompt: &mtg_engine::actions::CombatPrompt) -> mtg_engine::actions::Action {
    match player {
        PlayerKind::Cli(p) => p.choose_combat(view, prompt),
        PlayerKind::Llm(p) => p.choose_combat(view, prompt),
        PlayerKind::Random(p) => p.choose_combat(prompt),
    }
}

/// Resolve a deck spec: either a built-in name or a file path.
fn load_deck(spec: &str, registry: &CardRegistry) -> Decklist {
    match builtin_deck(spec) {
        Some(deck) => deck,
        None => load_deck_file(spec, registry),
    }
}

/// Short display name for a deck spec.
fn deck_display_name(spec: &str) -> String {
    if builtin_deck(spec).is_some() {
        spec.to_string()
    } else {
        // Use filename without extension.
        std::path::Path::new(spec)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(spec)
            .to_string()
    }
}

/// Look up a built-in deck by name.
fn builtin_deck(name: &str) -> Option<Decklist> {
    Some(match name {
        "red-green" | "rg" => Decklist { entries: vec![
            ("Mountain".into(), 10),
            ("Forest".into(), 10),
            ("Goblin Piker".into(), 4),
            ("Grizzly Bears".into(), 4),
            ("Kalonian Tusker".into(), 4),
            ("Lightning Bolt".into(), 4),
            ("Giant Growth".into(), 4),
        ]},
        "white-black" | "wb" => Decklist { entries: vec![
            ("Plains".into(), 10),
            ("Swamp".into(), 10),
            ("Savannah Lions".into(), 4),
            ("Walking Corpse".into(), 4),
            ("Swords to Plowshares".into(), 4),
            ("Doom Blade".into(), 4),
            ("Holy Strength".into(), 2),
            ("Pacifism".into(), 2),
        ]},
        "blue-white" | "uw" => Decklist { entries: vec![
            ("Island".into(), 12),
            ("Plains".into(), 8),
            ("Coral Merfolk".into(), 4),
            ("Savannah Lions".into(), 4),
            ("Counterspell".into(), 4),
            ("Swords to Plowshares".into(), 4),
            ("Divination".into(), 4),
        ]},
        "black-aggro" | "ba" => Decklist { entries: vec![
            ("Swamp".into(), 14),
            ("Typhoid Rats".into(), 2),
            ("Diregraf Ghoul".into(), 4),
            ("Vampire Interloper".into(), 3),
            ("Walking Corpse".into(), 1),
            ("Markov Patrician".into(), 2),
            ("Falkenrath Noble".into(), 2),
            ("Village Cannibals".into(), 2),
            ("Skeletal Grimace".into(), 2),
            ("Dead Weight".into(), 2),
            ("Victim of Night".into(), 2),
            ("Bump in the Night".into(), 2),
            ("Moan of the Unhallowed".into(), 2),
        ]},
        "innistrad-white" | "iw" => Decklist { entries: vec![
            ("Plains".into(), 14),
            ("Savannah Lions".into(), 4),
            ("Chapel Geist".into(), 4),
            ("Abbey Griffin".into(), 4),
            ("Voiceless Spirit".into(), 4),
            ("Bonds of Faith".into(), 4),
            ("Moment of Heroism".into(), 3),
            ("Rally the Peasants".into(), 3),
        ]},
        "innistrad-blue" | "iu" => Decklist { entries: vec![
            ("Island".into(), 14),
            ("Fortress Crab".into(), 4),
            ("Moon Heron".into(), 4),
            ("Invisible Stalker".into(), 4),
            ("Claustrophobia".into(), 4),
            ("Sensory Deprivation".into(), 4),
            ("Hysterical Blindness".into(), 3),
            ("Spectral Flight".into(), 3),
        ]},
        "innistrad-green" | "ig" => Decklist { entries: vec![
            ("Forest".into(), 16),
            ("Ambush Viper".into(), 3),
            ("Grizzly Bears".into(), 3),
            ("Somberwald Spider".into(), 3),
            ("Lumberknot".into(), 2),
            ("Kindercatch".into(), 2),
            ("Prey Upon".into(), 3),
            ("Ranger's Guile".into(), 2),
            ("Travel Preparations".into(), 2),
            ("Spidery Grasp".into(), 2),
            ("Gnaw to the Bone".into(), 2),
        ]},
        _ => return None,
    })
}

/// Load a deck from a text file. Format: one "COUNT CARD NAME" per line.
/// Lines starting with # or empty lines are ignored.
///
/// Example:
///   4 Lightning Bolt
///   4 Goblin Piker
///   10 Mountain
fn load_deck_file(path: &str, registry: &CardRegistry) -> Decklist {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read deck file '{}': {}", path, e));

    let mut entries = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (count_str, card_name) = line.split_once(' ')
            .unwrap_or_else(|| panic!("{}:{}: expected 'COUNT CARD NAME', got '{}'", path, line_num + 1, line));
        let count: u32 = count_str.parse()
            .unwrap_or_else(|_| panic!("{}:{}: invalid count '{}'", path, line_num + 1, count_str));
        let card_name = card_name.trim();
        if registry.get_id_by_name(card_name).is_none() {
            panic!("{}:{}: unknown card '{}'", path, line_num + 1, card_name);
        }
        entries.push((card_name.to_string(), count));
    }

    if entries.is_empty() {
        panic!("Deck file '{}' is empty", path);
    }

    Decklist { entries }
}
