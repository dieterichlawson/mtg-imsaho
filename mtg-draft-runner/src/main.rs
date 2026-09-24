use std::env;
use std::fs;
use std::path::PathBuf;

use mtg_draft::deckbuilding;
use mtg_draft::draft::DraftState;
use mtg_draft::pack::{generate_draft_packs, SheetData};
use mtg_draft::set_data::SetData;
use mtg_draft::tournament::{self, GameOutcome, MatchResult, Standing, Tournament, TournamentConfig, BYE};

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::{self, Decklist, GameConfig};
use mtg_engine::ids::PlayerId;
use mtg_engine::state::GameState;
use mtg_engine::view::GameView;

use mtg_player::llm::LlmPlayer;
use mtg_player::llm::MatchFormat;
use mtg_player::Player;

mod card_lines;
mod draft_log;
mod llm_client;

/// One row of the final standings, written once and printed by both surfaces
/// that show them — stderr and the log's FINAL STANDINGS block.
///
/// The row carries the seat's full match record and marks the wins that were
/// byes rather than matches played. Without the marker a seat that sat out a
/// round reads exactly like a seat that beat somebody, and the block does not
/// reconcile against the matches above it (issue #486, the shape of #195 and
/// #200).
pub(crate) fn standings_row(rank: usize, s: &Standing) -> String {
    let draws = if s.match_draws > 0 {
        format!("-{}", s.match_draws)
    } else {
        String::new()
    };
    let byes = match s.byes {
        0 => String::new(),
        1 => " [1 bye]".to_string(),
        n => format!(" [{n} byes]"),
    };
    format!(
        "{}. Seat {} — {}-{}{draws} ({} game wins){byes}",
        rank, s.seat, s.match_wins, s.match_losses, s.game_wins,
    )
}

/// Per-seat configuration used by [`play_match`].
struct PlayerSpec<'a> {
    seat: usize,
    deck: &'a Decklist,
    model_spec: &'a str,
    guide: Option<&'a str>,
}

/// (seat, pack, pool, picks) for a single player at a single pick step.
/// What one seat needs for one pick: its seat number, the pack in front
/// of it, and the pool it has drafted so far.
type PickInput = (usize, Vec<String>, Vec<String>);

// ─── CLI Argument Parsing ────────────────────────────────────────────

const USAGE: &str = "\
mtg-draft-runner — draft a set with LLM seats, then play a Swiss tournament

Usage: mtg-draft-runner [OPTIONS]

Options:
  --set <name>           Set to draft, from data/sets/<name>.json  (default isd)
  --players <N>          Number of drafters  (default 8)
  --model <spec>         Model for every seat  (default claude)
  --model-<N> <spec>     Model for seat N alone (0-based)
  --best-of <N>          Games per tournament match  (default 3)
  --seed <N>             Seed for packs, shuffles and play/draw. A run without
                         one generates a seed and logs it, so any draft can be
                         re-run by passing the seed from its log header
  --guide <path>         Draft guide file prepended to every seat's prompt
  --guide-<N> <path>     Draft guide file for seat N alone (0-based)
  --save <path>          Snapshot the draft here after every pick round, so a
                         failed model call costs one round and not the run
  --resume <path>        Replay a snapshot and carry on from it. Its seed, set
                         and seat count win over the flags — the packs are
                         re-dealt from the seed, so the position is exact
  --log <path>           Write the run log here  (default draft.log)
  --quiet, -q            Suppress progress output
  --help, -h             Print this help and exit
  --version              Print the version and exit

Model spec: provider[:model[:draft_thinking[:game_thinking]]]. claude and gemini
seats call metered APIs (ANTHROPIC_API_KEY / GEMINI_API_KEY); claude-code (alias
cc) runs the same seat through `claude -p` on the CLI's own login, for both the
draft and the games.";

/// A user error: report it and exit without a Rust panic/backtrace.
fn die(msg: &str) -> ! {
    eprintln!("Error: {msg}");
    // A run that stopped still spent what it spent. The usage summary was
    // printed only at the end of the happy path, so a seat's fatal, a
    // worker panic or a config error published no account of the
    // `claude -p` calls already paid for, and the resume that finished
    // reported a fragment labelled like a whole run (issue #578).
    llm_client::print_usage_summary(llm_client::RunOutcome::Stopped);
    // The summary's own log record is owed to the file now, for the same
    // reason the caller flushed before getting here.
    mtg_player::game_log::flush_here();
    // `process::exit` runs no destructors and raises no signal, so nothing
    // else takes this run's in-flight `claude -p` subprocesses down with
    // it. Every other seat is mid-call when one seat fatals — all seats
    // pick in parallel and the joins are walked in seat order — and each
    // one kept its whole process tree, orphaned to init and still spending
    // against a draft that had stopped (issue #537). Ctrl-C has swept them
    // since #206; the fatal path now sweeps the same registry.
    mtg_player::llm::claude_code_kill_live_calls();
    std::process::exit(1);
}

/// Silence the default panic output for a seat's fatal LLM failure.
///
/// Exhausting the retries is deliberately fatal, but it is an operational
/// condition — a usage limit, a CLI outage — not a bug, and the operator
/// used to get a worker-thread panic with a backtrace followed by a second
/// panic whose whole message was `Any { .. }`. The panic is still how the
/// worker unwinds; `report_worker_failure` prints the one line that
/// matters, so the hook keeps quiet for these and behaves normally for a
/// real bug (issue #218).
fn install_panic_hook() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let msg = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied());
        if msg.is_some_and(|m| m.starts_with(llm_client::FATAL_MARKER)) {
            return;
        }
        default(info);
    }));
}

/// Turn a joined worker's panic payload into one operator-facing line.
///
/// `context` says where the run stopped — the seat, and the pack and pick
/// it was on — which the payload itself does not know.
fn report_worker_failure(payload: &Box<dyn std::any::Any + Send>, context: &str) -> ! {
    // The first seat to get here is the one whose account the operator
    // reads; the rest are about to be killed with the process and have
    // nothing to add. Without this, two seats failing at once would race
    // each other to stderr with two accounts of one stop.
    // Whatever this worker was holding back for the deterministic flush is
    // owed to the log now: `process::exit` will not come back for it.
    mtg_player::game_log::flush_here();
    static REPORTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if REPORTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("worker thread failed");
    let msg = msg.strip_prefix(llm_client::FATAL_MARKER).unwrap_or(msg);
    die(&format!("{context}: {msg}"));
}

/// Run one seat's work, reporting a fatal failure where and when it happens.
///
/// The join loop used to be what discovered a failure, and that made the
/// run's account of what broke wrong in two ways (issue #539).
///
/// `handles.into_iter().enumerate()` walks seats 0, 1, 2, … and the first
/// `Err` ends the process, so the operator was told about the
/// *lowest-numbered* failed seat rather than the one that actually broke.
/// That is frequently the derived failure and not the real one: a seat that
/// merely timed out gets the headline while the seat that failed outright,
/// first, and for a nameable reason is not mentioned at all.
///
/// And a fatal in seat N was not printed until seats 0..N-1 had returned, so
/// a perfectly healthy but slow seat 0 held the whole run silent — with
/// shipped defaults, up to ten minutes of a run that was already dead,
/// showing nothing but `Pack 1 Pick 1/14`.
///
/// Reporting from the failing worker makes the first failure *by the clock*
/// the one that is reported, and makes it arrive when it happens. The join
/// arms stay as a backstop for a panic this never saw.
fn in_seat<T>(context: &str, body: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => report_worker_failure(&payload, context),
    }
}

/// Spawn a scoped worker whose thread is named for the seat it is.
///
/// Every line `game_log` writes carries the thread it came from, and for a
/// seat's own records — `API_FATAL`, `API_ERROR`, and the tournament's
/// whole LLM round trip — that was a bare `t5`, the one identifier that
/// means nothing to anybody. After a real run stopped there was no way back
/// from the log to the seat whose account or session was the broken one
/// (issues #539, #542). Naming the thread labels every line it writes, at
/// the one place a worker is created rather than at each call site.
fn spawn_seat<'scope, 'env, T: Send + 'scope>(
    s: &'scope std::thread::Scope<'scope, 'env>,
    name: String,
    body: impl FnOnce() -> T + Send + 'scope,
) -> std::thread::ScopedJoinHandle<'scope, T> {
    std::thread::Builder::new()
        .name(name)
        .spawn_scoped(s, body)
        .expect("a worker thread")
}

struct Args {
    set: String,
    players: usize,
    /// Per-player model specs. --model sets the default, --model-N overrides for player N.
    models: Vec<String>,
    best_of: usize,
    guides: Vec<Option<String>>,
    /// Where each seat's guide was read from, for the log header. The text
    /// alone doesn't say which file a seat was handed (issue #207).
    guide_paths: Vec<Option<String>>,
    /// Root seed for every random choice the run makes. Always set: a run
    /// given no `--seed` generates one and logs it, so that a draft nobody
    /// thought to seed can still be re-run afterwards (issue #212).
    seed: u64,
    log: String,
    quiet: bool,
    /// Where to snapshot the draft after every pick round.
    ///
    /// An eight-seat draft is 360 picks against a metered account, and a
    /// single failed model call ended the whole run with nothing written but
    /// a log — an hour of quota for a six-second hiccup (issues #212, #218).
    save: Option<String>,
    /// A snapshot to replay before drafting resumes. The packs come from the
    /// save's seed, so replaying the recorded picks reproduces the position
    /// exactly, without spending anything.
    resume: Option<String>,
    /// The ingredients the per-seat vectors above are built from, kept so
    /// that they can be rebuilt once the seat count is final.
    ///
    /// A resume's snapshot decides the seat count, and growing the pod used
    /// to `resize` the vectors in place: `models` padded with `models[0]`,
    /// so a global `--model` survived, but `guides` padded with `None`, so
    /// a global `--guide` — documented as "prepended to every seat's
    /// prompt" — reached only the first `--players` seats and the rest
    /// drafted with no guide at all, silently (#580).
    default_model: String,
    global_guide_path: Option<String>,
    /// `--model-N` / `--guide-N`, by seat.
    model_overrides: Vec<(usize, String)>,
    guide_overrides: Vec<(usize, String)>,
    /// Every `--model-N` / `--guide-N` the operator wrote, for the range
    /// check. It has to run against the seat count the draft will actually
    /// have: run in `parse_args`, before the snapshot is read, it refused
    /// `--guide-3` on a resume of a four-seat save whenever the flags said
    /// `--players 2` — a seat the run does have (#580).
    per_seat_flags: Vec<(String, usize)>,
    /// The flags the operator actually wrote, as opposed to the values that
    /// ended up in the fields above.
    ///
    /// They are not the same thing, and a resume's reconciliation note used
    /// to confuse them: `--seed` defaults to a fresh `rand::random::<u64>()`
    /// (#212), so the most ordinary resume there is — `--resume <save>` with
    /// no `--seed`, which is what the flag's help tells you to do — printed
    /// `note: --seed comes from the save (8502523799382835523 -> 3)`,
    /// announcing the override of a 19-digit number the operator never saw,
    /// that appears in no log and no snapshot, and that was invented one
    /// line earlier to be discarded (#582).
    supplied: std::collections::HashSet<String>,
}

impl Args {
    /// Whether the operator wrote this flag themselves.
    fn was_supplied(&self, flag: &str) -> bool {
        self.supplied.contains(flag)
    }

    /// Build the per-seat vectors for a pod of `players`, from the flags as
    /// written.
    ///
    /// Called once with the flag's seat count and again, when a snapshot
    /// overrides it, with the save's — so that a global `--model` or
    /// `--guide` reaches every seat the draft actually has, a `--model-N` /
    /// `--guide-N` for a seat the save adds is applied rather than dropped,
    /// and the range check refuses only the seats that really are outside
    /// the pod (#580).
    fn resolve_seats(&mut self, players: usize) {
        self.players = players;

        self.models = vec![self.default_model.clone(); players];
        for (seat, model) in self.model_overrides.iter().filter(|(s, _)| *s < players) {
            self.models[*seat] = model.clone();
        }

        let global_guide = self.global_guide_path.as_deref().map(read_guide);
        self.guides = vec![global_guide; players];
        self.guide_paths = vec![self.global_guide_path.clone(); players];
        for (seat, path) in self.guide_overrides.iter().filter(|(s, _)| *s < players) {
            self.guides[*seat] = Some(read_guide(path));
            self.guide_paths[*seat] = Some(path.clone());
        }
    }

    /// What each seat is drafting under, for the snapshot.
    fn seat_policies(&self) -> Vec<SeatPolicy> {
        (0..self.players)
            .map(|seat| SeatPolicy {
                model: self.models[seat].clone(),
                guide_path: self.guide_paths[seat].clone(),
                guide_hash: self.guides[seat].as_deref().map(guide_digest),
            })
            .collect()
    }

    /// Refuse a `--model-N` / `--guide-N` that names a seat the draft does
    /// not have.
    ///
    /// Run once, against the seat count the draft will actually be played
    /// with. Run in `parse_args` it used the *flag's* count, which a
    /// snapshot can still override, so it refused `--guide-3` on a resume
    /// of a four-seat save whenever the flags said `--players 2` — a seat
    /// the run does have (#580).
    fn check_seat_flags(&self) {
        for (flag, index) in &self.per_seat_flags {
            if *index >= self.players {
                die(&format!(
                    "{flag}: there is no seat {index} with {} players", self.players));
            }
        }
    }
}

/// An unreadable guide file is fatal: drafting without the guide the caller
/// asked for is a different draft than the one requested.
fn read_guide(path: &str) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| die(&format!("failed to read guide file '{path}': {e}")))
}

/// One pick, as the snapshot records it.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct PickRecord {
    round: usize,
    pick: usize,
    seat: usize,
    card: String,
    /// The runner made this pick because the seat's answer was unusable.
    /// A resumed run has to carry that forward: a snapshot of a seat that
    /// never chose a card must not replay into a clean draft (issue #401).
    /// Snapshots written before the field existed read as not substituted,
    /// which is the most a replay can say about them.
    #[serde(default)]
    substituted: bool,
}

/// What one seat was told to do while it was making the recorded picks.
///
/// A guide changes what a seat does more than any other flag, and none of
/// it was in the snapshot: `--resume` took the guide and the model from
/// whatever the operator happened to type, so resuming a draft under a
/// different guide produced one pool, one set of decks and one set of
/// standings assembled from picks made under two sets of instructions —
/// with no note, no warning and exit 0. The log header then named only the
/// guide the last process used, so the surviving artefact positively
/// asserted something false about the replayed picks (issue #579).
///
/// The hash is of the guide's contents, because a guide file can be edited
/// between two runs and the path alone would not notice.
#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Eq, Default, Debug)]
struct SeatPolicy {
    model: String,
    guide_path: Option<String>,
    guide_hash: Option<String>,
}

impl SeatPolicy {
    /// How a mismatch reads in a warning.
    fn describe(&self) -> String {
        match (&self.guide_path, &self.guide_hash) {
            (Some(path), _) => format!("model {}, guide {path}", self.model),
            (None, _) => format!("model {}, no guide", self.model),
        }
    }
}

/// A 64-bit FNV-1a of a guide's contents, rendered hex.
///
/// Only has to notice an edit, so it is arithmetic rather than a
/// dependency — and, like `match_seed`, it must not depend on a hasher
/// whose output may change between compiler versions and silently
/// invalidate every snapshot written before the upgrade.
fn guide_digest(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// A draft in progress: enough to deal the same packs again and replay every
/// pick that has been made.
#[derive(serde::Serialize, serde::Deserialize)]
struct DraftSave {
    seed: u64,
    set: String,
    players: usize,
    picks: Vec<PickRecord>,
    /// What each seat was drafting under when these picks were made.
    ///
    /// Empty in a snapshot written before the field existed, which reads as
    /// "unknown" rather than as agreement with the current flags — the most
    /// a replay can say about them (issue #579).
    #[serde(default)]
    seats: Vec<SeatPolicy>,
}

/// Refuse an argument vector `parse_args` wouldn't fully consume, and hand
/// back the `--model-N` / `--guide-N` flags it saw so their seat numbers can
/// be range-checked once `--players` is known. Every lookup below is an
/// exact-match position scan, so an unrecognized or misspelled flag was
/// silently dropped and its default silently used — a typo'd `--model` drafted
/// with a model nobody asked for, on a seat that bills per token.
fn validate_args(args: &[String]) -> Vec<(String, usize)> {
    const VALUE_FLAGS: &[&str] = &["--set", "--players", "--model", "--best-of", "--guide", "--log", "--seed", "--save", "--resume"];
    const BOOL_FLAGS: &[&str] = &["--quiet", "-q"];
    let mut indexed = Vec::new();
    let mut i = 1;
    while i < args.len() {
        let a = args[i].as_str();
        let per_seat = seat_flag(a);
        if VALUE_FLAGS.contains(&a) || per_seat.is_some() {
            if i + 1 >= args.len() {
                eprintln!("Error: {a} requires a value\n\n{USAGE}");
                std::process::exit(2);
            }
            if let Some(index) = per_seat {
                indexed.push((a.to_string(), index));
            }
            i += 2;
        } else if BOOL_FLAGS.contains(&a) {
            i += 1;
        } else {
            eprintln!("Error: unrecognized argument '{a}'\n\n{USAGE}");
            std::process::exit(2);
        }
    }
    indexed
}

/// The seat number of a `--model-N` / `--guide-N` flag, if it is one.
fn seat_flag(arg: &str) -> Option<usize> {
    let n = arg.strip_prefix("--model-").or_else(|| arg.strip_prefix("--guide-"))?;
    n.parse().ok()
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    // --help used to start a real eight-seat draft, so a typo cost money on
    // a metered seat. Both of these answer and exit without drafting.
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        std::process::exit(0);
    }
    if args.iter().any(|a| a == "--version") {
        println!("mtg-draft-runner {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    let per_seat_flags = validate_args(&args);

    let get = |flag: &str| -> Option<String> {
        args.iter()
            .position(|a| a == flag)
            .and_then(|i| args.get(i + 1)).cloned()
    };
    let count = |flag: &str, default: usize| -> usize {
        get(flag).map_or(default, |s| {
            let n = s.parse().unwrap_or_else(|_| die(&format!("{flag} takes a number, got '{s}'")));
            if n == 0 {
                die(&format!("{flag} must be at least 1"));
            }
            n
        })
    };

    let set = get("--set").unwrap_or_else(|| "isd".to_string());
    let players = count("--players", 8);
    let default_model = get("--model").unwrap_or_else(|| "claude".to_string());
    let best_of = count("--best-of", 3);
    let log = get("--log").unwrap_or_else(|| "draft.log".to_string());
    let quiet = args.iter().any(|a| a == "--quiet" || a == "-q");
    // No --seed means "pick one and write it down", not "be unrepeatable":
    // an interesting draft is usually only recognized as interesting after
    // it has finished.
    let seed = get("--seed").map_or_else(rand::random::<u64>, |s| {
        s.parse().unwrap_or_else(|_| die(&format!("--seed takes a number, got '{s}'")))
    });

    // --model-N / --guide-N, collected as written. They are applied — and
    // a seat number outside the pod refused — by `resolve_seats`, once the
    // seat count is final, because a resume's snapshot can still change it.
    //
    // A --model-N or --guide-N naming a seat outside the pod used to be read
    // by nobody — the same silent no-op as a misspelled flag, so it is
    // refused the same way.
    let seat_overrides = |prefix: &str| -> Vec<(usize, String)> {
        per_seat_flags
            .iter()
            .filter(|(flag, _)| flag.starts_with(prefix))
            .filter_map(|(flag, i)| get(flag).map(|v| (*i, v)))
            .collect()
    };
    let model_overrides = seat_overrides("--model-");
    let guide_overrides = seat_overrides("--guide-");

    // --guide applies to all seats, --guide-N overrides one; both are
    // resolved by `resolve_seats` below.
    let mut parsed = Args {
        set,
        players,
        models: Vec::new(),
        best_of,
        guides: Vec::new(),
        guide_paths: Vec::new(),
        seed,
        log,
        quiet,
        save: get("--save"),
        resume: get("--resume"),
        default_model,
        global_guide_path: get("--guide"),
        model_overrides,
        guide_overrides,
        per_seat_flags,
        supplied: args
            .iter()
            .filter(|a| a.starts_with("--"))
            .cloned()
            .collect(),
    };
    parsed.resolve_seats(players);
    parsed
}

/// The seed for one match, derived from the run's root seed and the match's
/// coordinates rather than drawn from a shared RNG.
///
/// The matches in a round are played on parallel threads, so anything drawn
/// sequentially would depend on thread scheduling and the run would not
/// replay. Derived this way, a match's games are the same games whichever
/// order the threads happen to run in.
///
/// The mix is SplitMix64's finalizer over the coordinates — plain arithmetic,
/// so it does not depend on a hasher whose output may change between compiler
/// versions and silently break replay of an older run.
fn match_seed(root: u64, round: usize, seat_a: usize, seat_b: usize) -> u64 {
    let mut z = root
        ^ (round as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (seat_a as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ (seat_b as u64).wrapping_mul(0x94D0_49BB_1331_11EB);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One round-trip with the model during deck building.
struct DeckAttempt {
    prompt: String,
    response: String,
    error: Option<String>,
}

/// Return type for deck building LLM interaction.
struct DeckBuildResult {
    deck: deckbuilding::DraftDeck,
    attempts: Vec<DeckAttempt>,
    retries: usize,
    /// True when no attempt produced a valid deck and the runner
    /// substituted one. A substituted deck is not a drafted deck, and
    /// every record of the run has to say so (issue #200).
    fallback: bool,
}

/// Validate model specs before starting the draft. Catches invalid thinking
/// levels and other config errors so we fail fast rather than silently
/// falling back to defaults mid-draft.
fn validate_model_specs(models: &[String]) {
    // Known thinking-level constraints per model family.
    // Models not listed here accept any level the API supports.
    let restricted: &[(&str, &[&str])] = &[
        ("gemini-3-pro", &["low", "high"]),
        ("gemini-3.0-pro", &["low", "high"]),
        ("gemini-3.1-pro", &["low", "high"]),
    ];

    let valid_levels = ["minimal", "low", "medium", "high"];

    for (i, spec) in models.iter().enumerate() {
        let parts: Vec<&str> = spec.split(':').collect();
        let provider = parts[0];
        match provider {
            "claude" => continue,
            // Every decision a claude-code seat makes shells out to `claude`.
            // Unchecked, a run on a machine without the binary drafted the
            // whole pod first — billing the other seats — and only then
            // failed every game decision. Refuse before anything is spent.
            "claude-code" | "cc" => {
                if !mtg_player::llm::claude_code_available() {
                    die(&format!(
                        "seat {i} model '{spec}' needs the Claude Code CLI: `{}` is not runnable (set {} to its path)",
                        mtg_player::llm::claude_code_binary(),
                        mtg_player::llm::CLAUDE_CODE_BINARY_ENV
                    ));
                }
                // Every seat calls at once — picks, deck builds, and a
                // tournament round's `players / 2` matches of two seats —
                // so the run's concurrency is its seat count, and the
                // Ctrl-C handler's registry has to be able to hold all of
                // it. A call that does not fit runs outside the handler and
                // is orphaned by an interrupt, which is what #538 cost at
                // the DEFAULT `--players 8` against a registry of 4. The
                // bound is checked here, before anything is spent, for the
                // same reason the binary is.
                if models.len() > mtg_player::llm::CLAUDE_CODE_MAX_LIVE_CALLS {
                    die(&format!(
                        "--players {} is more claude-code seats than can be taken down on Ctrl-C (limit {}); \
                         past that a seat's `claude -p` call would be orphaned by an interrupt",
                        models.len(),
                        mtg_player::llm::CLAUDE_CODE_MAX_LIVE_CALLS
                    ));
                }
                continue;
            }
            "gemini" => {}
            // Defaulting an unknown provider to claude spent real API money
            // drafting with a model nobody asked for, then printed standings
            // and exited 0 — indistinguishable from the requested run.
            other => die(&format!(
                "seat {i} model '{spec}': unknown provider '{other}' (expected {})",
                llm_client::ACCEPTED_PROVIDERS
            )),
        }
        let model = parts.get(1).copied().unwrap_or("gemini-2.5-flash");
        let levels: Vec<&str> = parts.iter().skip(2).copied().collect();

        for level in &levels {
            if !valid_levels.contains(level) {
                eprintln!("ERROR: Seat {} model '{}': '{}' is not a valid thinking level (valid: {})",
                    i, spec, level, valid_levels.join(", "));
                std::process::exit(1);
            }
            // Check model-specific restrictions
            for (model_prefix, allowed) in restricted {
                if model.contains(model_prefix) && !allowed.contains(level) {
                    eprintln!("ERROR: Seat {} model '{}': '{}' is not supported by {} (allowed: {})",
                        i, spec, level, model, allowed.join(", "));
                    std::process::exit(1);
                }
            }
        }
    }
}

// ─── Main ────────────────────────────────────────────────────────────

fn main() {
    install_panic_hook();
    let mut args = parse_args();
    validate_model_specs(&args.models);

    // A snapshot decides the seed, the set and the seat count: they are what
    // the recorded picks were made against, so a flag that disagreed would
    // replay them into a different draft (issue #218).
    let resumed: Option<DraftSave> = args.resume.as_ref().map(|path| {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| die(&format!("failed to read draft save '{path}': {e}")));
        let save: DraftSave = serde_json::from_str(&text)
            .unwrap_or_else(|e| die(&format!("draft save '{path}' is not a valid snapshot: {e}")));
        // Only a value the operator actually asked for can be overridden.
        // Where they asked for nothing, the note says where the value came
        // from rather than inventing an argument they never gave (#582).
        for (flag, saved, used) in [
            ("--seed", save.seed.to_string(), args.seed.to_string()),
            ("--set", save.set.clone(), args.set.clone()),
            ("--players", save.players.to_string(), args.players.to_string()),
        ] {
            if saved == used {
                continue;
            }
            if args.was_supplied(flag) {
                eprintln!("note: {flag} comes from the save ({used} -> {saved})");
            } else {
                eprintln!("note: {flag} {saved} comes from the save");
            }
        }
        save
    });
    if let Some(save) = &resumed {
        args.seed = save.seed;
        args.set.clone_from(&save.set);
        if save.players != args.players {
            // Rebuilt, not resized. Padding grew `models` with `models[0]`
            // and `guides` with `None`, so a global `--model` survived the
            // resume and a global `--guide` did not (#580).
            args.resolve_seats(save.players);
            validate_model_specs(&args.models);
        }
    }
    args.check_seat_flags();

    // What the replayed picks were made under, against what the rest of the
    // draft will be made under. `--seed`, `--set` and `--players` have been
    // reconciled since #218; the guide is the flag with the most effect on
    // what a seat does and had no check on it at all, so a resume under a
    // different guide silently produced a hybrid draft (issue #579).
    //
    // Not a refusal: swapping the guide mid-draft is a legitimate thing to
    // want to do. It has to be visible, which it was not — and the header
    // below carries it into the one artefact that outlives the terminal.
    let replayed_under: Vec<(usize, SeatPolicy, SeatPolicy)> = resumed
        .as_ref()
        .map(|save| {
            let now = args.seat_policies();
            save.seats
                .iter()
                .enumerate()
                .take(now.len())
                .filter(|(seat, was)| *was != &now[*seat])
                .map(|(seat, was)| (seat, was.clone(), now[seat].clone()))
                .collect()
        })
        .unwrap_or_default();
    for (seat, was, now) in &replayed_under {
        eprintln!("WARN: seat {seat}'s replayed picks were made under {}, and the rest of \
this draft will be made under {} — this draft is a mixture of the two",
            was.describe(), now.describe());
    }
    if let Some(save) = &resumed {
        if save.seats.is_empty() && !save.picks.is_empty() {
            eprintln!("note: this snapshot predates the guide/model record, so what its \
{} replayed picks were made under is unknown", save.picks.len());
        }
    }

    // One seeded root RNG, so the packs a run deals can be dealt again.
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(args.seed);

    // Load set data
    let set_path = PathBuf::from(format!("data/sets/{}.json", args.set));
    let mut set_data = SetData::load(&set_path).unwrap_or_else(|e| {
        eprintln!("Failed to load set data: {e}");
        std::process::exit(1);
    });

    let registry = CardRegistry::with_all_cards();
    let removed = set_data.filter_implemented(&registry);
    if !removed.is_empty() && !args.quiet {
        eprintln!(
            "Warning: {} cards not implemented, removed from draft pool",
            removed.len()
        );
    }

    let sheets = SheetData::from_set_data(&set_data).unwrap_or_else(|e| {
        eprintln!("Failed to build sheet data: {e}");
        std::process::exit(1);
    });

    // Create streaming log file
    let log = draft_log::DraftLogger::new(std::path::Path::new(&args.log));
    let resumed_from = resumed.as_ref().map(|save| {
        (args.resume.as_deref().unwrap_or_default(), save.picks.len())
    });
    let replayed_note: Vec<(usize, String, String)> = replayed_under
        .iter()
        .map(|(seat, was, now)| (*seat, was.describe(), now.describe()))
        .collect();
    log_header!(log, &set_data.set_name, args.players, args.best_of,
        args.models.as_slice(), args.guide_paths.as_slice(), args.seed, resumed_from,
        replayed_note.as_slice());

    if !args.quiet {
        eprintln!(
            "=== {} Draft: {} players, best-of-{} ===",
            set_data.set_name, args.players, args.best_of
        );
        eprintln!("Log file: {}", args.log);
    }

    // ── Phase 1: Generate packs ──
    if !args.quiet {
        eprintln!("Generating booster packs...");
    }
    let packs = generate_draft_packs(&sheets, args.players, &mut rng);

    // Log original pack contents
    log_section!(log, "BOOSTER PACKS");
    for (seat, player_packs) in packs.iter().enumerate() {
        for (pack_num, pack) in player_packs.iter().enumerate() {
            log_pack_contents!(log, seat, pack_num + 1, &pack.all_cards());
        }
    }

    // ── Phase 2: Draft ──
    log_section!(log, "DRAFT");
    if !args.quiet {
        eprintln!("Starting draft...");
    }
    let mut draft = DraftState::new(&packs);

    // Build card reference with oracle text for all cards in the set
    let card_reference = llm_client::build_card_reference(&set_data.all_card_names(), &registry);

    // Create LLM clients for each drafter (each may use a different model)
    // What a seat has to know about the table it is at, which used to be an
    // 8-pod description whatever the pod was (issue #485).
    let pack_size = packs.first().and_then(|seat_packs| seat_packs.first())
        .map_or(0, |pack| pack.all_cards().len());
    let table_for = |seat: usize| llm_client::Table {
        seat,
        pod_size: args.players,
        pack_size,
    };
    let mut clients: Vec<llm_client::DraftLlmClient> = (0..args.players)
        .map(|seat| {
            llm_client::DraftLlmClient::new(
                &args.models[seat],
                &set_data.set_name,
                args.guides[seat].as_deref(),
                &card_reference,
                table_for(seat),
            )
        })
        .collect();

    // Every card as a drafter sees it: cost, colour, type, size and rarity,
    // so a pack listing is something a pick can be made from (issue #483).
    let card_lines = card_lines::CardLines::new(
        &set_data.all_card_names(),
        &set_data.rarities(),
        &registry,
    );

    // Log every seat's system prompt, not seat 0's as a stand-in for the pod:
    // `--guide-N` and `--model-N` make them differ by construction, and a
    // draft has no seed to replay, so anything absent from the log is gone
    // (issue #207). Seats that match an earlier seat verbatim — the usual
    // case — are logged as a reference to it, so the common run's log is no
    // larger than before.
    for seat in 0..clients.len() {
        let prompt = clients[seat].system_prompt();
        let same_as = (0..seat).find(|&earlier| clients[earlier].system_prompt() == prompt);
        log_system_prompt!(log, seat, prompt, same_as);
    }

    // Picks the run had to make on a seat's behalf, per seat. Reported at
    // the end: a draft where a seat never made a choice must not present
    // its pools, decks and standings as if it had (issue #195).
    let mut substituted_picks = vec![0usize; args.players];

    // Every pick the run has made, in order: the snapshot, and on a resume
    // the picks replayed out of one.
    let mut recorded: Vec<PickRecord> = Vec::new();
    let replaying: Vec<PickRecord> = resumed.map(|s| s.picks).unwrap_or_default();
    if !replaying.is_empty() && !args.quiet {
        eprintln!("Replaying {} recorded pick(s) from the snapshot...", replaying.len());
    }
    // The cost summary counts this process's calls only, so it has to say
    // which part of the draft it is the cost of (issue #578).
    llm_client::note_replayed_picks(replaying.len());
    let write_snapshot = |picks: &[PickRecord]| {
        let Some(path) = &args.save else { return };
        let save = DraftSave {
            seed: args.seed,
            set: args.set.clone(),
            players: args.players,
            picks: picks.to_vec(),
            seats: args.seat_policies(),
        };
        // Write-then-rename: a snapshot half-written when the run dies is
        // worse than none, because it looks resumable.
        let tmp = format!("{path}.tmp");
        match serde_json::to_string(&save).map_err(|e| e.to_string())
            .and_then(|text| fs::write(&tmp, text).map_err(|e| e.to_string()))
            .and_then(|()| fs::rename(&tmp, path).map_err(|e| e.to_string()))
        {
            Ok(()) => {}
            Err(e) => eprintln!("\nWARN: could not write the draft snapshot to {path}: {e}"),
        }
    };

    // Run the draft — all players pick in parallel each round
    for round in 0..3 {
        if round > 0 {
            draft.start_next_pack_round();
        }

        let initial_cards = draft.cards_remaining(0);

        for pick_num in 0..initial_cards {
            // A pick the snapshot already holds is replayed, not re-asked:
            // it costs nothing and reproduces the position exactly.
            let from_save: Vec<&PickRecord> = replaying.iter()
                .filter(|p| p.round == round + 1 && p.pick == pick_num + 1)
                .collect();
            if pick_num == 0 {
                log_subsection!(log, &format!("Pack {}", round + 1));
            }
            if from_save.len() == args.players {
                // Replayed, not re-asked — but written down all the same. The
                // log is the run's record, and a resumed run's pools held
                // cards no line in it said anyone picked; and a pick the
                // runner made for a seat is still the runner's pick after a
                // resume (issue #401).
                for seat in 0..args.players {
                    let Some(rec) = from_save.iter().find(|p| p.seat == seat) else { continue };
                    let available = draft.current_pack_for(seat).len();
                    draft.make_pick(seat, &rec.card).unwrap_or_else(|e| {
                        die(&format!("draft save replays an impossible pick \
(seat {seat}, pack {}, pick {}, {}): {e}", round + 1, pick_num + 1, rec.card));
                    });
                    if rec.substituted {
                        substituted_picks[seat] += 1;
                    }
                    log_replayed_pick!(log, seat, round + 1, pick_num + 1, available, &rec.card, rec.substituted);
                    recorded.push((*rec).clone());
                }
                draft.rotate_packs();
                write_snapshot(&recorded);
                continue;
            }

            if !args.quiet {
                eprint!("\rPack {} Pick {}/{}", round + 1, pick_num + 1, initial_cards);
            }

            // Gather inputs for each player before spawning threads
            let pick_inputs: Vec<PickInput> =
                (0..args.players)
                    .map(|seat| {
                        (
                            seat,
                            draft.current_pack_for(seat).to_vec(),
                            draft.players[seat].pool.clone(),
                        )
                    })
                    .collect();

            // All players pick in parallel
            // The worker holds its own records and `main` writes them back in
            // seat order, the same way the deck-build and tournament scopes
            // do. Everything else in the pick phase is already written from
            // `main` after the join; the one record a worker emits itself is
            // the `SESSION` line (issue #542), and without this it landed in
            // whatever order the seats' first `claude -p` calls returned —
            // two runs of one seed writing the same lines in different order,
            // which is the defect #541 closed (issue #586).
            let pick_results: Vec<(usize, Pick, String, String)> =
                std::thread::scope(|s| {
                    let card_lines = &card_lines;
                    let handles: Vec<_> = pick_inputs
                        .iter()
                        .zip(clients.iter_mut())
                        .map(|((seat, available, pool), client)| {
                            let seat = *seat;
                            spawn_seat(s, format!("seat {seat}"), move || {
                                let context = format!(
                                    "seat {seat} could not make pack {} pick {}",
                                    round + 1,
                                    pick_num + 1
                                );
                                in_seat(&context, move || {
                                    mtg_player::game_log::buffer_here();
                                    let prompt = crate::llm_client::DraftLlmClient::build_pick_prompt(
                                        table_for(seat),
                                        round + 1,
                                        pick_num + 1,
                                        available,
                                        pool,
                                        card_lines,
                                    );
                                    let response =
                                        client.send_pick_message(&prompt, available.len());
                                    let chosen = parse_pick_response(&response, available);
                                    (seat, chosen, prompt, response,
                                        mtg_player::game_log::take_buffered())
                                })
                            })
                        })
                        .collect();

                    // `in_seat` has already reported and exited for any
                    // panic inside a worker's body, so this arm is a
                    // backstop for one raised outside it.
                    handles
                        .into_iter()
                        .enumerate()
                        .map(|(seat, h)| match h.join() {
                            Ok((s, chosen, prompt, response, records)) => {
                                mtg_player::game_log::write_block(&records);
                                (s, chosen, prompt, response)
                            }
                            Err(payload) => report_worker_failure(
                                &payload,
                                &format!("seat {seat}'s pick worker failed"),
                            ),
                        })
                        .collect()
                });

            // Apply picks sequentially (mutates draft state) and log
            for (seat, pick, prompt, response) in pick_results {
                let available = draft.current_pack_for(seat).to_vec();

                let substituted = pick.was_substituted();
                if substituted {
                    // A seat whose answers never parse is a failed seat, and
                    // the run has to be able to say so: without this, 42
                    // unusable answers read exactly like 42 deliberate picks
                    // (issue #195).
                    substituted_picks[seat] += 1;
                    eprintln!("\nWARN: seat {} pack {} pick {}: could not use the response, \
substituting {} (the first card). Response: {}",
                        seat, round + 1, pick_num + 1, pick.card(),
                        response.trim().replace('\n', " "));
                    log_draft_warning!(log, seat, round + 1, pick_num + 1, pick.card(), &response);
                }
                let chosen = pick.into_card();

                draft.make_pick(seat, &chosen).unwrap_or_else(|e| {
                    eprintln!("\nDraft pick error for seat {seat}: {e}");
                    let first = draft.current_pack_for(seat)[0].clone();
                    draft.make_pick(seat, &first).unwrap();
                });

                log_draft_pick!(log, seat, round + 1, pick_num + 1, &available, &chosen, &prompt, &response);
                recorded.push(PickRecord {
                    round: round + 1,
                    pick: pick_num + 1,
                    seat,
                    card: chosen,
                    substituted,
                });
            }

            draft.rotate_packs();
            // After the round, not during it: a snapshot is only resumable
            // at a pick boundary, where every seat has picked.
            write_snapshot(&recorded);
        }
    }

    if !args.quiet {
        eprintln!("\nDraft complete!");
    }

    // Log final pools
    log_section!(log, "DRAFT POOLS");
    for seat in 0..args.players {
        log_pool_summary!(log, seat, &draft.players[seat].pool);
    }

    // ── Phase 3: Deck Building ──
    log_section!(log, "DECK BUILDING");
    if !args.quiet {
        eprintln!("Building decks...");
    }

    // Build all decks in parallel. Each worker logs its own result as
    // soon as it finishes so progress shows up in the log in real time
    // rather than after the slowest worker blocks the batch.
    // game_log::write_at serializes writes via a global Mutex, so
    // concurrent writes from different workers are safe. Per-seat
    // entries may interleave in wall-clock order; each entry carries a
    // `[Seat N]` label so grep-by-seat still works.
    //
    // That interleave is the run's record disagreeing with the run: two
    // `--seed 41` runs replay the same packs, picks, decks and games and
    // then write logs that differ in a thousand places, because the order
    // is the scheduler's (issue #541). Each worker now holds its records
    // and they are written back in seat order, the way the pick loop above
    // has always done it. An `Error` record is not held — see
    // `game_log::buffer_here`.
    let pools: Vec<Vec<String>> = draft.players.iter().map(|p| p.pool.clone()).collect();

    let deck_results: Vec<DeckBuildResult> = std::thread::scope(|s| {
        let log_ref = &log;
        let registry_ref = &registry;
        let card_lines_ref = &card_lines;
        let handles: Vec<_> = clients
            .iter_mut()
            .zip(pools.iter())
            .enumerate()
            .map(|(seat, (client, pool))| spawn_seat(s, format!("seat {seat}"), move || {
                let context = format!("seat {seat} could not build its deck");
                in_seat(&context, move || {
                mtg_player::game_log::buffer_here();
                let result = build_deck_with_llm(client, pool, registry_ref, card_lines_ref);
                let attempts: Vec<(&str, &str, Option<&str>)> = result
                    .attempts
                    .iter()
                    .map(|a| (a.prompt.as_str(), a.response.as_str(), a.error.as_deref()))
                    .collect();
                log_deck_building!(log_ref,
                    seat,
                    &result.deck.maindeck,
                    &result.deck.lands,
                    &result.deck.sideboard,
                    &attempts,
                    result.retries,
                    result.fallback,
                );
                (result, mtg_player::game_log::take_buffered())
                })
            }))
            .collect();

        handles
            .into_iter()
            .enumerate()
            .map(|(seat, h)| match h.join() {
                Ok((result, records)) => {
                    mtg_player::game_log::write_block(&records);
                    result
                }
                Err(payload) => {
                    report_worker_failure(&payload, &format!("seat {seat}'s deck worker failed"))
                }
            })
            .collect()
    });

    // Build the decklist collection in seat order now that all workers
    // have finished. No further logging — that already happened above.
    let decklists: Vec<Decklist> = deck_results
        .iter()
        .map(|result| Decklist {
            entries: deckbuilding::to_decklist(&result.deck),
        })
        .collect();

    if !args.quiet {
        eprintln!("\nDecks built!");
    }

    // ── Phase 4: Tournament ──
    log_section!(log, "TOURNAMENT");
    if !args.quiet {
        eprintln!("Starting Swiss tournament...");
    }

    let tournament_config = TournamentConfig {
        best_of: args.best_of,
    };
    let mut tournament = Tournament::new(args.players, tournament_config);

    while !tournament.is_complete() {
        let round_num = tournament.rounds.len() + 1;
        let pairings = tournament.generate_pairings();

        if !args.quiet {
            eprintln!("Round {}/{}", round_num, tournament.total_rounds());
        }

        // Separate byes from real matches
        let real_matches: Vec<(usize, usize)> = pairings
            .iter()
            .filter(|&&(_, b)| b != BYE)
            .copied()
            .collect();

        for &(a, _) in pairings.iter().filter(|&&(_, b)| b == BYE) {
            if !args.quiet {
                eprintln!("  Seat {a} gets a bye");
            }
        }

        if !args.quiet {
            for &(a, b) in &real_matches {
                eprintln!("  Seat {a} vs Seat {b}");
            }
        }

        // Play all matches in the round in parallel
        let results: Vec<MatchResult> = std::thread::scope(|s| {
            let handles: Vec<_> = real_matches
                .iter()
                .map(|&(a, b)| {
                    let deck_a = &decklists[a];
                    let deck_b = &decklists[b];
                    let reg = &registry;
                    let model_a = &args.models[a];
                    let model_b = &args.models[b];
                    let guide_a = args.guides[a].as_deref();
                    let guide_b = args.guides[b].as_deref();
                    let card_ref = &card_reference;
                    let best_of = args.best_of;
                    let quiet = args.quiet;
                    // Computed here, on the main thread, from the match's own
                    // coordinates — not drawn inside the worker, where the
                    // draw order would be whatever the scheduler chose.
                    let seed = match_seed(args.seed, round_num, a, b);
                    spawn_seat(s, format!("seat {a} v {b}"), move || {
                        let context =
                            format!("the match between seat {a} and seat {b} could not finish");
                        in_seat(&context, move || {
                        // Held and written back in `real_matches` order, so
                        // two runs of one seed record the round the same
                        // way rather than as a scheduler-shuffled merge of
                        // the concurrent matches (issue #541).
                        mtg_player::game_log::buffer_here();
                        let outcome = play_match(
                            &PlayerSpec { seat: a, deck: deck_a, model_spec: model_a, guide: guide_a },
                            &PlayerSpec { seat: b, deck: deck_b, model_spec: model_b, guide: guide_b },
                            reg,
                            best_of,
                            quiet,
                            card_ref,
                            seed,
                        );
                        (outcome, mtg_player::game_log::take_buffered())
                        })
                    })
                })
                .collect();

            handles
                .into_iter()
                .zip(real_matches.iter())
                .map(|(h, (a, b))| match h.join() {
                    Ok((result, records)) => {
                        mtg_player::game_log::write_block(&records);
                        result
                    }
                    Err(payload) => report_worker_failure(
                        &payload,
                        &format!("the seat {a} v seat {b} match worker failed"),
                    ),
                })
                .collect()
        });

        // Log byes
        for &(a, b) in &pairings {
            if b == BYE {
                log_bye!(log, round_num, a);
            }
        }

        // Log match results and game logs
        for result in &results {
            log_match_result!(log, 
                round_num,
                result.player_a,
                result.player_b,
                result.wins_a,
                result.wins_b,
                result.winner(),
            );
            for (game_num, game) in result.games.iter().enumerate() {
                log_game_log!(log, 
                    round_num,
                    game_num + 1,
                    result.player_a,
                    result.player_b,
                    &game.game_log,
                );
            }

            if !args.quiet {
                // A forfeited game is a game nobody played; the score line
                // is where a reader is looking when it happens (#488).
                let forfeits = result.games.iter().filter(|g| g.stalled_seat.is_some()).count();
                let forfeited = match forfeits {
                    0 => String::new(),
                    1 => " [1 game forfeited: a seat stalled]".to_string(),
                    n => format!(" [{n} games forfeited: a seat stalled]"),
                };
                eprintln!(
                    "  Seat {} vs Seat {}: {}-{} (winner: Seat {}){forfeited}",
                    result.player_a,
                    result.player_b,
                    result.wins_a,
                    result.wins_b,
                    result.winner().map_or("draw".to_string(), |w| w.to_string())
                );
            }
        }

        tournament.record_round(pairings, results);
    }

    // ── Phase 5: Output ──
    log_section!(log, "FINAL STANDINGS");
    let sorted = tournament.sorted_standings();
    log_standings!(log, &sorted);

    if !args.quiet {
        eprintln!("\nFinal Standings:");
        for (rank, s) in sorted.iter().enumerate() {
            eprintln!("  {}", standings_row(rank + 1, s));
        }
    }

    // Count total games played
    let total_games: usize = tournament.rounds.iter()
        .flat_map(|r| r.results.iter())
        .map(|m| m.games.len())
        .sum();

    // Print token usage summary (draft client + game player combined)
    llm_client::print_usage_summary(llm_client::RunOutcome::Finished { total_games });

    // A run whose seats never picked must not look like one that did. This
    // is the last thing printed before "Done", next to the standings it
    // qualifies (issue #195).
    // Same rule for a seat whose deck the runner had to build: the
    // standings rank it, so the standings have to say it is not a built
    // deck (issue #200).
    let fallback_seats: Vec<usize> = deck_results
        .iter()
        .enumerate()
        .filter(|(_, r)| r.fallback)
        .map(|(seat, _)| seat)
        .collect();
    if !fallback_seats.is_empty() {
        eprintln!("\n=== Substituted Decks ===");
        for seat in &fallback_seats {
            eprintln!("    Seat {seat}: no valid deck after {} attempts — the runner built \
this seat's deck, so its results are not a built deck's", deck_results[*seat].retries);
        }
        eprintln!("  (grep the log for FALLBACK to see each one)");
    }

    // A game the watchdog forfeited is counted as a loss in the standings
    // and was never played out, so the run has to say so next to them —
    // the same rule as a substituted deck or pick (#195, #200, #488).
    let mut stalled_games = vec![0usize; args.players];
    for game in tournament.rounds.iter().flat_map(|r| r.results.iter()).flat_map(|m| m.games.iter()) {
        if let Some(seat) = game.stalled_seat {
            stalled_games[seat] += 1;
        }
    }
    if stalled_games.iter().any(|n| *n > 0) {
        eprintln!("\n=== Forfeited Games ===");
        for (seat, n) in stalled_games.iter().enumerate() {
            if *n > 0 {
                eprintln!("    Seat {seat}: {n} game(s) forfeited — this seat stopped making \
progress (the same unusable answer over and over), so the game was awarded to its opponent");
            }
        }
        eprintln!("  (grep the log for STALLED to see each one)");
    }

    let substituted_total: usize = substituted_picks.iter().sum();
    if substituted_total > 0 {
        eprintln!("\n=== Substituted Picks ===");
        eprintln!("  {substituted_total} pick(s) were made by the runner, not by a seat:");
        for (seat, n) in substituted_picks.iter().enumerate() {
            if *n > 0 {
                eprintln!("    Seat {seat}: {n} pick(s) unusable — this seat's pool, \
deck and results are not a drafted one");
            }
        }
        eprintln!("  (grep the log for WARN to see each one)");
    }

    if !args.quiet {
        eprintln!("\nDone. Log written to {}", args.log);
    }
}

// ─── Draft Pick Parsing ──────────────────────────────────────────────

/// What a seat's answer amounted to: the card it picked, and whether that
/// card was actually chosen or substituted because the answer was unusable.
///
/// The substitution itself is deliberate — a draft has to continue — but it
/// used to be silent, so 42 unparsable answers produced 42 confident
/// "Chose:" lines and a tournament built on them (issue #195). The adjacent
/// backend code already treats this class of failure as loud; this carries
/// the same fact out of the parser so the caller can too.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Pick {
    /// The seat named this card.
    Chosen(String),
    /// The answer could not be used; this is the first card of the pack.
    Substituted(String),
}

impl Pick {
    fn card(&self) -> &str {
        match self {
            Pick::Chosen(c) | Pick::Substituted(c) => c,
        }
    }

    fn into_card(self) -> String {
        match self {
            Pick::Chosen(c) | Pick::Substituted(c) => c,
        }
    }

    fn was_substituted(&self) -> bool {
        matches!(self, Pick::Substituted(_))
    }
}

fn parse_pick_response(response: &str, available: &[String]) -> Pick {
    // Primary path: JSON response like `{"thoughts": "...", "pick": N}`.
    // Secondary path (legacy or stray wrappers): strip markdown code fences
    // and retry. Last resort: fall through to a text scan for "PICK: N".
    let try_json = |s: &str| -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(s).ok()?;
        let idx = usize::try_from(v["pick"].as_u64()?).unwrap_or(usize::MAX);
        (idx < available.len()).then(|| available[idx].clone())
    };

    if let Some(pick) = try_json(response) {
        return Pick::Chosen(pick);
    }

    // Strip optional ```json ... ``` fencing that some models still add.
    let stripped = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Some(pick) = try_json(stripped) {
        return Pick::Chosen(pick);
    }

    // Legacy text scan — kept for robustness against older responses.
    for line in response.lines().rev() {
        let trimmed = line.trim().to_uppercase();
        if let Some(rest) = trimmed.strip_prefix("PICK:") {
            if let Ok(idx) = rest.trim().trim_start_matches('"').trim_end_matches('"').trim_end_matches(',').parse::<usize>() {
                if idx < available.len() {
                    return Pick::Chosen(available[idx].clone());
                }
            }
        }
    }

    // Last resort: the draft must continue, so take the first card — but say
    // so, rather than letting it pass for a decision.
    Pick::Substituted(available[0].clone())
}

// ─── Deck Building ───────────────────────────────────────────────────

fn build_deck_with_llm(
    client: &mut llm_client::DraftLlmClient,
    pool: &[String],
    registry: &CardRegistry,
    cards: &card_lines::CardLines,
) -> DeckBuildResult {
    let prompt = build_deck_prompt(pool, cards);
    let mut last_error = String::new();
    let mut attempts: Vec<DeckAttempt> = Vec::new();
    let max_retries = 10;

    for attempt in 0..max_retries {
        if attempt > 0 {
            // Brief delay before retry (helps with transient network errors)
            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        let msg = if attempt == 0 {
            prompt.clone()
        } else {
            format!(
                "Your previous deck was invalid: {last_error}. Please try again.\n\n{prompt}"
            )
        };

        let response = client.send_deck_building_message(&msg, pool);

        match deckbuilding::parse_deck_response(&response) {
            Ok((maindeck, lands)) => match deckbuilding::validate_deck(pool, &maindeck, &lands) {
                Ok(deck) => {
                    attempts.push(DeckAttempt { prompt: msg, response, error: None });
                    let retries = attempts.len() - 1;
                    return DeckBuildResult { deck, attempts, retries, fallback: false };
                }
                Err(e) => {
                    attempts.push(DeckAttempt { prompt: msg, response, error: Some(e.clone()) });
                    last_error = e;
                }
            },
            Err(e) => {
                attempts.push(DeckAttempt { prompt: msg, response, error: Some(e.clone()) });
                last_error = e;
            }
        }
    }

    // No attempt produced a valid deck. The draft has already been played,
    // so the round still has to happen — but the deck it happens with is the
    // runner's, not the seat's, and everything downstream is told so.
    eprintln!("Warning: deck building failed after {max_retries} attempts, using fallback");
    let retries = attempts.len();
    DeckBuildResult {
        deck: deckbuilding::fallback_deck(pool, registry),
        attempts,
        retries,
        fallback: true,
    }
}

/// The one message that decides a seat's whole deck.
///
/// It used to be a sentence and a list of names and counts: no colour, no
/// cost, no type — the two things a limited deck is built on — no land
/// target, no statement of the answer's shape, and no mention of the
/// sideboard it was silently creating. The only land guidance a seat ever
/// got was a `description` string inside the JSON schema (issue #487).
fn build_deck_prompt(pool: &[String], cards: &card_lines::CardLines) -> String {
    let mut prompt = format!(
        "Draft complete! Build your deck out of the {} cards you drafted.\n\n\
         Your pool ({} cards):\n",
        pool.len(),
        pool.len(),
    );
    prompt.push_str(&cards.pool_listing(pool));
    prompt.push_str(
        "\n## Building it\n\
         - A deck is at least 40 cards (CR 100.2b); a smaller one is rejected and you are asked again\n\
         - The usual limited build is 17 basic lands and 23 spells from the pool\n\
         - Two colors is the normal build, a third only as a splash you can reliably cast\n\
         - Everything you leave out is your sideboard. It is recorded with your deck, but nothing is sideboarded between games of a match, so a card you leave out is a card you will not play\n\
         \n## Your answer\n\
         - `maindeck` maps each drafted card you are playing to how many copies (0, or leave it out, to cut it)\n\
         - `lands` maps each basic land to how many to add. Basic lands are not drafted and are not limited: they go here, and only here, even if you drafted one\n",
    );
    prompt
}

// ─── Tournament Game Execution ───────────────────────────────────────

/// Whether a match of `best_of` games is decided, given what has been played.
///
/// Two ways a match ends, and the loop used to know only the first (#484):
/// somebody has won more than half the games, or `best_of` games have been
/// played. A drawn game wins nothing but is still a game played — MTR 6.5
/// ends a best-of-three after three games however they went — so without the
/// second clause a match with draws in it has no bound on its length, and an
/// even `--best-of` plays one game more than it says.
fn match_is_over(best_of: usize, games_played: usize, wins_a: usize, wins_b: usize) -> bool {
    let needed = tournament::wins_needed(best_of);
    wins_a >= needed || wins_b >= needed || games_played >= best_of
}

fn play_match(
    a: &PlayerSpec<'_>,
    b: &PlayerSpec<'_>,
    registry: &CardRegistry,
    best_of: usize,
    _quiet: bool,
    card_reference: &str,
    seed: u64,
) -> MatchResult {
    let mut wins_a = 0;
    let mut wins_b = 0;
    let mut games = Vec::new();

    let seat_a = a.seat;
    let seat_b = b.seat;
    let deck_a = a.deck;
    let deck_b = b.deck;

    // Create LLM players once per match, reuse across games.
    // Set log file so all API prompts/responses are written to the draft log.
    let name_a = format!("Seat{seat_a}");
    let name_b = format!("Seat{seat_b}");
    let mut p1 = make_game_player(a.model_spec, &name_a, a.guide);
    let mut p2 = make_game_player(b.model_spec, &name_b, b.guide);

    // Play/draw per MTG tournament rules, delegated to the engine helpers:
    //   Game 1: a fair coin flip.
    //   Games 2+: engine::next_starter_loser_plays() — the loser of the
    //   previous game always elects to play first (the strategically
    //   dominant choice in Limited); drawn games keep the previous starter.
    //
    // The flip and each game's engine seed come off this match's own RNG
    // rather than the thread's, so a seeded run replays its games and not
    // only its packs (issue #212).
    let mut match_rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(seed);
    let mut starter = mtg_engine::ids::PlayerId(
        if rand::Rng::gen_bool(&mut match_rng, 0.5) { 1 } else { 0 });

    while !match_is_over(best_of, games.len(), wins_a, wins_b) {
        let outcome = play_game(
            seat_a,
            seat_b,
            deck_a,
            deck_b,
            registry,
            &mut p1,
            &mut p2,
            starter,
            card_reference,
            best_of,
            rand::Rng::gen(&mut match_rng),
        );

        // Engine's winner is a PlayerId (0 = seat_a, 1 = seat_b).
        let prev_winner: Option<mtg_engine::ids::PlayerId> = outcome.winner.map(|w| {
            if w == seat_a { mtg_engine::ids::PlayerId(0) } else { mtg_engine::ids::PlayerId(1) }
        });
        starter = engine::next_starter_loser_plays(starter, prev_winner, 2);

        match outcome.winner {
            Some(w) if w == seat_a => wins_a += 1,
            Some(_) => wins_b += 1,
            None => {}
        }

        games.push(outcome);
    }

    MatchResult {
        player_a: seat_a,
        player_b: seat_b,
        wins_a,
        wins_b,
        games,
    }
}

fn play_game(
    seat_a: usize,
    seat_b: usize,
    deck_a: &Decklist,
    deck_b: &Decklist,
    registry: &CardRegistry,
    p1: &mut LlmPlayer,
    p2: &mut LlmPlayer,
    starting_player: mtg_engine::ids::PlayerId,
    card_reference: &str,
    best_of: usize,
    rng_seed: u64,
) -> GameOutcome {
    let config = GameConfig {
        player_names: vec![p1.name().to_string(), p2.name().to_string()],
        decklists: vec![deck_a.clone(), deck_b.clone()],
        starting_life: 20,
        starting_player: Some(starting_player),
        // Derived from the match's seed, so the shuffles replay (issue #212).
        rng_seed: Some(rng_seed),
    };

    let mut state = engine::setup_game(&config, registry);

    // Re-initialize conversations for this game (fresh context per game)
    let format = MatchFormat::BestOf(best_of);
    p1.init_conversation(&deck_a.entries, card_reference, registry, format);
    p2.init_conversation(&deck_b.entries, card_reference, registry, format);

    let mut action_count: u64 = 0;
    let max_actions: u64 = 50_000;

    // The progress watchdog `mtg-runner` has had since #462. This loop is
    // the other copy, and it had only the 50,000-action cap — which for a
    // pod of `cc` seats is 50,000 `claude -p` subprocesses spent re-asking
    // one question, and then a silent concede. A stalled game here forfeits
    // for the seat that is stuck and says so everywhere the game is
    // reported, rather than killing the tournament around it (#488).
    let mut watchdog = mtg_player::watchdog::ProgressWatchdog::new();
    let mut stalled_seat: Option<usize> = None;

    let mut game_callback =
        |game_state: &GameState,
         acting_player: PlayerId,
         legal: &engine::LegalActions|
         -> mtg_engine::actions::Action {
            action_count += 1;

            let stalled = watchdog.observe(game_state);
            if stalled && stalled_seat.is_none() {
                let seat = if acting_player == PlayerId(0) { seat_a } else { seat_b };
                stalled_seat = Some(seat);
                let report = mtg_player::watchdog::stall_report(
                    game_state, acting_player, legal, &seat.to_string(),
                );
                eprintln!("\nWARN: {report} The game is forfeit to seat {}.",
                    if acting_player == PlayerId(0) { seat_b } else { seat_a });
                draft_log::DraftLogger::stalled_game(
                    seat_a, seat_b, seat,
                    game_state.turn_number,
                    &format!("{:?}", game_state.step),
                    &report,
                    file!(), line!(),
                );
            }

            if stalled || action_count >= max_actions {
                // Sent, not looked up. `legal.actions` lists `Concede` only
                // on the normal-priority path, so reaching for it there made
                // both the forfeit and the 50,000-action ceiling no-ops at
                // every prompt — a mulligan, a discard, a declaration, any
                // resolution choice — which is where a spinning seat usually
                // is, leaving the game with no termination condition at all
                // (issue #559). The engine accepts a concede at any decision
                // point (CR 104.3a, `LegalActions::permits`).
                return mtg_player::watchdog::forfeit_move();
            }

            let view = GameView::for_player(game_state, acting_player, registry);

            let player: &mut LlmPlayer = if acting_player == PlayerId(0) {
                p1
            } else {
                p2
            };

            if let Some(prompt) = &legal.combat_prompt {
                return player.choose_combat(&view, prompt);
            }

            player.choose_action(&view, legal)
        };

    engine::run_game_loop(&mut state, registry, &mut game_callback);

    let winner = state.result.as_ref().and_then(|r| {
        match r {
            mtg_engine::state::GameResult::Winner(pid) => {
                if *pid == PlayerId(0) {
                    Some(seat_a)
                } else {
                    Some(seat_b)
                }
            }
            mtg_engine::state::GameResult::Draw => None,
        }
    });

    // Capture game log, filtering out Debug-level entries (priority passes etc.)
    // to keep the log readable
    let game_log: Vec<String> = state
        .game_log
        .iter()
        .filter(|entry| entry.level as u8 >= 1) // Info and above
        .map(|entry| entry.message.clone())
        .collect();

    GameOutcome {
        winner,
        turns: state.turn_number,
        game_log,
        stalled_seat,
    }
}

fn make_game_player(model_spec: &str, name: &str, guide: Option<&str>) -> LlmPlayer {
    // Parse "provider:model:draft_thinking:game_thinking"
    let parts: Vec<&str> = model_spec.split(':').collect();
    let provider = parts[0];
    let model = parts.get(1).copied();
    // Game thinking is the 4th part, or falls back to 3rd, or defaults
    let game_thinking = parts.get(3).or(parts.get(2)).copied();

    let mut p = match provider {
        "gemini" => {
            let mut p = LlmPlayer::new_gemini(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "claude" => {
            let mut p = LlmPlayer::new(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        "claude-code" | "cc" => {
            let mut p = LlmPlayer::new_claude_code(name);
            if let Some(m) = model {
                p = p.with_model(m);
            }
            p
        }
        // Unreachable once validate_model_specs has run, and fatal if it ever
        // is reached: substituting a seat plays a different game than the one
        // requested and still prints a winner.
        other => die(&format!(
            "unknown model provider '{other}' (expected {})",
            llm_client::ACCEPTED_PROVIDERS
        )),
    };
    if let Some(level) = game_thinking {
        p = p.with_thinking_level(level);
    }
    if let Some(g) = guide {
        p = p.with_guide(g.to_string());
    }
    p
}

#[cfg(test)]
mod deck_prompt_tests {
    use super::{build_deck_prompt, card_lines::CardLines, CardRegistry};

    /// #487: the whole prompt used to be one sentence and a `Nx Name` list —
    /// no colour or cost to build on, no land target, no statement of the
    /// answer's shape, and no mention of the sideboard it creates.
    #[test]
    fn the_deck_prompt_says_what_it_is_asking_for() {
        let set_data = mtg_draft::set_data::SetData::load(std::path::Path::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../data/sets/isd.json"
        )))
        .expect("ISD set data");
        let registry = CardRegistry::with_all_cards();
        let cards = CardLines::new(&set_data.all_card_names(), &set_data.rarities(), &registry);

        let pool = vec![
            "Moon Heron".to_string(),
            "Moon Heron".to_string(),
            "Chapel Geist".to_string(),
            "Plains".to_string(),
        ];
        let prompt = build_deck_prompt(&pool, &cards);

        // The pool, with what each card costs and is.
        assert!(prompt.contains("2x Moon Heron {3}{U} | Creature — Spirit Bird 3/2"), "{prompt}");
        assert!(prompt.contains("Colors"), "{prompt}");
        assert!(prompt.contains("Curve"), "{prompt}");
        // The deck it is asking for.
        assert!(prompt.contains("40 cards"), "{prompt}");
        assert!(prompt.contains("17 basic lands and 23 spells"), "{prompt}");
        // The answer's shape, and where a drafted basic land goes.
        assert!(prompt.contains("`maindeck`"), "{prompt}");
        assert!(prompt.contains("`lands`"), "{prompt}");
        assert!(prompt.contains("even if you drafted one"), "{prompt}");
        // The sideboard it is silently creating.
        assert!(prompt.contains("sideboard"), "{prompt}");
    }
}

#[cfg(test)]
mod match_length_tests {
    use super::match_is_over;

    /// #484: `--best-of 2` played three games, because only a win target
    /// ended the match and two wins are needed to take a two-game match.
    #[test]
    fn an_even_best_of_stops_at_the_games_it_names() {
        assert!(!match_is_over(2, 0, 0, 0));
        assert!(!match_is_over(2, 1, 1, 0));
        // 1-1 after both games: the match is over and drawn, not extended.
        assert!(match_is_over(2, 2, 1, 1));
        // Winning both still ends it at two.
        assert!(match_is_over(2, 2, 2, 0));
    }

    /// A drawn game wins nothing but is still a game played, so a match with
    /// draws in it is bounded (MTR 6.5) instead of running forever.
    #[test]
    fn drawn_games_still_count_toward_the_match_length() {
        // best-of-three, two draws and a win: 1-0 with three games played.
        assert!(!match_is_over(3, 1, 0, 0));
        assert!(!match_is_over(3, 2, 0, 0));
        assert!(match_is_over(3, 3, 1, 0));
        // And a best-of-three that draws every game ends drawn.
        assert!(match_is_over(3, 3, 0, 0));
    }

    #[test]
    fn a_decided_match_does_not_play_its_dead_game() {
        assert!(match_is_over(3, 2, 2, 0));
        assert!(!match_is_over(3, 2, 1, 1));
        assert!(match_is_over(1, 1, 1, 0));
    }
}

#[cfg(test)]
mod standings_row_tests {
    use super::{standings_row, Standing};

    fn standing(seat: usize, match_wins: usize, match_losses: usize, game_wins: usize, byes: usize) -> Standing {
        Standing {
            seat,
            match_wins,
            match_losses,
            match_draws: 0,
            game_wins,
            game_losses: 0,
            byes,
        }
    }

    /// #486: seat 0 went 1-1 in matches it played; seat 1 lost its only match
    /// and was given a bye. Both are "1-1" in the counters, so the row has to
    /// say which win was awarded — otherwise the seat that lost to seat 0
    /// prints identically to seat 0.
    #[test]
    fn a_bye_is_not_printed_as_a_won_match() {
        let played = standings_row(2, &standing(0, 1, 1, 1, 0));
        let byed = standings_row(3, &standing(1, 1, 1, 1, 1));

        assert_eq!(played, "2. Seat 0 — 1-1 (1 game wins)");
        assert_eq!(byed, "3. Seat 1 — 1-1 (1 game wins) [1 bye]");
    }

    #[test]
    fn several_byes_and_draws_are_both_reported() {
        let mut s = standing(4, 2, 1, 2, 2);
        s.match_draws = 1;
        assert_eq!(standings_row(1, &s), "1. Seat 4 — 2-1-1 (2 game wins) [2 byes]");
    }
}

#[cfg(test)]
mod pick_parsing_tests {
    use super::{parse_pick_response, Pick};

    fn pack() -> Vec<String> {
        ["Hysterical Blindness", "Voiceless Spirit", "Ambush Viper", "Delver of Secrets"]
            .iter().map(std::string::ToString::to_string).collect()
    }

    #[test]
    fn a_usable_answer_is_the_seats_own_pick() {
        let p = pack();
        assert_eq!(parse_pick_response(r#"{"pick": 2}"#, &p),
            Pick::Chosen("Ambush Viper".into()));
        assert_eq!(parse_pick_response("```json\n{\"pick\": 1}\n```", &p),
            Pick::Chosen("Voiceless Spirit".into()));
        assert_eq!(parse_pick_response("thinking...\nPICK: 3", &p),
            Pick::Chosen("Delver of Secrets".into()));
    }

    /// The four shapes from issue #195: each is a well-formed JSON object
    /// that never reaches the backend's loud "no structured object" path,
    /// so the parser is the only place that can notice. Each still yields a
    /// card — a draft has to continue — but it must be marked as the
    /// runner's substitution, not the seat's choice.
    #[test]
    fn an_unusable_answer_is_reported_as_a_substitution() {
        let p = pack();
        for response in [
            r#"{"pick": 9999}"#,            // out-of-range index
            r#"{"choice": 3}"#,             // right shape, wrong key
            "{}",                           // empty object
            r#"{"pick": "Ambush Viper"}"#,  // a name where an index goes
        ] {
            let got = parse_pick_response(response, &p);
            assert_eq!(got, Pick::Substituted("Hysterical Blindness".into()),
                "{response} is not a usable pick, so it must not pass for one");
            assert!(got.was_substituted(),
                "{response} must be reportable as a substitution");
            // The draft still gets a card to continue with.
            assert_eq!(got.card(), "Hysterical Blindness");
        }
    }

    /// A snapshot written before picks recorded who made them still loads,
    /// and reads as a seat's picks — the most a replay can say about it.
    #[test]
    fn a_snapshot_without_the_substituted_field_still_loads() {
        let save: super::DraftSave = serde_json::from_str(
            r#"{"seed":1,"set":"isd","players":2,
                "picks":[{"round":1,"pick":1,"seat":0,"card":"Silverchase Fox"}]}"#,
        ).expect("an older snapshot is still a snapshot");
        assert_eq!(save.picks.len(), 1);
        assert!(!save.picks[0].substituted);
    }
}
