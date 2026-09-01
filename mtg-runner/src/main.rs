use std::env;
use std::fmt::Write;
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

const USAGE: &str = "\
mtg-runner — run one game of the MTG engine

Usage: mtg-runner [OPTIONS]

Options:
  --p1 <spec>            Player 1: cli | random | claude[:model] | gemini[:model]  (default cli)
  --p2 <spec>            Player 2: same specs  (default random)
  --deck1 <name-or-file> Deck for player 1: built-in name or deck file  (default red-green)
  --deck2 <name-or-file> Deck for player 2  (default white-black)
  --seed <N>             Deterministic seed for shuffles and random players
  --log <path>           Append the game log to this file
  --save <path>          Continuously write a resumable save to this file
  --resume <path>        Resume from a save file (saved decks/seed win over flags)
  --check-invariants     Check structural invariants at every decision point
  --quiet, -q            Suppress the pre-game banner
  --help, -h             Print this help and exit
  --version              Print the version and exit

Built-in decks: red-green (rg), white-black (wb), blue-white (uw),
black-aggro (ba), innistrad-white (iw), innistrad-blue (iu), innistrad-green (ig)";

/// A user error: report it and exit without a Rust panic/backtrace.
fn die(msg: &str) -> ! {
    eprintln!("Error: {msg}");
    std::process::exit(1);
}

/// Stream engine game-log entries `[from..]` to the `--log` file (a no-op
/// when `--log` wasn't given — the global writer isn't initialized) and
/// return the new high-water mark. This is what makes `--log` do what its
/// help text says: without it a 219-action game produced seven lines —
/// GAME_START and RESULT, no history — leaving crash triage blind and
/// every playtest issue screen-scraping its evidence from the LOG pane
/// (issue #77). Engine levels map Debug→DEBUG and everything else →INFO,
/// so the default view reads like the CLI's LOG pane while priority
/// passes and mana taps stay greppable underneath.
fn stream_game_log(state: &GameState, from: usize) -> usize {
    for entry in &state.game_log[from..] {
        let level = match entry.level {
            mtg_engine::state::LogLevel::Debug => mtg_player::game_log::LogLevel::Debug,
            _ => mtg_player::game_log::LogLevel::Info,
        };
        mtg_player::game_log::write_at(level, file!(), line!(), "GAME", &entry.message);
    }
    state.game_log.len()
}

/// Write a save via a same-directory temp file and `rename(2)`, so a reader
/// always sees either the previous complete save or the new complete save —
/// never a half-written one. Writing in place (`fs::write` = O_TRUNC +
/// write_all) tore ~12% of raw reads against a live writer (the 0-byte
/// O_TRUNC window, 64 KiB-boundary cuts) and let two runners on one path
/// braid their saves into one file (issue #75). The temp name carries the
/// pid so two writers can't braid the temp either — the last rename wins
/// whole, which is the most a shared path can promise.
fn write_save_atomically(path: &str, json: &str) -> std::io::Result<()> {
    let tmp = format!("{}.{}.tmp", path, std::process::id());
    fs::write(&tmp, json)?;
    fs::rename(&tmp, path)
}

/// Refuse an argument vector the parser below wouldn't fully consume. Every
/// lookup in `main` is an exact-match position scan, so an unrecognized or
/// misspelled flag used to be silently dropped and its default silently used —
/// a typo'd --deck1 quietly played the wrong deck (issue #55).
fn validate_args(args: &[String]) {
    const VALUE_FLAGS: &[&str] = &["--p1", "--p2", "--deck1", "--deck2",
        "--seed", "--log", "--save", "--resume"];
    const BOOL_FLAGS: &[&str] = &["--check-invariants", "--quiet", "-q"];
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        if VALUE_FLAGS.contains(&a) {
            if i + 1 >= args.len() {
                eprintln!("Error: {a} requires a value\n\n{USAGE}");
                std::process::exit(2);
            }
            i += 2;
        } else if BOOL_FLAGS.contains(&a) {
            i += 1;
        } else {
            eprintln!("Error: unrecognized argument '{a}'\n\n{USAGE}");
            std::process::exit(2);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return;
    }
    if args.iter().any(|a| a == "--version") {
        println!("mtg-runner {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    validate_args(&args);

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
        .map_or("cli", std::string::String::as_str);

    let p2_spec = args.iter().position(|a| a == "--p2")
        .and_then(|i| args.get(i + 1))
        .map_or("random", std::string::String::as_str);

    let log_file = args.iter().position(|a| a == "--log")
        .and_then(|i| args.get(i + 1))
        .map(std::string::String::as_str);

    // Deck specs: --deck1 <name-or-file> --deck2 <name-or-file>
    // Built-in deck names: red-green, white-black, blue-white,
    //   black-aggro, innistrad-white, innistrad-blue
    // Or a path to a deck file (one "COUNT CARD NAME" per line).
    let deck1_spec = args.iter().position(|a| a == "--deck1")
        .and_then(|i| args.get(i + 1))
        .map_or("red-green", std::string::String::as_str);

    let deck2_spec = args.iter().position(|a| a == "--deck2")
        .and_then(|i| args.get(i + 1))
        .map_or("white-black", std::string::String::as_str);

    let save_file = args.iter().position(|a| a == "--save")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let resume_file = args.iter().position(|a| a == "--resume")
        .and_then(|i| args.get(i + 1))
        .cloned();

    // --seed N makes the whole game deterministic: the engine's shuffles and
    // the random players' picks all derive from it, so a failure replays.
    let seed: Option<u64> = args.iter().position(|a| a == "--seed")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse().unwrap_or_else(|_| die(&format!("--seed takes a number, got '{s}'"))));

    // --check-invariants runs the structural GameState invariants at every
    // decision point and exits nonzero on the first violation.
    let check_invariants = args.iter().any(|a| a == "--check-invariants");

    let registry = CardRegistry::with_all_cards();

    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");

    // Fail fast on unwritable --log/--save paths: these are arguments, so a
    // bad one is reported like any other bad argument — a one-line Error and
    // exit 1 — not a panic with a backtrace after the game has already
    // started (issue #69, the --log/--save siblings of #52).
    if let Some(path) = log_file {
        mtg_player::game_log::init(path)
            .unwrap_or_else(|e| die(&format!("failed to open log file '{path}': {e}")));
    }
    if let Some(ref path) = save_file {
        // The probe never touches the save path itself — even a create+
        // delete leaves a momentary empty file that reads as a torn save to
        // anything polling the path (issue #75). A directory is checked
        // directly (the later rename would only fail mid-game), and
        // writability is proven on the same temp sibling the atomic write
        // uses, which also catches a missing parent and bad permissions.
        if fs::metadata(path).is_ok_and(|m| m.is_dir()) {
            die(&format!("cannot write save file '{path}': Is a directory"));
        }
        let probe = format!("{}.{}.tmp", path, std::process::id());
        fs::write(&probe, b"")
            .unwrap_or_else(|e| die(&format!("cannot write save file '{path}': {e}")));
        let _ = fs::remove_file(&probe);
    }

    let (player_names, mut state) = if let Some(ref path) = resume_file {
        // The saved game carries its own decks and RNG: flags that only
        // shape a NEW game are ignored, and silently ignored flags corrupt
        // repro provenance (issues #52/#55) — so say so.
        for flag in ["--deck1", "--deck2", "--seed"] {
            if args.iter().any(|a| a == flag) {
                eprintln!("note: {flag} is ignored with --resume; the save file's game wins");
            }
        }
        // Every failure here is a user-supplied file being wrong (a typo'd
        // path, a truncated or edited save) — report and exit(1), never
        // panic (issue #52).
        let data = fs::read_to_string(path)
            .unwrap_or_else(|e| die(&format!("failed to read save file '{path}': {e}")));
        let save: SaveData = serde_json::from_str(&data)
            .unwrap_or_else(|e| die(&format!("save file '{path}' is not a valid game save: {e}")));
        if save.player_names.len() != save.state.players.len()
            || save.state.players.len() != 2
        {
            die(&format!(
                "save file '{path}' is not a valid game save: {} players but {} player names",
                save.state.players.len(), save.player_names.len()));
        }
        // A schema-valid save can still describe an impossible game (an
        // out-of-range active_player panics deep in the engine; a phantom
        // library id silently plays a wrong game). Loading foreign state is
        // exactly where the structural invariants are needed, so they run
        // here unconditionally, not only under --check-invariants.
        let violations = mtg_engine::invariants::check_core(&save.state, &registry);
        if !violations.is_empty() {
            eprintln!("Error: save file '{path}' describes an invalid game state:");
            for v in &violations {
                eprintln!("  - {v}");
            }
            std::process::exit(1);
        }
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
            match seed {
                Some(s) => println!("MTG Engine — {p1_spec} ({name1}) vs {p2_spec} ({name2}) [seed {s}]"),
                None => println!("MTG Engine — {p1_spec} ({name1}) vs {p2_spec} ({name2})"),
            }
            println!();
        }

        let config = GameConfig {
            player_names: vec![name1.clone(), name2.clone()],
            decklists: vec![deck1, deck2],
            starting_life: 20,
            starting_player: None,
            // A fresh seed per game unless --seed pins one.
            rng_seed: seed,
        };
        let player_names = config.player_names.clone();
        let state = engine::setup_game(&config, &registry);
        (player_names, state)
    };


    let mut p1 = make_player(p1_spec, "P1", seed.map(|s| s.wrapping_add(1)));
    let mut p2 = make_player(p2_spec, "P2", seed.map(|s| s.wrapping_add(2)));

    // Log game metadata.
    {
        let p1_model = if let PlayerKind::Llm(ref llm) = p1 { llm.model_name().to_string() } else { p1_spec.to_string() };
        let p2_model = if let PlayerKind::Llm(ref llm) = p2 { llm.model_name().to_string() } else { p2_spec.to_string() };
        let meta = format!(
            "P1: {} (deck: {})\nP2: {} (deck: {})",
            p1_model, deck_display_name(deck1_spec),
            p2_model, deck_display_name(deck2_spec),
        );
        mtg_player::game_log::write(file!(), line!(), "GAME_START", &meta);
    }

    // Initialize LLM player conversations with decklists.
    let deck1_entries = load_deck(deck1_spec, &registry).entries;
    let deck2_entries = load_deck(deck2_spec, &registry).entries;
    // Build a card reference from all cards in both decks
    let card_reference = build_card_reference(&deck1_entries, &deck2_entries, &registry);
    if let PlayerKind::Llm(ref mut llm) = p1 {
        llm.init_conversation(&deck1_entries, &card_reference, &registry);
    }
    if let PlayerKind::Llm(ref mut llm) = p2 {
        llm.init_conversation(&deck2_entries, &card_reference, &registry);
    }

    // If resuming, feed the existing game log to LLM players so they
    // have context about what happened before the reload.
    if resume_file.is_some() {
        let full_log: Vec<String> = state.game_log.iter()
            .filter(|e| e.level >= mtg_engine::state::LogLevel::Info)
            .map(|e| e.message.clone())
            .collect();
        if let PlayerKind::Llm(ref mut llm) = p1 {
            llm.resume_from_log(&full_log, mtg_engine::ids::PlayerId(0));
        }
        if let PlayerKind::Llm(ref mut llm) = p2 {
            llm.resume_from_log(&full_log, mtg_engine::ids::PlayerId(1));
        }
    }

    let has_human = matches!(p1, PlayerKind::Cli(_)) || matches!(p2, PlayerKind::Cli(_));

    // Serializing the full game state (log included) every action is what
    // makes hot reload and --save work, but it turns long AI-vs-AI games
    // quadratic — the state grows with the log, and a 50k-action random game
    // writes it 50k times. Only pay for it when someone can actually use it.
    let write_saves = has_human || save_file.is_some();

    let mut action_count: u64 = 0;
    let max_actions: u64 = 50_000;

    let save_file_ref = save_file.clone();
    let player_names_ref = player_names.clone();
    // Always save to a hot-reload temp file so 'rr' can work without --save.
    // The path is per-process: a shared fixed path let concurrent runners
    // clobber each other's snapshots, silently swapping a hot-reloaded
    // player into a different game (playtest issue #37).
    let hot_reload_path = std::env::temp_dir()
        .join(format!("mtg-hot-reload-{}.json", std::process::id()))
        .to_string_lossy()
        .into_owned();
    let hot_reload_ref = hot_reload_path.clone();

    // Per-owner count of non-token objects, captured at the first decision
    // point. No effect in the pool creates or destroys real cards, so the
    // count must stay constant for the whole game (tokens come and go).
    let mut nontoken_baseline: Option<Vec<usize>> = None;

    let registry_ref = &registry;
    // High-water mark of engine log entries already streamed to --log
    // (issue #77). A Cell so both the per-decision callback and the
    // end-of-game flush below can advance it.
    let streamed_log = std::cell::Cell::new(0usize);
    let streamed_log_ref = &streamed_log;
    let mut game_callback = |game_state: &GameState, acting_player: PlayerId, legal: &engine::LegalActions| -> mtg_engine::actions::Action {
        action_count += 1;

        // Stream the engine's game log to --log as it grows, so the file
        // holds the full history the moment each decision is made — a game
        // killed mid-run still leaves the sequence that led there.
        streamed_log_ref.set(stream_game_log(game_state, streamed_log_ref.get()));

        if check_invariants {
            // A resolution prompt interrupts a spell or ability mid-effect,
            // before state-based actions have caught up; everything else is a
            // settled decision point (CR 704.3).
            let violations = if legal.resolution_prompt.is_some() {
                mtg_engine::invariants::check_core(game_state, registry_ref)
            } else {
                mtg_engine::invariants::check_settled(game_state, registry_ref)
            };

            let mut counts = vec![0usize; game_state.players.len()];
            for obj in game_state.objects.values() {
                if !obj.is_token {
                    counts[obj.owner.0 as usize] += 1;
                }
            }
            let mut extra = Vec::new();
            match &nontoken_baseline {
                None => nontoken_baseline = Some(counts),
                Some(base) if *base != counts => {
                    extra.push(format!("non-token card counts changed: {base:?} -> {counts:?}"));
                }
                Some(_) => {}
            }

            if legal.actions.is_empty() && legal.combat_prompt.is_none() && legal.resolution_prompt.is_none() {
                extra.push("no legal actions and no prompt: the game is stuck".to_string());
            }

            // Serialization round-trip: --save/--resume depend on a state
            // surviving serialize → deserialize with nothing lost. Compared
            // as JSON values, not bytes — HashMap-keyed fields serialize in
            // per-instance order. Checked on a stride because it serializes
            // the whole state.
            if action_count % 8 == 0 {
                match serde_json::to_value(game_state) {
                    Ok(v1) => match serde_json::from_value::<GameState>(v1.clone()) {
                        Ok(reloaded) => match serde_json::to_value(&reloaded) {
                            Ok(v2) if v2 != v1 => extra.push(
                                "state does not survive a serialization round-trip".to_string()),
                            Ok(_) => {}
                            Err(e) => extra.push(format!("reloaded state failed to serialize: {e}")),
                        },
                        Err(e) => extra.push(format!("state failed to deserialize from its own save: {e}")),
                    },
                    Err(e) => extra.push(format!("state failed to serialize: {e}")),
                }
            }

            let all: Vec<String> = violations.into_iter().chain(extra).collect();
            if !all.is_empty() {
                eprintln!("INVARIANT VIOLATION at action {action_count} (turn {}, step {:?}):",
                    game_state.turn_number, game_state.step);
                for msg in &all {
                    eprintln!("  - {msg}");
                    mtg_player::game_log::write(file!(), line!(), "INVARIANT", msg);
                }
                eprintln!("last game log entries:");
                let tail = game_state.game_log.len().saturating_sub(20);
                for entry in &game_state.game_log[tail..] {
                    eprintln!("  | {}", entry.message);
                }
                std::process::exit(2);
            }
        }

        // Save state before each decision point.
        if write_saves {
            let save = SaveData {
                state: game_state.clone(),
                player_names: player_names_ref.clone(),
            };
            let json = serde_json::to_string(&save).expect("Failed to serialize game state");
            // Always write hot-reload save.
            write_save_atomically(&hot_reload_ref, &json)
                .expect("Failed to write hot-reload save");
            // Also write user-specified save file if set. The path was
            // probed writable at startup, but the disk can still fill or the
            // file be replaced mid-game — that's a user-environment failure,
            // reported cleanly, not a panic (issue #69).
            if let Some(ref path) = save_file_ref {
                write_save_atomically(path, &json)
                    .unwrap_or_else(|e| die(&format!("failed to write save file '{path}': {e}")));
            }
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
    drop(game_callback);

    // Flush log entries written after the last decision point (final combat
    // damage, "wins the game") — the callback never sees them (issue #77).
    streamed_log.set(stream_game_log(&state, streamed_log.get()));

    // Check for hot reload request.
    if mtg_player::cli::HOT_RELOAD_REQUESTED.load(std::sync::atomic::Ordering::SeqCst) {
        eprintln!("\nHot reload requested. Rebuilding and relaunching...");
        // Build the project.
        let build_status = std::process::Command::new("cargo")
            .args(["build", "--release"])
            .status();
        match build_status {
            Ok(s) if s.success() => {
                use std::os::unix::process::CommandExt;

                // Re-exec with --resume pointing to the hot-reload save.
                let exe = env::current_exe().expect("Failed to get current exe path");
                let mut new_args: Vec<String> = env::args().collect();
                // Remove any existing --resume args.
                while let Some(pos) = new_args.iter().position(|a| a == "--resume") {
                    new_args.remove(pos); // --resume
                    if pos < new_args.len() { new_args.remove(pos); } // its value
                }
                // Add --resume with hot-reload path.
                new_args.push("--resume".into());
                new_args.push(hot_reload_path.clone());
                // Exec replaces the current process.
                let err = std::process::Command::new(&exe)
                    .args(&new_args[1..]) // skip argv[0]
                    .exec();
                eprintln!("Failed to exec: {err}");
                std::process::exit(1);
            }
            Ok(s) => {
                eprintln!("Build failed (exit code {:?}). Continuing with save at {}",
                    s.code(), hot_reload_path);
                eprintln!("Resume manually with: cargo run --release -- --resume {hot_reload_path}");
            }
            Err(e) => {
                eprintln!("Failed to run cargo build: {e}. Save at {hot_reload_path}");
            }
        }
        return;
    }

    // Clean up save file when game completes normally, plus any temp file a
    // crash mid-write could have stranded.
    if let Some(ref path) = save_file {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{}.{}.tmp", path, std::process::id()));
    }
    let _ = fs::remove_file(&hot_reload_path);
    let _ = fs::remove_file(format!("{}.{}.tmp", hot_reload_path, std::process::id()));

    // The CLI paints full frames without ever clearing on exit; printing the
    // summary straight onto the last frame merges it with stale rows. Wipe
    // the TUI first so the summary is the only thing on screen (issue #47).
    if has_human {
        mtg_player::cli::reset_terminal_for_exit();
    }

    let result_msg = match &state.result {
        Some(mtg_engine::state::GameResult::Winner(id)) => {
            let name = &player_names[id.0 as usize];
            format!("Game over! {name} wins!")
        }
        Some(mtg_engine::state::GameResult::Draw) => {
            "Game over! It's a draw!".to_string()
        }
        None => {
            "Game ended without a result.".to_string()
        }
    };
    let summary = format!("{}\nTotal actions: {}\nFinal turn: {}", result_msg, action_count, state.turn_number);
    println!("\n{summary}");
    mtg_player::game_log::write(file!(), line!(), "RESULT", &summary);

    // Log token usage per model.
    let usage = mtg_player::llm::get_llm_model_usage();
    if !usage.is_empty() {
        let mut usage_lines = String::new();
        for (model, stats) in &usage {
            writeln!(usage_lines,
                "{}: {} calls, {} input, {} output, {} cache_read, {} cache_create",
                model, stats.calls, stats.input, stats.output, stats.cache_read, stats.cache_create
            ).unwrap();
        }
        println!("{}", usage_lines.trim());
        mtg_player::game_log::write(file!(), line!(), "TOKEN_USAGE", usage_lines.trim());
    }
}

fn make_player(spec: &str, name: &str, seed: Option<u64>) -> PlayerKind {
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
            PlayerKind::Llm(player)
        }
        "gemini" => {
            let mut player = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                player = player.with_model(m);
            }
            PlayerKind::Llm(player)
        }
        "random" => PlayerKind::Random(match seed {
            Some(s) => RandomPlayer::with_seed(name, s),
            None => RandomPlayer::new(name),
        }),
        // An unrecognized value for a recognized flag is refused like an
        // unrecognized flag (issues #70/#55): substituting `random` played a
        // different game than the one requested, printed a winner, and
        // exited 0 — indistinguishable from a legitimate run.
        other => die(&format!(
            "unknown player type '{other}' (expected cli, random, claude[:model], or gemini[:model])")),
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
/// Build a card reference with oracle text for all unique cards across both decks.
fn build_card_reference(
    deck1: &[(String, u32)],
    deck2: &[(String, u32)],
    registry: &CardRegistry,
) -> String {
    use mtg_engine::types::CardType;
    let mut names: Vec<String> = deck1.iter().chain(deck2.iter()).map(|(n, _)| n.clone()).collect();
    names.sort();
    names.dedup();
    let mut s = String::new();
    for name in &names {
        let lookup = name.split(" // ").next().unwrap_or(name);
        let Some(id) = registry.get_id_by_name(lookup) else { continue };
        let Some(data) = registry.card_data(id) else { continue };
        let cost = data.cost.as_ref().map(|c| format!(" {c}")).unwrap_or_default();
        let types: Vec<&str> = data.card_types.iter().map(|t| match t {
            CardType::Creature => "Creature", CardType::Instant => "Instant",
            CardType::Sorcery => "Sorcery", CardType::Enchantment => "Enchantment",
            CardType::Artifact => "Artifact", CardType::Land => "Land",
            CardType::Planeswalker => "Planeswalker",
        }).collect();
        let subtypes = if data.subtypes.is_empty() { String::new() }
            else { format!(" — {}", data.subtypes.join(" ")) };
        let pt = match (data.power, data.toughness) {
            (Some(p), Some(t)) => format!(" {p}/{t}"),
            _ => String::new(),
        };
        writeln!(s, "{}{} | {}{}{}", name, cost, types.join(" "), subtypes, pt).unwrap();
        if !data.oracle_text.is_empty() {
            writeln!(s, "  {}", data.oracle_text.replace('\n', "\n  ")).unwrap();
        }
    }
    s
}

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
        .unwrap_or_else(|e| die(&format!(
            "'{path}' is not a built-in deck name and could not be read as a deck file: {e}")));

    let mut entries = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((count_str, card_name)) = line.split_once(' ') else {
            die(&format!("{}:{}: expected 'COUNT CARD NAME', got '{}'", path, line_num + 1, line));
        };
        let count: u32 = count_str.parse()
            .unwrap_or_else(|_| die(&format!("{}:{}: invalid count '{}'", path, line_num + 1, count_str)));
        let card_name = card_name.trim();
        if registry.get_id_by_name(card_name).is_none() {
            die(&format!("{}:{}: unknown card '{}'", path, line_num + 1, card_name));
        }
        entries.push((card_name.to_string(), count));
    }

    if entries.is_empty() {
        die(&format!("Deck file '{path}' is empty"));
    }

    Decklist { entries }
}
