//! `mtg-draft-server`: host a booster draft that people and AI seats sit
//! at together. See `docs/draft-with-friends.md`.

use std::path::PathBuf;
use std::sync::Arc;

use mtg_draft::pack::{generate_draft_packs, SheetData};
use mtg_draft::set_data::SetData;
use mtg_draft_runner::card_lines::CardLines;
use mtg_draft_runner::draft_log::DraftLogger;
use mtg_draft_runner::lobby::{parse_seats, Lobby, LobbyConfig, SeatKind, TableSave};
use mtg_draft_runner::server::{self, ServerConfig, Shared};
use mtg_draft_runner::{die, install_panic_hook, llm_client};
use mtg_engine::cards::CardRegistry;

const USAGE: &str = "\
mtg-draft-server — host a booster draft for people and AI seats

Usage: mtg-draft-server --seats <list> [OPTIONS]

Options:
  --seats <list>         Who sits where, in seat order: a comma list of human,
                         ai, ai:<model spec> and cli, each optionally Nx-prefixed
                         (1xhuman,7xai). 2 to 8 seats.  (default human,ai,ai,ai)
  --ai <spec>            The model spec a bare `ai` seat uses  (default
                         cc:claude-haiku-5-5, the plan-quota seat on the model
                         that drafted and played best for its cost; never a
                         metered one by default)
  --set <name>           Set to draft, from data/sets/<name>.json  (default isd)
  --bind <addr>          Address to listen on  (default 127.0.0.1; 0.0.0.0 for
                         friends on the network)
  --port <N>             The lobby's port  (default 8800)
  --game-ports <lo-hi>   Ports the humans' game pages take  (default 8801-8899)
  --best-of <N>          Games per tournament match  (default 3)
  --seed <N>             Seed for packs, shuffles and play/draw; generated and
                         logged when not given
  --guide <path>         Draft guide file prepended to every AI seat's prompt
  --pick-seconds <N>     Pick for a human who has not picked in N seconds
                         (1 to 86400, as is --build-seconds)
  --build-seconds <N>    Finish a human's deck after N seconds of building: the
                         deck they last sent (basics fill a short one, up to
                         18 lands), or else the table's own
  --log <path>           The draft log  (default logs/draft-with-friends/draft.log);
                         decks/ and games/ go in the directory beside it
  --save <path>          Where the table's snapshot is written after every
                         change  (default table.save.json beside the log)
  --resume <path>        Carry on from a snapshot: the same seats and join
                         links, every pick, deck and finished match; a match
                         that was in progress is played again. Its seed, set,
                         seats and match length win over the flags
  --quiet, -q            No event lines on the terminal
  --help, -h             Print this help and exit
  --version              Print the version and exit

At the keyboard: Enter or `start` starts the draft before everybody has
joined (an absent seat is picked for until it joins), `kick <seat>` hands a
seat to the table for good, `status` says where everybody is, `quit` stops
the server.";

struct Args {
    seats: Vec<SeatKind>,
    set: String,
    bind: String,
    port: u16,
    game_ports: (u16, u16),
    best_of: usize,
    seed: u64,
    guide: Option<String>,
    pick_seconds: Option<u64>,
    build_seconds: Option<u64>,
    log: String,
    quiet: bool,
    save: Option<String>,
    resume: Option<String>,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut seats_spec = "human,ai,ai,ai".to_string();
    // Named, not the CLI's own default: that model has refused the game
    // prompt before, and this one picked and played closest to a stronger
    // model's choices at a twentieth of the price (reports/llm-cost.md,
    // phases 3 and 4).
    let mut default_ai = "cc:claude-haiku-5-5".to_string();
    let mut args = Args {
        seats: Vec::new(),
        set: "isd".to_string(),
        bind: "127.0.0.1".to_string(),
        port: 8800,
        game_ports: (8801, 8899),
        best_of: 3,
        seed: rand::random(),
        guide: None,
        pick_seconds: None,
        build_seconds: None,
        log: "logs/draft-with-friends/draft.log".to_string(),
        quiet: false,
        save: None,
        resume: None,
    };
    let mut i = 0;
    let value = |i: &mut usize, flag: &str| -> String {
        *i += 1;
        argv.get(*i).cloned().unwrap_or_else(|| die(&format!("{flag} needs a value")))
    };
    let number = |text: &str, flag: &str| -> u64 {
        text.parse().unwrap_or_else(|_| die(&format!("{flag}: '{text}' is not a number")))
    };
    while i < argv.len() {
        let flag = argv[i].as_str();
        match flag {
            "--help" | "-h" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            "--version" => {
                println!("mtg-draft-server {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "--quiet" | "-q" => args.quiet = true,
            "--seats" => seats_spec = value(&mut i, flag),
            "--ai" => default_ai = value(&mut i, flag),
            "--set" => args.set = value(&mut i, flag),
            "--bind" => args.bind = value(&mut i, flag),
            "--port" => args.port = u16::try_from(number(&value(&mut i, flag), flag))
                .unwrap_or_else(|_| die("--port: not a port")),
            "--game-ports" => {
                let text = value(&mut i, flag);
                let (lo, hi) = text.split_once('-')
                    .unwrap_or_else(|| die("--game-ports takes a range like 8801-8899"));
                let lo = u16::try_from(number(lo, flag)).unwrap_or_else(|_| die("--game-ports: not a port"));
                let hi = u16::try_from(number(hi, flag)).unwrap_or_else(|_| die("--game-ports: not a port"));
                if lo > hi {
                    die("--game-ports: the range is backwards");
                }
                args.game_ports = (lo, hi);
            }
            "--best-of" => {
                args.best_of = usize::try_from(number(&value(&mut i, flag), flag)).unwrap_or(0);
                if args.best_of == 0 {
                    die("--best-of: a match needs at least one game");
                }
            }
            "--seed" => args.seed = number(&value(&mut i, flag), flag),
            "--guide" => args.guide = Some(value(&mut i, flag)),
            "--pick-seconds" | "--build-seconds" => {
                let secs = timer_seconds(number(&value(&mut i, flag), flag), flag);
                if flag == "--pick-seconds" { args.pick_seconds = Some(secs) } else { args.build_seconds = Some(secs) }
            }
            "--log" => args.log = value(&mut i, flag),
            "--resume" => args.resume = Some(value(&mut i, flag)),
            "--save" => args.save = Some(value(&mut i, flag)),
            other => die(&format!("unknown flag '{other}' (see --help)")),
        }
        i += 1;
    }
    args.seats = parse_seats(&seats_spec, &default_ai).unwrap_or_else(|e| die(&format!("--seats: {e}")));
    args
}

/// The longest a pick or build timer may run: a day. A sentinel like
/// 2^64 overflowed the deadline and wedged the table (#812).
const MAX_TIMER_SECONDS: u64 = 86_400;

/// A timer's seconds, 1 to `MAX_TIMER_SECONDS`. Zero is refused, as
/// `--best-of 0` is: a 0-second build clock built every person's deck the
/// moment their draft ended, and leaving the flag out is how to have none.
fn timer_seconds(secs: u64, flag: &str) -> u64 {
    if !(1..=MAX_TIMER_SECONDS).contains(&secs) {
        die(&format!("{flag}: a timer runs 1 to {MAX_TIMER_SECONDS} seconds (leave the flag out for none)"));
    }
    secs
}

/// Refuse a model spec a seat cannot run before anything is dealt, as
/// the runner does.
fn check_models(seats: &[SeatKind]) {
    let cc_seats = seats.iter().filter(|s| matches!(s, SeatKind::Ai(m) if {
        let p = m.split(':').next().unwrap_or(""); p == "cc" || p == "claude-code"
    })).count();
    for (seat, kind) in seats.iter().enumerate() {
        let SeatKind::Ai(spec) = kind else { continue };
        let provider = spec.split(':').next().unwrap_or("");
        match provider {
            "claude" | "gemini" => {}
            "claude-code" | "cc" => {
                if !mtg_player::llm::claude_code_available() {
                    die(&format!(
                        "seat {seat} '{spec}' needs the Claude Code CLI: `{}` is not runnable (set {} to its path)",
                        mtg_player::llm::claude_code_binary(), mtg_player::llm::CLAUDE_CODE_BINARY_ENV));
                }
                if cc_seats > mtg_player::llm::CLAUDE_CODE_MAX_LIVE_CALLS {
                    die(&format!("{cc_seats} claude-code seats is more than can be taken down on Ctrl-C (limit {})",
                        mtg_player::llm::CLAUDE_CODE_MAX_LIVE_CALLS));
                }
            }
            other => die(&format!("seat {seat} '{spec}': unknown provider '{other}' (expected {})",
                llm_client::ACCEPTED_PROVIDERS)),
        }
    }
    if seats.contains(&SeatKind::Cli) {
        die("a cli seat is not implemented in this version: sit at a human seat and open its link \
on this machine (docs/draft-with-friends.md)");
    }
}

fn main() {
    install_panic_hook();
    let mut args = parse_args();
    // A snapshot names its own table: the seed, the set, the seats and the
    // match length are its, whatever the flags say.
    let save: Option<TableSave> = args.resume.as_ref().map(|path| {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| die(&format!("--resume: cannot read {path}: {e}")));
        let save: TableSave = serde_json::from_str(&text)
            .unwrap_or_else(|e| die(&format!("--resume: {path} is not a table snapshot: {e}")));
        save
    });
    if let Some(save) = &save {
        args.seats = save.seat_kinds().unwrap_or_else(|e| die(&format!("--resume: {e}")));
        args.seed = save.seed;
        args.set.clone_from(&save.set);
        args.best_of = save.best_of;
    }
    check_models(&args.seats);

    let guide = args.guide.as_deref().map(|path| std::fs::read_to_string(path)
        .unwrap_or_else(|e| die(&format!("failed to read guide file '{path}': {e}"))));

    let set_path = PathBuf::from(format!("data/sets/{}.json", args.set));
    let mut set_data = SetData::load(&set_path)
        .unwrap_or_else(|e| die(&format!("failed to load set data: {e}")));
    let registry = Arc::new(CardRegistry::with_all_cards());
    let removed = set_data.filter_implemented(&registry);
    if !removed.is_empty() && !args.quiet {
        mtg_player::stderr_line!("note: {} cards not implemented, removed from the draft pool", removed.len());
    }
    let sheets = SheetData::from_set_data(&set_data)
        .unwrap_or_else(|e| die(&format!("failed to build sheet data: {e}")));

    let web_dir = std::env::var(mtg_player::gui::WEB_DIR_ENV)
        .map_or_else(|_| PathBuf::from(mtg_player::gui::DEFAULT_WEB_DIR), PathBuf::from);
    if !web_dir.join("index.html").is_file() {
        die(&format!("the page directory is not at '{}' (no index.html there); run from the repository \
root or set {} to the mtg-gui directory", web_dir.display(), mtg_player::gui::WEB_DIR_ENV));
    }
    let page_missing = !web_dir.join(server::PAGE).is_file();

    let log_path = PathBuf::from(&args.log);
    if let Some(dir) = log_path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)
                .unwrap_or_else(|e| die(&format!("cannot create {}: {e}", dir.display())));
        }
    }
    // A resumed table opens its log after the replay, so what the snapshot
    // already recorded is not written to it twice.
    let log = if save.is_some() { DraftLogger::silent() } else { DraftLogger::new(&log_path) };
    let out_dir = mtg_draft_runner::lobby::out_dir_for(&log_path);

    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(args.seed);
    let packs = generate_draft_packs(&sheets, args.seats.len(), &mut rng);

    let card_lines = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);
    let card_reference = llm_client::build_card_reference(&set_data.all_card_names(), &registry);

    let config = LobbyConfig {
        set_code: args.set.clone(),
        set_name: set_data.set_name.clone(),
        seats: args.seats.clone(),
        best_of: args.best_of,
        seed: args.seed,
        guide_path: args.guide.clone(),
        pick_seconds: args.pick_seconds,
        build_seconds: args.build_seconds,
        out_dir: Some(out_dir.clone()),
    };
    let mut lobby = Lobby::new(config, &packs, &set_data, Arc::clone(&registry), &card_lines, log)
        .unwrap_or_else(|e| die(&format!("could not seat the table: {e}")));
    if let (Some(save), Some(path)) = (&save, &args.resume) {
        lobby.restore(save).unwrap_or_else(|e| die(&format!("--resume {path}: {e}")));
        let _ = DraftLogger::new(&log_path);
        let note = format!("the table carries on from the snapshot {path} (seed {}, seats {}): {} picks, {} \
decks and {} finished matches replayed; a match that was in progress is played again from its first game",
            save.seed, save.seats.iter().map(|s| s.kind.as_str()).collect::<Vec<_>>().join(","),
            save.picks.len(), save.decks.len(), save.matches.len());
        mtg_player::game_log::write(file!(), line!(), &format!("NOTE {note}"), "");
        mtg_player::stderr_line!("{note}");
    }
    let save_path = args.save.as_ref().map_or_else(|| out_dir.join("table.save.json"), PathBuf::from);

    let advertise = server::advertised_host(&args.bind);
    let host = advertise.clone().unwrap_or_else(|| args.bind.clone());
    let keys = lobby.human_keys();

    let shared = Shared::new(
        lobby,
        ServerConfig {
            bind: args.bind.clone(),
            port: args.port,
            advertise: host.clone(),
            game_ports: args.game_ports,
            web_dir,
            quiet: args.quiet,
            out_dir: Some(out_dir),
            guide,
            save: Some(save_path.clone()),
        },
        registry,
        card_lines,
        card_reference,
        set_data.set_name.clone(),
    );

    // The port first: nothing below is true of a table that cannot listen.
    let listener = server::bind(&shared).unwrap_or_else(|e| die(&e));

    mtg_player::stderr_line!("{} draft: {} seats, best-of-{}, seed {}; log {}",
        set_data.set_name, args.seats.len(), args.best_of, args.seed, args.log);
    mtg_player::stderr_line!("snapshot: {} (if the server stops, carry on with --resume {})",
        save_path.display(), save_path.display());
    if page_missing {
        mtg_player::stderr_line!("note: {}/{} is missing, so the browser page is not served; the terminal \
client still works", shared.config.web_dir.display(), server::PAGE);
    }
    for (seat, key) in &keys {
        mtg_player::stderr_line!("{}", server::join_lines(&host, args.port, *seat, key));
    }
    if args.bind == "0.0.0.0" && advertise.is_none() {
        mtg_player::stderr_line!("(replace 0.0.0.0 with this machine's address)");
    }
    if keys.is_empty() {
        mtg_player::stderr_line!("no human seats: the AI seats draft on their own");
        shared.lobby().start();
        shared.notify();
    } else if shared.lobby().phase() == mtg_draft_runner::lobby::Phase::Lobby {
        mtg_player::stderr_line!("the draft starts when everybody has joined, or press Enter to start without them");
    } else {
        // A table carried on from a snapshot is under way: the links are
        // the ones everybody already has.
        mtg_player::stderr_line!("the table carries on: everybody rejoins with the link they had");
        shared.notify();
    }

    // A Ctrl-C takes the seats' calls down at once and is then the
    // table's to act on: the log's last word, the pages told, the usage
    // summary — the same ending as `quit` (#823).
    mtg_player::llm::stop_gracefully_on_signal();
    if let Err(e) = server::run(&shared, listener) {
        die(&e);
    }
    // The log is the table's record: say who ended it, and where it was
    // (the first playtest's quit mid-match left the log ending in a usage
    // summary with no word of why).
    let reason = shared.stop_reason().unwrap_or_else(|| "the host quit".to_string());
    {
        let lobby = shared.lobby();
        let phase = format!("{:?}", lobby.phase()).to_lowercase();
        let unfinished = lobby.phase() != mtg_draft_runner::lobby::Phase::Done;
        mtg_player::game_log::write(file!(), line!(), &format!(
            "NOTE {reason}; the table was {phase}{}", if unfinished { " and is not finished" } else { "" }), "");
    }
    if mtg_player::llm::stop_signal().is_some() {
        mtg_player::stderr_line!("{reason}; the table is closed");
    }
    mtg_player::llm::claude_code_kill_live_calls();
    mtg_player::game_log::flush_all();
    let outcome = {
        let lobby = shared.lobby();
        if lobby.phase() == mtg_draft_runner::lobby::Phase::Done {
            llm_client::RunOutcome::Finished { total_games: lobby.games_played() }
        } else {
            llm_client::RunOutcome::Stopped
        }
    };
    llm_client::print_usage_summary(outcome);
    // The exit status a stopped process has, as if the signal had ended it.
    if let Some(sig) = mtg_player::llm::stop_signal() {
        std::process::exit(128 + sig);
    }
}
