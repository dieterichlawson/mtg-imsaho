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
mod progress;
use progress::{draw_progress, end_progress_line};

/// One row of the final standings, written once and printed by both surfaces
/// that show them — stderr and the log's FINAL STANDINGS block.
///
/// The row carries the seat's full match record and marks the wins that were
/// byes rather than matches played. Without the marker a seat that sat out a
/// round reads exactly like a seat that beat somebody, and the block does not
/// reconcile against the matches above it (issue #486, the shape of #195 and
/// #200).
pub(crate) fn standings_row(rank: usize, s: &Standing, tags: &RowTags) -> String {
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
        "{}. Seat {} — {}-{}{draws} ({} game wins){byes}{}",
        rank, s.seat, s.match_wins, s.match_losses, s.game_wins, tags.render(),
    )
}

/// What a standings row says about a result that is not wholly the seat's
/// own, in the same `[...]` form as a bye (#486). The sections after the
/// standings say which and why; the row is where a reader is looking when
/// it ranks the seat (issue #588).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct RowTags {
    /// Answers the harness could not use and chose for the seat.
    pub answers_substituted: u64,
    /// Decisions the seat's backend never answered at all (#587).
    pub never_answered: u64,
    /// The runner built this seat's deck (#200).
    pub runner_built_deck: bool,
    /// Games the watchdog forfeited for this seat (#488).
    pub games_forfeited: usize,
    /// Matches carried over from a snapshot rather than played by this
    /// process (#581).
    pub matches_from_snapshot: usize,
}

impl RowTags {
    fn render(&self) -> String {
        let plural = |n: u64, one: &str, many: &str| if n == 1 { one.to_string() } else { many.to_string() };
        let mut out = String::new();
        if self.answers_substituted > 0 {
            let n = self.answers_substituted;
            out.push_str(&format!(" [{n} {} substituted]", plural(n, "answer", "answers")));
        }
        if self.never_answered > 0 {
            let n = self.never_answered;
            out.push_str(&format!(" [{n} {} never answered]", plural(n, "decision", "decisions")));
        }
        if self.runner_built_deck {
            out.push_str(" [runner-built deck]");
        }
        if self.games_forfeited > 0 {
            let n = self.games_forfeited as u64;
            out.push_str(&format!(" [{n} {} forfeited]", plural(n, "game", "games")));
        }
        if self.matches_from_snapshot > 0 {
            let n = self.matches_from_snapshot as u64;
            out.push_str(&format!(" [{n} {} from snapshot]", plural(n, "match", "matches")));
        }
        out
    }
}

/// A match result as the snapshot keeps it: the games' logs are the run's
/// log's, and a snapshot written after every match has to stay small.
fn without_game_logs(mut result: MatchResult) -> MatchResult {
    for game in &mut result.games {
        game.game_log.clear();
    }
    result
}

/// What the score line adds about the match's games that were not played
/// out: a forfeit is a seat the watchdog caught (#488), an abandoned game is
/// one the runner stopped at its action budget with no winner (#630).
fn unplayed_games_note(games: &[GameOutcome]) -> String {
    let count = |n: usize, what: &str| match n {
        0 => String::new(),
        1 => format!(" [1 game {what}]"),
        n => format!(" [{n} games {what}]"),
    };
    let forfeits = games.iter().filter(|g| g.stalled_seat.is_some()).count();
    let abandoned = games.iter().filter(|g| g.abandoned).count();
    format!(
        "{}{}",
        count(forfeits, "forfeited: a seat stalled"),
        count(abandoned, "abandoned: the action budget ran out, no winner"),
    )
}

/// The per-match progress line on stderr. A forfeited game is a game nobody
/// played; the score line is where a reader is looking when it happens
/// (#488). A level match has no winner to name (#650).
fn match_score_line(result: &MatchResult) -> String {
    let outcome = match result.winner() {
        Some(w) => format!("winner: Seat {w}"),
        None => "drawn".to_string(),
    };
    format!(
        "  Seat {} vs Seat {}: {}-{} ({outcome}){}",
        result.player_a,
        result.player_b,
        result.wins_a,
        result.wins_b,
        unplayed_games_note(&result.games),
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
  --save <path>          Snapshot the run here: after every pick round, after
                         deck building, and after every tournament match. An
                         interruption costs the round, the build or the match
                         in progress, and --resume carries on from there
  --resume <path>        Replay a snapshot and carry on from it. Its seed, set
                         and seat count win over the flags — the packs are
                         re-dealt from the seed, so the position is exact.
                         Its decks are used as built, and its finished matches
                         are counted, not replayed: the standings mark them
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
    // `process::exit` runs no destructors and raises no signal, so nothing
    // else takes this run's in-flight `claude -p` subprocesses down with
    // it. Every other seat is mid-call when one seat fatals — all seats
    // pick in parallel and the joins are walked in seat order — and each
    // one kept its whole process tree, orphaned to init and still spending
    // against a draft that had stopped (issue #537). Ctrl-C has swept them
    // since #206; the fatal path now sweeps the same registry.
    //
    // First, before any I/O: the runtime ignores SIGPIPE, so a write to a
    // closed stderr panics instead of killing the process. When `die`'s own
    // `eprintln!` came first and panicked, the sweep and the exit were
    // never reached, and every other thread had already parked for good in
    // `report_worker_failure` — a run wedged at 0% CPU forever (#652).
    mtg_player::llm::claude_code_kill_live_calls();
    // Everything else is a report, and a report that cannot be written must
    // not stop the exit.
    let _ = std::panic::catch_unwind(|| {
        use std::io::Write;
        let _ = writeln!(std::io::stderr(), "{}Error: {msg}", end_progress_line());
        // A run that stopped still spent what it spent. The usage summary
        // was printed only at the end of the happy path, so a seat's fatal,
        // a worker panic or a config error published no account of the
        // `claude -p` calls already paid for, and the resume that finished
        // reported a fragment labelled like a whole run (issue #578).
        llm_client::print_usage_summary(llm_client::RunOutcome::Stopped);
    });
    // The summary's own log record is owed to the file now, for the same
    // reason the caller flushed before getting here.
    // Every worker's held records, not only this thread's: the process
    // will not come back for any of them (#658).
    let _ = std::panic::catch_unwind(mtg_player::game_log::flush_all);
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
    // Whatever the workers were holding back for the deterministic flush is
    // owed to the log now — this one's and every other match's, in their
    // order: `process::exit` will not come back for any of it (#658).
    mtg_player::game_log::flush_all();
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

/// Whether a snapshot's picks are whole pick steps in draft order: one
/// record per seat at every step from pack 1 pick 1 up to where it stops,
/// and nothing after that.
///
/// The runner only ever writes a snapshot at a pick boundary, through a
/// rename, so anything else is a hand edit or corruption. The replay used to
/// count records rather than seats: two records for one seat replayed both
/// into it and the run exited 0 with pools of 42 and 41 (#655), and a step
/// missing one seat was re-asked live for every seat, dropping the other
/// seats' records in silence while the summary still counted them as
/// replayed (#656). A save that would draw a different draft from the one
/// it records is refused, naming the record, the way an impossible card is.
fn check_snapshot_shape(picks: &[PickRecord], players: usize, pack_size: usize) -> Result<(), String> {
    const PACKS: usize = 3;
    let mut seen = vec![vec![false; players]; PACKS * pack_size];
    for r in picks {
        if !(1..=PACKS).contains(&r.round) || !(1..=pack_size).contains(&r.pick) {
            return Err(format!(
                "it has a record for pack {} pick {}, and the draft has {PACKS} packs of {pack_size} picks",
                r.round, r.pick
            ));
        }
        if r.seat >= players {
            return Err(format!(
                "it has a record for seat {} at pack {} pick {}, and the draft has {players} seats",
                r.seat, r.round, r.pick
            ));
        }
        let slot = &mut seen[(r.round - 1) * pack_size + r.pick - 1][r.seat];
        if *slot {
            return Err(format!("it has two records for seat {} at pack {} pick {}", r.seat, r.round, r.pick));
        }
        *slot = true;
    }
    let whole = seen.iter().take_while(|step| step.iter().all(|&s| s)).count();
    let Some(last) = seen.iter().rposition(|step| step.iter().any(|&s| s)) else { return Ok(()) };
    if last < whole {
        return Ok(());
    }
    let (round, pick) = (whole / pack_size + 1, whole % pack_size + 1);
    let seats = |want: bool| -> Vec<String> {
        (0..players).filter(|&s| seen[whole][s] == want).map(|s| s.to_string()).collect()
    };
    if seats(true).is_empty() {
        Err(format!("it has no records for pack {round} pick {pick} but has records after it"))
    } else {
        Err(format!(
            "at pack {round} pick {pick} it has records for seat(s) {} and none for seat(s) {}",
            seats(true).join(", "),
            seats(false).join(", ")
        ))
    }
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
    /// Each seat's built deck, in seat order, once deck building is done —
    /// empty until then (issue #581). The build attempts are not kept: they
    /// are in the log, and a snapshot written after every match has to stay
    /// cheap to write.
    #[serde(default)]
    decks: Vec<SavedDeck>,
    /// Every match the tournament has finished, in the order they finished
    /// (issue #581). A resumed run takes these instead of playing them again,
    /// and says so on the standings.
    #[serde(default)]
    matches: Vec<SavedMatch>,
}

/// A seat's deck as the snapshot keeps it.
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct SavedDeck {
    deck: deckbuilding::DraftDeck,
    /// The runner built it: no attempt produced a valid deck (#200).
    fallback: bool,
    /// How many attempts failed before the deck was settled.
    retries: usize,
}

/// A finished match as the snapshot keeps it: the round it was played in,
/// and its result without the games' logs (those are in the log of the run
/// that played them).
#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct SavedMatch {
    round: usize,
    result: MatchResult,
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
                mtg_player::stderr_line!("Error: {a} requires a value\n\n{USAGE}");
                std::process::exit(2);
            }
            if let Some(index) = per_seat {
                indexed.push((a.to_string(), index));
            }
            i += 2;
        } else if BOOL_FLAGS.contains(&a) {
            i += 1;
        } else {
            mtg_player::stderr_line!("Error: unrecognized argument '{a}'\n\n{USAGE}");
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
        mtg_player::stdout_line!("{USAGE}");
        std::process::exit(0);
    }
    if args.iter().any(|a| a == "--version") {
        mtg_player::stdout_line!("mtg-draft-runner {}", env!("CARGO_PKG_VERSION"));
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
                mtg_player::stderr_line!("ERROR: Seat {} model '{}': '{}' is not a valid thinking level (valid: {})",
                    i, spec, level, valid_levels.join(", "));
                std::process::exit(1);
            }
            // Check model-specific restrictions
            for (model_prefix, allowed) in restricted {
                if model.contains(model_prefix) && !allowed.contains(level) {
                    mtg_player::stderr_line!("ERROR: Seat {} model '{}': '{}' is not supported by {} (allowed: {})",
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
                mtg_player::stderr_line!("note: {flag} comes from the save ({used} -> {saved})");
            } else {
                mtg_player::stderr_line!("note: {flag} {saved} comes from the save");
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
        mtg_player::stderr_line!("WARN: seat {seat}'s replayed picks were made under {}, and the rest of \
this draft will be made under {} — this draft is a mixture of the two",
            was.describe(), now.describe());
    }
    if let Some(save) = &resumed {
        if save.seats.is_empty() && !save.picks.is_empty() {
            mtg_player::stderr_line!("note: this snapshot predates the guide/model record, so what its \
{} replayed picks were made under is unknown", save.picks.len());
        }
    }

    // One seeded root RNG, so the packs a run deals can be dealt again.
    let mut rng = <rand::rngs::StdRng as rand::SeedableRng>::seed_from_u64(args.seed);

    // Load set data
    let set_path = PathBuf::from(format!("data/sets/{}.json", args.set));
    let mut set_data = SetData::load(&set_path).unwrap_or_else(|e| {
        mtg_player::stderr_line!("Failed to load set data: {e}");
        std::process::exit(1);
    });

    let registry = CardRegistry::with_all_cards();
    let removed = set_data.filter_implemented(&registry);
    if !removed.is_empty() && !args.quiet {
        mtg_player::stderr_line!(
            "Warning: {} cards not implemented, removed from draft pool",
            removed.len()
        );
    }

    let sheets = SheetData::from_set_data(&set_data).unwrap_or_else(|e| {
        mtg_player::stderr_line!("Failed to build sheet data: {e}");
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
        mtg_player::stderr_line!(
            "=== {} Draft: {} players, best-of-{} ===",
            set_data.set_name, args.players, args.best_of
        );
        mtg_player::stderr_line!("Log file: {}", args.log);
    }

    // ── Phase 1: Generate packs ──
    if !args.quiet {
        mtg_player::stderr_line!("Generating booster packs...");
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
        mtg_player::stderr_line!("Starting draft...");
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
    // What the snapshot carries past the picks (issue #581): the decks, once
    // they were built, and every match the tournament finished.
    let resumed_decks: Vec<SavedDeck> = resumed.as_ref().map(|s| s.decks.clone()).unwrap_or_default();
    let resumed_matches: Vec<SavedMatch> = resumed.as_ref().map(|s| s.matches.clone()).unwrap_or_default();
    let replaying: Vec<PickRecord> = resumed.map(|s| s.picks).unwrap_or_default();
    if let Err(e) = check_snapshot_shape(&replaying, args.players, draft.cards_remaining(0)) {
        die(&format!("draft save '{}' cannot be replayed: {e}",
            args.resume.as_deref().unwrap_or_default()));
    }
    // Decks are all of them or none, and matches need decks to have been
    // played with (issue #581). The runner never writes anything else.
    if !resumed_decks.is_empty() && resumed_decks.len() != args.players {
        die(&format!("draft save '{}' cannot be replayed: it holds {} decks for {} seats",
            args.resume.as_deref().unwrap_or_default(), resumed_decks.len(), args.players));
    }
    if !resumed_decks.is_empty() && replaying.len() != args.players * 3 * draft.cards_remaining(0) {
        die(&format!("draft save '{}' cannot be replayed: it holds decks but the draft is not finished",
            args.resume.as_deref().unwrap_or_default()));
    }
    if resumed_decks.is_empty() && !resumed_matches.is_empty() {
        die(&format!("draft save '{}' cannot be replayed: it holds matches but no decks",
            args.resume.as_deref().unwrap_or_default()));
    }
    if !replaying.is_empty() && !args.quiet {
        mtg_player::stderr_line!("Replaying {} recorded pick(s) from the snapshot...", replaying.len());
    }
    // The cost summary counts this process's calls only, so it has to say
    // which part of the draft it is the cost of (issue #578).
    llm_client::note_replayed_picks(replaying.len());
    let write_snapshot = |picks: &[PickRecord], decks: &[SavedDeck], matches: &[SavedMatch]| {
        let Some(path) = &args.save else { return };
        let save = DraftSave {
            seed: args.seed,
            set: args.set.clone(),
            players: args.players,
            picks: picks.to_vec(),
            seats: args.seat_policies(),
            decks: decks.to_vec(),
            matches: matches.to_vec(),
        };
        // Write-then-rename: a snapshot half-written when the run dies is
        // worse than none, because it looks resumable.
        let tmp = format!("{path}.tmp");
        match serde_json::to_string(&save).map_err(|e| e.to_string())
            .and_then(|text| fs::write(&tmp, text).map_err(|e| e.to_string()))
            .and_then(|()| fs::rename(&tmp, path).map_err(|e| e.to_string()))
        {
            Ok(()) => {}
            Err(e) => mtg_player::stderr_line!("\nWARN: could not write the draft snapshot to {path}: {e}"),
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
                write_snapshot(&recorded, &[], &[]);
                continue;
            }

            if !args.quiet {
                draw_progress(&format!("Pack {} Pick {}/{}", round + 1, pick_num + 1, initial_cards));
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
                                    mtg_player::game_log::buffer_ranked(seat as u64);
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
                    mtg_player::stderr_line!("{}WARN: seat {} pack {} pick {}: could not use the response, \
substituting {} (the first card). Response: {}",
                        end_progress_line(), seat, round + 1, pick_num + 1, pick.card(),
                        response.trim().replace('\n', " "));
                    log_draft_warning!(log, seat, round + 1, pick_num + 1, pick.card(), &response);
                }
                let chosen = pick.into_card();

                draft.make_pick(seat, &chosen).unwrap_or_else(|e| {
                    mtg_player::stderr_line!("\nDraft pick error for seat {seat}: {e}");
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
            write_snapshot(&recorded, &[], &[]);
        }
    }

    if !args.quiet {
        mtg_player::stderr_line!("{}Draft complete!", end_progress_line());
    }

    // What the protection covers from here, said where it matters rather
    // than only in `--save`'s help. It used to end at this line, and said so
    // (#581's first half); it now runs through the tournament.
    if args.save.is_some() && !args.quiet {
        mtg_player::stderr_line!("note: the snapshot now covers deck building and every match too — \
an interruption from here costs the build or the match in progress, not the run");
    }

    // Log final pools
    log_section!(log, "DRAFT POOLS");
    for seat in 0..args.players {
        log_pool_summary!(log, seat, &draft.players[seat].pool);
    }

    // ── Phase 3: Deck Building ──
    log_section!(log, "DECK BUILDING");
    if !args.quiet {
        mtg_player::stderr_line!("Building decks...");
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

    // A snapshot written after deck building carries the decks, and a
    // resumed run uses them as built rather than paying for the builds again
    // (issue #581). A deck the runner substituted stays one.
    let deck_results: Vec<DeckBuildResult> = if resumed_decks.len() == args.players {
        if !args.quiet {
            mtg_player::stderr_line!("Taking the {} decks from the snapshot...", args.players);
        }
        resumed_decks.iter().enumerate().map(|(seat, saved)| {
            log_deck_building!(log, seat, &saved.deck.maindeck, &saved.deck.lands,
                &saved.deck.sideboard, &[], saved.retries, saved.fallback);
            mtg_player::game_log::write(file!(), line!(), &format!(
                "[Seat {seat}] DECK FROM SNAPSHOT — built by the run that wrote the snapshot; \
its build attempts are in that run's log"), "");
            DeckBuildResult {
                deck: saved.deck.clone(),
                attempts: Vec::new(),
                retries: saved.retries,
                fallback: saved.fallback,
            }
        }).collect()
    } else { std::thread::scope(|s| {
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
                mtg_player::game_log::buffer_ranked(seat as u64);
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
    }) };

    // Checkpoint the builds: from here an interruption costs a match, not
    // the deck-building phase again (issue #581).
    let saved_decks: Vec<SavedDeck> = deck_results.iter().map(|r| SavedDeck {
        deck: r.deck.clone(), fallback: r.fallback, retries: r.retries,
    }).collect();
    let mut saved_matches: Vec<SavedMatch> = Vec::new();
    write_snapshot(&recorded, &saved_decks, &saved_matches);
    // Matches this process did not play, per seat, for the standings.
    let mut from_snapshot = vec![0usize; args.players];

    // Build the decklist collection in seat order now that all workers
    // have finished. No further logging — that already happened above.
    let decklists: Vec<Decklist> = deck_results
        .iter()
        .map(|result| Decklist {
            entries: deckbuilding::to_decklist(&result.deck),
        })
        .collect();

    if !args.quiet {
        mtg_player::stderr_line!("\nDecks built!");
    }

    // ── Phase 4: Tournament ──
    log_section!(log, "TOURNAMENT");
    if !args.quiet {
        mtg_player::stderr_line!("Starting Swiss tournament...");
    }

    let tournament_config = TournamentConfig {
        best_of: args.best_of,
    };
    let mut tournament = Tournament::new(args.players, tournament_config);

    // A pod with nobody to pair off plays no rounds — `total_rounds` returns 0
    // for it deliberately — and nothing said so: the log printed an empty
    // TOURNAMENT header straight into FINAL STANDINGS, and a seat that played
    // nothing was ranked `0-0`, which is the row for a seat that played and
    // went even. The same rule as a bye, a substituted pick or deck, and a
    // forfeited game: a standing the run did not earn has to be marked where
    // the standings are (#195, #200, #486, #488, issue #608).
    let played_nothing = tournament.total_rounds() == 0;
    if played_nothing {
        // `WARN` in the label, so `grep WARN` over the log finds it the way
        // it finds a substituted pick (#195).
        mtg_player::game_log::write(
            file!(), line!(),
            &format!(
                "WARN NO ROUNDS — a {}-seat pod has no pairings, so no match was played \
and the FINAL STANDINGS below record none",
                args.players
            ),
            "",
        );
    }

    while !tournament.is_complete() {
        let round_num = tournament.rounds.len() + 1;
        let pairings = tournament.generate_pairings();

        if !args.quiet {
            mtg_player::stderr_line!("Round {}/{}", round_num, tournament.total_rounds());
        }

        // Separate byes from real matches
        let real_matches: Vec<(usize, usize)> = pairings
            .iter()
            .filter(|&&(_, b)| b != BYE)
            .copied()
            .collect();

        for &(a, _) in pairings.iter().filter(|&&(_, b)| b == BYE) {
            if !args.quiet {
                mtg_player::stderr_line!("  Seat {a} gets a bye");
            }
        }

        if !args.quiet {
            for &(a, b) in &real_matches {
                mtg_player::stderr_line!("  Seat {a} vs Seat {b}");
            }
        }

        // A match the snapshot already finished is taken from it, not played
        // again: the pairings are a function of the results so far, so the
        // same round pairs the same seats (issue #581).
        let carried: Vec<Option<MatchResult>> = real_matches.iter().map(|&(a, b)| {
            resumed_matches.iter()
                .find(|m| m.round == round_num && m.result.player_a == a && m.result.player_b == b)
                .map(|m| m.result.clone())
        }).collect();
        let to_play: Vec<(usize, usize)> = real_matches.iter().zip(&carried)
            .filter(|(_, c)| c.is_none())
            .map(|(m, _)| *m)
            .collect();
        for (&(a, b), c) in real_matches.iter().zip(&carried) {
            if let Some(result) = c {
                from_snapshot[a] += 1;
                from_snapshot[b] += 1;
                saved_matches.push(SavedMatch { round: round_num, result: result.clone() });
            }
        }
        // Written now, before any match is played: the only other write is
        // when a match of this process's own finishes, so a resume that
        // carried everything, or was stopped before its first match ended,
        // left a `--save` holding none of the matches it carried — and
        // `--resume X --save X` erased them from the only snapshot (#732).
        if carried.iter().any(Option::is_some) {
            write_snapshot(&recorded, &saved_decks, &saved_matches);
        }

        // Play the rest in parallel. Each is checkpointed the moment it
        // finishes, in the order they finish, so an interruption costs the
        // matches still in progress and no more (issue #581).
        let played: Vec<MatchResult> = std::thread::scope(|s| {
            let (finished_tx, finished_rx) = std::sync::mpsc::channel::<MatchResult>();
            let handles: Vec<_> = to_play
                .iter()
                .enumerate()
                .map(|(k, &(a, b))| {
                    let finished_tx = finished_tx.clone();
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
                        mtg_player::game_log::buffer_ranked(k as u64);
                        let outcome = play_match(
                            &PlayerSpec { seat: a, deck: deck_a, model_spec: model_a, guide: guide_a },
                            &PlayerSpec { seat: b, deck: deck_b, model_spec: model_b, guide: guide_b },
                            reg,
                            best_of,
                            quiet,
                            card_ref,
                            seed,
                        );
                        let _ = finished_tx.send(outcome.clone());
                        (outcome, mtg_player::game_log::take_buffered())
                        })
                    })
                })
                .collect();
            drop(finished_tx);
            for finished in finished_rx {
                saved_matches.push(SavedMatch { round: round_num, result: without_game_logs(finished) });
                write_snapshot(&recorded, &saved_decks, &saved_matches);
            }

            handles
                .into_iter()
                .zip(to_play.iter())
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

        let mut played = played.into_iter();
        let from_save: Vec<bool> = carried.iter().map(Option::is_some).collect();
        let results: Vec<MatchResult> = carried.into_iter()
            .map(|c| c.unwrap_or_else(|| played.next().expect("a played result for every match not carried")))
            .collect();

        // Log byes
        for &(a, b) in &pairings {
            if b == BYE {
                log_bye!(log, round_num, a);
            }
        }

        // Log match results and game logs
        for (result, &carried_over) in results.iter().zip(&from_save) {
            if carried_over {
                mtg_player::game_log::write(file!(), line!(), &format!(
                    "MATCH FROM SNAPSHOT (Seat {} vs Seat {}) — played by the run that wrote \
the snapshot, not by this one; its games are in that run's log",
                    result.player_a, result.player_b), "");
            }
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
                let note = if carried_over { " [from snapshot]" } else { "" };
                mtg_player::stderr_line!("{}{note}", match_score_line(result));
            }
        }

        tournament.record_round(pairings, results);
    }

    // ── Phase 5: Output ──
    // A game the watchdog forfeited is counted as a loss in the standings
    // and was never played out, so the run has to say so next to them —
    // the same rule as a substituted deck or pick (#195, #200, #488).
    let mut stalled_games = vec![0usize; args.players];
    for game in tournament.rounds.iter().flat_map(|r| r.results.iter()).flat_map(|m| m.games.iter()) {
        if let Some(seat) = game.stalled_seat {
            stalled_games[seat] += 1;
        }
    }
    // Every qualifier the standings carry, counted before they are printed
    // rather than after, so the row a reader ranks a seat by says it
    // (issue #588). The game seats are named `Seat{n}` in the per-seat
    // tallies (see `play_match`).
    let rejected = mtg_player::llm::get_rejected_by_seat();
    let unanswered = mtg_player::llm::get_unanswered_by_seat();
    let row_tags: Vec<RowTags> = (0..args.players).map(|seat| RowTags {
        answers_substituted: rejected.get(&format!("Seat{seat}")).copied().unwrap_or(0),
        never_answered: unanswered.get(&format!("Seat{seat}")).copied().unwrap_or(0),
        runner_built_deck: deck_results[seat].fallback,
        games_forfeited: stalled_games[seat],
        matches_from_snapshot: from_snapshot[seat],
    }).collect();

    log_section!(log, "FINAL STANDINGS");
    let sorted = tournament.sorted_standings();
    log_standings!(log, &sorted, &row_tags);

    if !args.quiet {
        mtg_player::stderr_line!("\nFinal Standings:");
        for (rank, s) in sorted.iter().enumerate() {
            mtg_player::stderr_line!("  {}", standings_row(rank + 1, s, &row_tags[s.seat]));
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
        mtg_player::stderr_line!("\n=== Substituted Decks ===");
        for seat in &fallback_seats {
            mtg_player::stderr_line!("    Seat {seat}: no valid deck after {} attempts — the runner built \
this seat's deck, so its results are not a built deck's", deck_results[*seat].retries);
        }
        mtg_player::stderr_line!("  (grep the log for FALLBACK to see each one)");
    }

    if stalled_games.iter().any(|n| *n > 0) {
        mtg_player::stderr_line!("\n=== Forfeited Games ===");
        for (seat, n) in stalled_games.iter().enumerate() {
            if *n > 0 {
                mtg_player::stderr_line!("    Seat {seat}: {n} game(s) forfeited — this seat stopped making \
progress (the same unusable answer over and over), so the game was awarded to its opponent");
            }
        }
        mtg_player::stderr_line!("  (grep the log for STALLED to see each one)");
    }

    // A game the runner stopped at its action budget is in the standings as
    // a game neither seat won; say which, next to them (#630).
    let abandoned: Vec<(usize, usize)> = tournament.rounds.iter()
        .flat_map(|r| r.results.iter())
        .flat_map(|m| m.games.iter().filter(|g| g.abandoned).map(|_| (m.player_a, m.player_b)))
        .collect();
    if !abandoned.is_empty() {
        mtg_player::stderr_line!("\n=== Abandoned Games ===");
        for (a, b) in &abandoned {
            mtg_player::stderr_line!("    Seat {a} vs Seat {b}: the game ran past the runner's action budget \
with no result, so the runner stopped it — neither seat won it");
        }
        mtg_player::stderr_line!("  (grep the log for ABANDONED to see each one)");
    }

    if played_nothing {
        mtg_player::stderr_line!("\n=== No Tournament ===");
        mtg_player::stderr_line!(
            "    A {}-seat pod has no pairings, so no match was played: every standing above \
is an unplayed 0-0, not a result",
            args.players
        );
    }

    let substituted_total: usize = substituted_picks.iter().sum();
    if substituted_total > 0 {
        mtg_player::stderr_line!("\n=== Substituted Picks ===");
        mtg_player::stderr_line!("  {substituted_total} pick(s) were made by the runner, not by a seat:");
        for (seat, n) in substituted_picks.iter().enumerate() {
            if *n > 0 {
                mtg_player::stderr_line!("    Seat {seat}: {n} pick(s) unusable — this seat's pool, \
deck and results are not a drafted one");
            }
        }
        mtg_player::stderr_line!("  (grep the log for WARN to see each one)");
    }

    if !args.quiet {
        mtg_player::stderr_line!("\nDone. Log written to {}", args.log);
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
    mtg_player::stderr_line!("Warning: deck building failed after {max_retries} attempts, using fallback");
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
        let game_number = games.len() + 1;
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
            MatchFormat::BestOf {
                best_of,
                game: game_number,
                your_wins: wins_a,
                their_wins: wins_b,
            },
            MatchFormat::BestOf {
                best_of,
                game: game_number,
                your_wins: wins_b,
                their_wins: wins_a,
            },
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

        // A seat whose backend gave up forfeits the rest of the match, not
        // only the game it gave up in: the games it would have played are
        // its opponent's, recorded as forfeits so the standings and
        // `=== Forfeited Games ===` say so (#587). Nothing is asked of it.
        let dead = [(seat_a, &p1), (seat_b, &p2)].into_iter()
            .find(|(_, p)| mtg_player::Player::gave_up(*p).is_some())
            .map(|(seat, _)| seat);
        if let Some(dead) = dead {
            let winner = if dead == seat_a { seat_b } else { seat_a };
            while !match_is_over(best_of, games.len(), wins_a, wins_b) {
                if winner == seat_a { wins_a += 1 } else { wins_b += 1 }
                games.push(GameOutcome {
                    winner: Some(winner), turns: 0, game_log: Vec::new(),
                    stalled_seat: Some(dead), abandoned: false,
                });
            }
        }
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
    // Each seat's own side of the match: the score is stated from the seat's
    // point of view, so the two are mirrors of each other (issue #609).
    format_a: MatchFormat,
    format_b: MatchFormat,
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

    // Re-initialize conversations for this game (fresh context per game).
    // The context is fresh, so whatever the seat is to know about the match
    // around this game has to be in the system prompt — it is the only thing
    // that survives (issue #609).
    p1.init_conversation(&deck_a.entries, card_reference, registry, format_a);
    p2.init_conversation(&deck_b.entries, card_reference, registry, format_b);

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
    let mut abandoned = false;

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
                mtg_player::stderr_line!("\nWARN: {report} The game is forfeit to seat {}.",
                    if acting_player == PlayerId(0) { seat_b } else { seat_a });
                draft_log::DraftLogger::stalled_game(
                    seat_a, seat_b, seat,
                    game_state.turn_number,
                    &format!("{:?}", game_state.step),
                    &report,
                    file!(), line!(),
                );
            }

            if let Some(stop) = harness_move(stalled, action_count, max_actions) {
                if matches!(stop, mtg_engine::actions::Action::AbandonGame) && !abandoned {
                    abandoned = true;
                    mtg_player::stderr_line!("\nWARN: Seat {seat_a} vs Seat {seat_b}: the game reached \
{max_actions} actions without a result at turn {} {:?}; the runner abandoned it — no winner.",
                        game_state.turn_number, game_state.step);
                    draft_log::DraftLogger::abandoned_game(
                        seat_a, seat_b, max_actions,
                        game_state.turn_number,
                        &format!("{:?}", game_state.step),
                        file!(), line!(),
                    );
                }
                return stop;
            }

            let view = GameView::for_player(game_state, acting_player, registry);

            let player: &mut LlmPlayer = if acting_player == PlayerId(0) {
                p1
            } else {
                p2
            };

            let answer = if let Some(prompt) = &legal.combat_prompt {
                player.choose_combat(&view, prompt)
            } else {
                player.choose_action(&view, legal)
            };
            // A seat whose backend spent its whole retry budget without an
            // answer has stopped playing: it forfeits this game through the
            // stall path, and `play_match` forfeits it the rest of the match.
            // Not a silent degrade onto fallbacks, and not a fatal that stops
            // every other match in the process (#587).
            if let Some(why) = mtg_player::Player::gave_up(&*player) {
                if stalled_seat.is_none() {
                    let seat = if acting_player == PlayerId(0) { seat_a } else { seat_b };
                    stalled_seat = Some(seat);
                    let report = format!("Seat {seat}'s backend never answered within its retry \
budget ({why}), so the seat forfeits its match");
                    mtg_player::stderr_line!("\nWARN: {report}.");
                    draft_log::DraftLogger::stalled_game(
                        seat_a, seat_b, seat,
                        game_state.turn_number,
                        &format!("{:?}", game_state.step),
                        &report,
                        file!(), line!(),
                    );
                }
                return mtg_player::watchdog::forfeit_move();
            }
            answer
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
        abandoned,
    }
}

/// The move the runner itself makes, when it makes one, before the acting
/// seat is asked anything.
///
/// The two reasons to stop are not the same event. A seat the watchdog
/// caught spinning forfeits: the loss is that seat's, and the report says
/// so. A game that is still moving but has run out of budget is the
/// harness stopping, and nobody wins it — sending the forfeit there handed
/// the game to whichever seat was *not* acting at action 50,000 (#630,
/// the draft runner's copy of #233).
///
/// Either is sent, not looked up. `legal.actions` lists `Concede` only on
/// the normal-priority path, so reaching for it there made both no-ops at
/// every prompt — a mulligan, a discard, a declaration, any resolution
/// choice — which is where a spinning seat usually is, leaving the game
/// with no termination condition at all (issue #559). The engine accepts
/// either at any decision point (CR 104.3a, `LegalActions::permits`).
fn harness_move(stalled: bool, action_count: u64, max_actions: u64) -> Option<mtg_engine::actions::Action> {
    if stalled {
        Some(mtg_player::watchdog::forfeit_move())
    } else if action_count >= max_actions {
        Some(mtg_player::watchdog::ceiling_move())
    } else {
        None
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
    use super::{standings_row, RowTags, Standing};

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
        let played = standings_row(2, &standing(0, 1, 1, 1, 0), &RowTags::default());
        let byed = standings_row(3, &standing(1, 1, 1, 1, 1), &RowTags::default());

        assert_eq!(played, "2. Seat 0 — 1-1 (1 game wins)");
        assert_eq!(byed, "3. Seat 1 — 1-1 (1 game wins) [1 bye]");
    }

    #[test]
    fn several_byes_and_draws_are_both_reported() {
        let mut s = standing(4, 2, 1, 2, 2);
        s.match_draws = 1;
        assert_eq!(standings_row(1, &s, &RowTags::default()), "1. Seat 4 — 2-1-1 (2 game wins) [2 byes]");
    }

    /// Issue #588: a result that is not wholly the seat's own says so on
    /// the row the seat is ranked by, the way a bye does — not in a section
    /// under a heading about tokens.
    #[test]
    fn every_qualifier_is_on_the_row() {
        let s = standing(1, 3, 0, 6, 0);
        let one = |t: RowTags| standings_row(1, &s, &t);
        assert_eq!(one(RowTags { answers_substituted: 39, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [39 answers substituted]");
        assert_eq!(one(RowTags { never_answered: 1, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [1 decision never answered]");
        assert_eq!(one(RowTags { runner_built_deck: true, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [runner-built deck]");
        assert_eq!(one(RowTags { games_forfeited: 1, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [1 game forfeited]");
        assert_eq!(one(RowTags { matches_from_snapshot: 2, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [2 matches from snapshot]");
        let mut byed = s.clone();
        byed.byes = 1;
        assert_eq!(standings_row(1, &byed, &RowTags { games_forfeited: 2, runner_built_deck: true, ..RowTags::default() }),
            "1. Seat 1 — 3-0 (6 game wins) [1 bye] [runner-built deck] [2 games forfeited]");
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

#[cfg(test)]
mod snapshot_shape_tests {
    use super::{check_snapshot_shape, PickRecord};

    fn rec(round: usize, pick: usize, seat: usize) -> PickRecord {
        PickRecord { round, pick, seat, card: "Moon Heron".into(), substituted: false }
    }

    /// Whole steps for `players` seats, from pack 1 pick 1, `steps` of them.
    fn whole(players: usize, pack_size: usize, steps: usize) -> Vec<PickRecord> {
        (0..steps)
            .flat_map(|i| (0..players).map(move |seat| rec(i / pack_size + 1, i % pack_size + 1, seat)))
            .collect()
    }

    #[test]
    fn a_save_the_runner_writes_is_accepted() {
        assert_eq!(check_snapshot_shape(&[], 2, 14), Ok(()));
        assert_eq!(check_snapshot_shape(&whole(2, 14, 5), 2, 14), Ok(()));
        // Across a pack boundary, and the whole draft.
        assert_eq!(check_snapshot_shape(&whole(8, 14, 16), 8, 14), Ok(()));
        assert_eq!(check_snapshot_shape(&whole(2, 14, 42), 2, 14), Ok(()));
    }

    /// #655: two records for seat 0 and none for seat 1 replayed both picks
    /// into seat 0.
    #[test]
    fn two_records_for_one_seat_are_refused() {
        let save = vec![rec(1, 1, 0), rec(1, 1, 0)];
        let err = check_snapshot_shape(&save, 2, 14).unwrap_err();
        assert!(err.contains("two records for seat 0 at pack 1 pick 1"), "{err}");
    }

    /// #656: a step missing a seat silently dropped the other seat's record.
    #[test]
    fn a_step_missing_a_seat_is_refused() {
        let mut save = whole(2, 14, 5);
        save.retain(|r| !(r.pick == 5 && r.seat == 1));
        let err = check_snapshot_shape(&save, 2, 14).unwrap_err();
        assert!(err.contains("pack 1 pick 5 it has records for seat(s) 0 and none for seat(s) 1"), "{err}");

        let mut gap = whole(2, 14, 5);
        gap.retain(|r| r.pick != 3);
        let err = check_snapshot_shape(&gap, 2, 14).unwrap_err();
        assert!(err.contains("no records for pack 1 pick 3 but has records after it"), "{err}");
    }

    #[test]
    fn a_record_outside_the_draft_is_refused() {
        assert!(check_snapshot_shape(&[rec(1, 1, 2)], 2, 14).unwrap_err().contains("seat 2"));
        assert!(check_snapshot_shape(&[rec(4, 1, 0)], 2, 14).unwrap_err().contains("pack 4 pick 1"));
        assert!(check_snapshot_shape(&[rec(1, 15, 0)], 2, 14).unwrap_err().contains("pack 1 pick 15"));
        assert!(check_snapshot_shape(&[rec(1, 0, 0)], 2, 14).unwrap_err().contains("pack 1 pick 0"));
    }
}

#[cfg(test)]
mod match_score_line_tests {
    use super::match_score_line;
    use mtg_draft::tournament::MatchResult;

    fn result(wins_a: usize, wins_b: usize) -> MatchResult {
        MatchResult { player_a: 0, player_b: 1, wins_a, wins_b, games: Vec::new() }
    }

    /// #650: a level match printed "(winner: Seat draw)".
    #[test]
    fn a_drawn_match_names_no_winner() {
        assert_eq!(match_score_line(&result(1, 1)), "  Seat 0 vs Seat 1: 1-1 (drawn)");
        assert_eq!(match_score_line(&result(0, 2)), "  Seat 0 vs Seat 1: 0-2 (winner: Seat 1)");
    }
}

#[cfg(test)]
mod harness_stop_tests {
    use super::{harness_move, unplayed_games_note};
    use mtg_draft::tournament::GameOutcome;
    use mtg_engine::actions::Action;

    /// #630: the action ceiling and the stall forfeit shared one move, so a
    /// game still progressing at action 50,000 was conceded on behalf of
    /// whichever seat held the decision — the loss #233 removed from
    /// `mtg-runner`. The ceiling is the harness stopping; nobody wins it.
    #[test]
    fn the_action_ceiling_abandons_the_game_rather_than_conceding_it() {
        assert!(harness_move(false, 49_999, 50_000).is_none());
        assert!(matches!(harness_move(false, 50_000, 50_000), Some(Action::AbandonGame)),
            "the ceiling must not be a seat's concede");
        // A stalled seat still forfeits: that loss is the stuck seat's.
        assert!(matches!(harness_move(true, 10, 50_000), Some(Action::Concede)));
        assert!(matches!(harness_move(true, 50_000, 50_000), Some(Action::Concede)));
    }

    #[test]
    fn an_abandoned_game_is_named_on_the_score_line() {
        let game = |stalled_seat, abandoned| GameOutcome {
            winner: None, turns: 30, game_log: vec![], stalled_seat, abandoned,
        };
        assert_eq!(unplayed_games_note(&[game(None, false)]), "");
        assert_eq!(unplayed_games_note(&[game(None, true)]),
            " [1 game abandoned: the action budget ran out, no winner]");
        assert_eq!(unplayed_games_note(&[game(Some(1), false), game(None, true), game(None, true)]),
            " [1 game forfeited: a seat stalled] [2 games abandoned: the action budget ran out, no winner]");
    }
}
