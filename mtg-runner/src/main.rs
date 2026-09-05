use std::env;
use std::fmt::Write;
use std::fs;
use std::path::{Path, PathBuf};

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;

use mtg_player::llm::MatchFormat;
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
  --p1 <spec>            Player 1: cli | random | claude[:model] | gemini[:model] | claude-code[:model]  (default cli)
  --p2 <spec>            Player 2: same specs  (default random)
                         Aliases: ai and llm mean claude; cc means claude-code.
                         claude/gemini seats call metered APIs (ANTHROPIC_API_KEY / GEMINI_API_KEY);
                         claude-code runs the same LLM seat through `claude -p` on the CLI's own login.
  --deck1 <name-or-file> Deck for player 1: built-in name or deck file  (default red-green)
  --deck2 <name-or-file> Deck for player 2  (default white-black)
  --seed <N>             Deterministic seed for shuffles and random players
  --on-the-play <1|2>    Which seat takes the first turn (default: random, CR 103.1)
  --log <path>           Append the game log to this file
  --save <path>          Continuously write a resumable save to this file. The
                         file is overwritten from the first decision, follows a
                         symlink, and is left in place at game over holding the
                         final position
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
            // Private entries are one player's hidden information (a "look
            // at", a fruitless search); the --log file is shared between
            // hotseat seats, so they never reach it (issue #119).
            mtg_engine::state::LogLevel::Private => continue,
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
fn write_save_atomically(path: &str, json: &str, private: bool) -> std::io::Result<()> {
    let tmp = format!("{}.{}.tmp", path, std::process::id());
    let write = write_file(&tmp, json, private).and_then(|()| fs::rename(&tmp, path));
    if write.is_err() {
        // The failure path used to be what stranded a partial temp file —
        // on a filesystem that just ran out of space, which is exactly when
        // the write fails, and every retry left another (issue #214).
        let _ = fs::remove_file(&tmp);
    }
    write
}

/// Write `json` to `path`, optionally with only this user able to read it.
///
/// The hot-reload snapshot is a complete game state — both players' hands
/// and both libraries in draw order — written to a world-readable /tmp at a
/// name derived only from the pid, so any other user on the box could read
/// a live game's answer key (issue #239). Nothing but this process ever
/// reads it, so 0600 costs nothing. A `--save` file is a path the operator
/// named and keeps the umask they expect.
fn write_file(path: &str, json: &str, private: bool) -> std::io::Result<()> {
    use std::io::Write;
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    if private {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(json.as_bytes())
}

/// Resolve a `--save` path that is a symlink to the file it points at.
///
/// The atomic write renames onto the path, and `rename(2)` does not follow
/// a final symlink: every save landed *next to* the link and destroyed the
/// link on the first write, while the target the operator pointed at stayed
/// empty. `--log`, which opens rather than renames, followed the link as
/// anyone would expect (issue #215). Following it here restores the
/// atomicity guarantee (the rename happens in the target's own directory)
/// and makes the startup writability probe test the directory that will
/// actually be written.
fn resolve_save_symlink(path: &str) -> String {
    let mut current = PathBuf::from(path);
    // Bounded, so a symlink loop is a refusal rather than a hang.
    for _ in 0..8 {
        let Ok(meta) = fs::symlink_metadata(&current) else { break };
        if !meta.file_type().is_symlink() {
            break;
        }
        let Ok(target) = fs::read_link(&current) else { break };
        current = if target.is_absolute() {
            target
        } else {
            current.parent().unwrap_or(Path::new(".")).join(target)
        };
    }
    let resolved = current.to_string_lossy().into_owned();
    if resolved != path {
        eprintln!("note: --save '{path}' is a symlink; writing through it to '{resolved}'");
    }
    resolved
}

/// Delete hot-reload snapshots left by runs that are no longer running.
///
/// The snapshot is unlinked on the way out, but no cleanup runs on a
/// `kill -9`, and one night of playtesting left 91 files and 8.6 MB of dead
/// game states in /tmp (issue #234). A file is named
/// `mtg-hot-reload-<pid>.json`; one whose pid is gone belongs to a dead run
/// and is ours to remove. A live pid is left alone.
fn sweep_stale_hot_reload_saves() {
    let Ok(entries) = fs::read_dir(std::env::temp_dir()) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix("mtg-hot-reload-") else { continue };
        let Some(pid) = rest.strip_suffix(".json").or_else(|| rest.split('.').next()) else { continue };
        let Ok(pid) = pid.parse::<i32>() else { continue };
        // `kill(pid, 0)` reads 0 as "my process group" and negatives as
        // other groups, so only a positive pid asks about one process.
        // ESRCH means gone; EPERM means alive and someone else's.
        let alive = pid > 0
            && unsafe { libc::kill(pid, 0) == 0 || *libc::__errno_location() == libc::EPERM };
        if !alive {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Refuse an argument vector the parser below wouldn't fully consume. Every
/// lookup in `main` is an exact-match position scan, so an unrecognized or
/// misspelled flag used to be silently dropped and its default silently used —
/// a typo'd --deck1 quietly played the wrong deck (issue #55).
fn validate_args(args: &[String]) {
    const VALUE_FLAGS: &[&str] = &["--p1", "--p2", "--deck1", "--deck2",
        "--seed", "--log", "--save", "--resume", "--on-the-play"];
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

    // --on-the-play 1|2 pins which seat takes the first turn; without it the
    // opening player is randomized per CR 103.1 (seeded, so reproducible) —
    // p0-always-on-the-play biased every recorded match (issue #112).
    let starting_player: Option<PlayerId> = args.iter().position(|a| a == "--on-the-play")
        .and_then(|i| args.get(i + 1))
        .map(|v| match v.as_str() {
            "1" => PlayerId(0),
            "2" => PlayerId(1),
            other => die(&format!("--on-the-play takes 1 or 2, got '{other}'")),
        });

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
    // `rename(2)` would replace a symlink instead of writing through it, so
    // resolve it before anything (including the probe below) uses the path.
    let save_file = save_file.map(|p| resolve_save_symlink(&p));
    if let Some(ref path) = save_file {
        // A path that already holds something is overwritten from the first
        // decision on. That is what "write a save to this file" means, but
        // it is worth one line when the operator has aimed it at a file that
        // was already there (issue #242).
        if fs::metadata(path).is_ok_and(|m| m.is_file()) {
            eprintln!("note: --save '{path}' already exists and will be overwritten");
        }
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
        for flag in ["--deck1", "--deck2"] {
            if args.iter().any(|a| a == flag) {
                eprintln!("note: {flag} is ignored with --resume; the save file's game wins");
            }
        }
        // --seed is NOT ignored, and calling it ignored was worse than
        // saying nothing: only the *engine* RNG comes from the save
        // (`GameState.rng_state`). The seats' RNG does not, so --seed still
        // seeds them, and dropping it — as the old note advised — is what
        // makes a resumed replay non-reproducible (issue #196).
        if args.iter().any(|a| a == "--seed") {
            eprintln!("note: --seed does not change the saved game's shuffle (the save's RNG wins), \
but it still seeds the random/AI seats — keep it to replay a resume deterministically");
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
                Some(s) => println!("MTG Engine — p0: {p1_spec} ({name1}) vs p1: {p2_spec} ({name2}) [seed {s}]"),
                None => println!("MTG Engine — p0: {p1_spec} ({name1}) vs p1: {p2_spec} ({name2})"),
            }
            println!();
        }

        let config = GameConfig {
            player_names: vec![name1.clone(), name2.clone()],
            decklists: vec![deck1, deck2],
            starting_life: 20,
            starting_player,
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
        // `player_names` holds the decks actually in play: on --resume the
        // save's decks win over flags (and their defaults), and the banner
        // must say so — it used to echo the default --deck1/--deck2 values,
        // contradicting the RESULT line in the same file (issue #94).
        // One naming scheme with the game log: the log says p0/p1, so the
        // banner states the mapping to the --p1/--p2 flags explicitly
        // instead of using a third name for each seat (issue #115).
        let meta = format!(
            "p0 (--p1): {} (deck: {})\np1 (--p2): {} (deck: {})",
            p1_model, player_names[0],
            p2_model, player_names[1],
        );
        mtg_player::game_log::write(file!(), line!(), "GAME_START", &meta);
    }

    // Initialize LLM player conversations with decklists.
    //
    // On --resume these come from the saved game, not from --deck1/--deck2:
    // those flags were just declared ignored, and loading them anyway made
    // an ignored flag fatal — `load_deck` die()s, so a resume aborted on a
    // deck file it did not need, even with no LLM seat in the game. A save
    // is meant to be self-sufficient, and the scratch deck file a game was
    // started from is often gone by the time it is resumed (issue #197).
    let (deck1_entries, deck2_entries) = if resume_file.is_some() {
        (decklist_from_state(&state, mtg_engine::ids::PlayerId(0), &registry),
         decklist_from_state(&state, mtg_engine::ids::PlayerId(1), &registry))
    } else {
        (load_deck(deck1_spec, &registry).entries,
         load_deck(deck2_spec, &registry).entries)
    };
    // Build a card reference from all cards in both decks
    let card_reference = build_card_reference(&deck1_entries, &deck2_entries, &registry);
    if let PlayerKind::Llm(ref mut llm) = p1 {
        llm.init_conversation(&deck1_entries, &card_reference, &registry, MatchFormat::SingleGame);
    }
    if let PlayerKind::Llm(ref mut llm) = p2 {
        llm.init_conversation(&deck2_entries, &card_reference, &registry, MatchFormat::SingleGame);
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

    // A signal (closed window, kill, timeout) landing while a CLI prompt
    // holds the terminal in raw mode must not leave the pty raw for the
    // shell (issue #78).
    if has_human {
        mtg_player::cli::install_terminal_restore_signal_handlers();
    }

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
    sweep_stale_hot_reload_saves();
    let hot_reload_path = std::env::temp_dir()
        .join(format!("mtg-hot-reload-{}.json", std::process::id()))
        .to_string_lossy()
        .into_owned();
    let hot_reload_ref = hot_reload_path.clone();
    // Cleared the first time the snapshot cannot be written.
    let hot_reload_ok = std::cell::Cell::new(true);
    // Take it with us however this process dies — Ctrl-C at a prompt, a
    // signal, a closed window. Only the normal-completion path used to
    // unlink it (issues #234, #239).
    mtg_player::cli::unlink_on_exit(&hot_reload_path);

    // Per-owner count of non-token objects, captured at the first decision
    // point. No effect in the pool creates or destroys real cards, so the
    // count must stay constant for the whole game (tokens come and go).
    let mut nontoken_baseline: Option<Vec<usize>> = None;
    // The previous decision point and the action chosen there, for the
    // transition invariants (what one action may and must have done).
    let mut last_decision: Option<(GameState, mtg_engine::actions::Action)> = None;

    let registry_ref = &registry;
    // High-water mark of engine log entries already streamed to --log
    // (issue #77). A Cell so both the per-decision callback and the
    // end-of-game flush below can advance it.
    let streamed_log = std::cell::Cell::new(0usize);
    let streamed_log_ref = &streamed_log;
    // The decision itself, separated so the callback can record what was chosen.
    let mut choose = |game_state: &GameState, acting_player: PlayerId, legal: &engine::LegalActions, action_count: u64| -> mtg_engine::actions::Action {
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
            let mut violations = if legal.resolution_prompt.is_some() {
                mtg_engine::invariants::check_core(game_state, registry_ref)
            } else {
                mtg_engine::invariants::check_settled(game_state, registry_ref)
            };
            if let Some((prev, act)) = &last_decision {
                violations.extend(mtg_engine::invariants::check_transition(prev, Some(act), game_state, registry_ref));
            }
            violations.extend(mtg_engine::invariants::check_legal(game_state, acting_player, legal, registry_ref));

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
            // The hot-reload snapshot, so `rr` works without --save. Nobody
            // asked for it, so a failure to write it must not take the
            // operator's game down — it used to `.expect()`, and a full
            // /tmp panicked the game with a backtrace while the --save
            // sibling three lines below failed cleanly (issue #214). Say so
            // once, stop writing it, and let the game go on; `rr` refuses
            // rather than reloading a snapshot that has gone stale.
            if hot_reload_ok.get() {
                if let Err(e) = write_save_atomically(&hot_reload_ref, &json, true) {
                    hot_reload_ok.set(false);
                    eprintln!("warning: cannot write the hot-reload snapshot to \
'{hot_reload_ref}': {e}. `rr` is disabled for the rest of this game; \
use --save if you need a resumable file.");
                }
            }
            // Also write user-specified save file if set. The path was
            // probed writable at startup, but the disk can still fill or the
            // file be replaced mid-game — that's a user-environment failure,
            // reported cleanly, not a panic (issue #69).
            if let Some(ref path) = save_file_ref {
                write_save_atomically(path, &json, false)
                    .unwrap_or_else(|e| die(&format!("failed to write save file '{path}': {e}")));
            }
        }

        let chosen = choose(game_state, acting_player, legal, action_count);
        if check_invariants {
            last_decision = Some((game_state.clone(), chosen.clone()));
        }
        chosen
    };


    // Under the checker every submitted action is a decision point, so the
    // event ledgers see the passes the loop would otherwise make silently.
    state.observe_every_submit = check_invariants;
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

    // The operator's save stays. It used to be unlinked here, silently and
    // unconditionally — which threw away the one artifact of the final
    // position, made `--resume` on the path they had been using all game
    // fail with a missing file, and destroyed whatever else happened to be
    // at that path, including the very save a `--resume x --save x` was
    // playing from (issues #237, #242). Saves are written *before* each
    // decision, so the last one on disk was the state one action before the
    // end; write the final state over it, which is the state worth keeping.
    if let Some(ref path) = save_file {
        let save = SaveData { state: state.clone(), player_names: player_names.clone() };
        match serde_json::to_string(&save) {
            Ok(json) => {
                if let Err(e) = write_save_atomically(path, &json, false) {
                    eprintln!("warning: could not write the final save to '{path}': {e}");
                }
            }
            Err(e) => eprintln!("warning: could not serialize the final position: {e}"),
        }
        // Only our own temp sibling is ours to remove.
        let _ = fs::remove_file(format!("{}.{}.tmp", path, std::process::id()));
    }
    // The hot-reload snapshot is this process's scratch file and goes with
    // it, on this path and on every other (see `unlink_on_exit` above).
    let _ = fs::remove_file(&hot_reload_path);
    let _ = fs::remove_file(format!("{}.{}.tmp", hot_reload_path, std::process::id()));

    // The CLI paints full frames without ever clearing on exit; printing the
    // summary straight onto the last frame merges it with stale rows. Wipe
    // the TUI first so the summary is the only thing on screen (issue #47).
    if has_human {
        mtg_player::cli::reset_terminal_for_exit();
    }

    // Say HOW the game was decided, not only who won (issue #86): every
    // lost player's recorded LossReason joins the headline.
    let losses: Vec<String> = state.players.iter()
        .filter(|p| p.lost)
        .filter_map(|p| p.loss_reason.map(|r|
            format!("{} {}", player_names[p.id.0 as usize], r.describe())))
        .collect();
    let loss_suffix = if losses.is_empty() {
        String::new()
    } else {
        format!(" ({})", losses.join("; "))
    };
    let result_msg = match &state.result {
        Some(mtg_engine::state::GameResult::Winner(id)) => {
            let name = &player_names[id.0 as usize];
            format!("Game over! {name} wins!{loss_suffix}")
        }
        Some(mtg_engine::state::GameResult::Draw) => {
            format!("Game over! It's a draw!{loss_suffix}")
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

/// An API key that is present but empty cannot authenticate any better than
/// one that is unset, so the seat guards reject both.
fn env_key_set(var: &str) -> bool {
    env::var(var).is_ok_and(|v| !v.trim().is_empty())
}

fn make_player(spec: &str, name: &str, seed: Option<u64>) -> PlayerKind {
    let (kind, model) = match spec.split_once(':') {
        Some((k, m)) => (k, Some(m)),
        None => (spec, None),
    };

    match kind {
        // A cli seat with no usable terminal (stdin redirected AND no
        // controlling tty) can never read a keystroke: every event read
        // fails and the first prompt becomes a silent 100%-CPU spin
        // (issue #103). Refuse it up front, like other arguments that
        // cannot work (#55/#69/#70).
        "cli" if !mtg_player::cli::terminal_available() => die(&format!(
            "--{} cli needs an interactive terminal (stdin is not a tty and \
             /dev/tty is unavailable); use --{} random or run under a tty",
            name.to_lowercase(), name.to_lowercase())),
        "cli" => PlayerKind::Cli(CliPlayer::new(name)),
        // Both API seats build a backend that unwraps the key out of the
        // environment, so a missing key surfaced as a panic and a backtrace
        // while every other unusable seat argument here refuses cleanly
        // (issues #55/#69/#70/#103). Pre-flight the variable instead. An
        // empty value is as unusable as an unset one, so treat it the same.
        "ai" | "llm" | "claude" if !env_key_set("ANTHROPIC_API_KEY") => die(&format!(
            "--{} {kind} needs an Anthropic API key: ANTHROPIC_API_KEY is not set; \
             use --{} claude-code to run the same seat through the Claude Code CLI, \
             or --{} random",
            name.to_lowercase(), name.to_lowercase(), name.to_lowercase())),
        "ai" | "llm" | "claude" => {
            let mut player = LlmPlayer::new(name);
            if let Some(m) = model {
                player = player.with_model(m);
            }
            PlayerKind::Llm(player)
        }
        "gemini" if !env_key_set("GEMINI_API_KEY") => die(&format!(
            "--{} gemini needs a Gemini API key: GEMINI_API_KEY is not set; \
             use --{} random",
            name.to_lowercase(), name.to_lowercase())),
        "gemini" => {
            let mut player = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                player = player.with_model(m);
            }
            PlayerKind::Llm(player)
        }
        // The same LLM seat driven through the Claude Code CLI (`claude -p`)
        // instead of the metered Messages API — plan quota, no API key.
        "claude-code" | "cc" if !mtg_player::llm::claude_code_available() => die(&format!(
            "--{} {kind} needs the Claude Code CLI: `{}` is not runnable (set {} to its path)",
            name.to_lowercase(), mtg_player::llm::claude_code_binary(),
            mtg_player::llm::CLAUDE_CODE_BINARY_ENV)),
        "claude-code" | "cc" => {
            let mut player = LlmPlayer::new_claude_code(name);
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
            "unknown player type '{other}' (expected cli, random, claude[:model], \
             gemini[:model], or claude-code[:model]; ai and llm are accepted for \
             claude, cc for claude-code)")),
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
/// The cards a player owns in a saved game, as decklist entries.
///
/// A save carries objects, not the decklist they were dealt from, so this
/// counts every non-token card the player owns across all zones — which is
/// exactly the deck they started with, since a card never changes owner
/// (CR 108.3). Used to build the LLM card reference on --resume, so the
/// save needs no deck file beside it.
fn decklist_from_state(
    state: &mtg_engine::state::GameState,
    player: mtg_engine::ids::PlayerId,
    registry: &CardRegistry,
) -> Vec<(String, u32)> {
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    for obj in state.objects_in_id_order() {
        if obj.owner != player || obj.is_token {
            continue;
        }
        let Some(data) = registry.card_data(obj.card_id) else { continue };
        *counts.entry(data.name.clone()).or_default() += 1;
    }
    counts.into_iter().collect()
}

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
        // A double-faced card contributes both faces, each under its own
        // name: the back face is what a transform decision is about, and
        // what the board line reads after the permanent flips (issue #205).
        for (face_name, data) in mtg_player::llm::card_faces(name, registry) {
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
            writeln!(s, "{}{} | {}{}{}", face_name, cost, types.join(" "), subtypes, pt).unwrap();
            if !data.oracle_text.is_empty() {
                writeln!(s, "  {}", data.oracle_text.replace('\n', "\n  ")).unwrap();
            }
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
