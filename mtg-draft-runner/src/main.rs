use std::env;
use std::fs;
use std::path::PathBuf;

use mtg_draft::deckbuilding;
use mtg_draft::draft::DraftState;
use mtg_draft::pack::{generate_draft_packs, SheetData};
use mtg_draft::set_data::SetData;
use mtg_draft::tournament::{MatchResult, Tournament, TournamentConfig, BYE};

use mtg_engine::cards::CardRegistry;
use mtg_engine::engine::Decklist;

use mtg_draft_runner::deck::{build_deck_with_llm, DeckBuildResult};
use mtg_draft_runner::game::{make_game_player, match_is_over, match_seed, play_match, GameSeat, MatchSeat};
use mtg_draft_runner::pick::{parse_pick_response, Pick};
use mtg_draft_runner::progress::{draw_progress, end_progress_line};
use mtg_draft_runner::standings::{match_score_line, standings_row, RowTags};
use mtg_draft_runner::{card_lines, die, draft_log, llm_client};
use mtg_draft_runner::{log_bye, log_deck_building, log_draft_pick, log_draft_warning, log_game_log,
    log_header, log_match_result, log_pack_contents, log_pool_summary, log_replayed_pick,
    log_section, log_standings, log_subsection, log_system_prompt};

/// A match result as the snapshot keeps it: the games' logs are the run's
/// log's, and a snapshot written after every match has to stay small.
fn without_game_logs(mut result: MatchResult) -> MatchResult {
    for game in &mut result.games {
        game.game_log.clear();
    }
    result
}

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
  --resume <path>        Replay a snapshot and carry on from it. Its seed, set,
                         seat count and match length win over the flags — the packs are
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

/// Every seat's pool after `picks`, made on a copy of `draft` — or the
/// first record that names a card its pack did not hold.
///
/// Run before the resumed run writes anything. The replay used to find an
/// impossible card only when it reached it, after it had already rewritten
/// `--save` with the steps before it: `--resume X --save X` refused the
/// save and destroyed it in one go, and the next resume re-asked the lost
/// picks in silence (#733). `picks` has passed `check_snapshot_shape`.
fn replay_pools(mut draft: DraftState, picks: &[PickRecord], players: usize) -> Result<Vec<Vec<String>>, String> {
    'draft: for round in 0..3 {
        if round > 0 {
            draft.start_next_pack_round();
        }
        for pick_num in 0..draft.cards_remaining(0) {
            let step: Vec<&PickRecord> = picks.iter()
                .filter(|p| p.round == round + 1 && p.pick == pick_num + 1)
                .collect();
            if step.is_empty() {
                break 'draft;
            }
            for seat in 0..players {
                let Some(rec) = step.iter().find(|p| p.seat == seat) else { continue };
                draft.make_pick(seat, &rec.card).map_err(|e| format!(
                    "it replays an impossible pick (seat {seat}, pack {}, pick {}, {}): {e}",
                    round + 1, pick_num + 1, rec.card))?;
            }
            draft.rotate_packs();
        }
    }
    Ok(draft.players.iter().map(|p| p.pool.clone()).collect())
}

/// Whether every saved deck is one its seat could have built from the pool
/// the replayed picks give it: the checks a freshly built deck passes.
///
/// The saved decks were played as they stood, so a hand-edited or corrupted
/// snapshot played a 30-card deck of cards its seat never drafted, exit 0,
/// and a name that is not a card panicked a match worker once the
/// tournament was under way (#730). A card that is not in the pool is
/// refused here, and every card in a pool is a card the set dealt.
fn check_snapshot_decks(decks: &[SavedDeck], pools: &[Vec<String>]) -> Result<(), String> {
    for (seat, (saved, pool)) in decks.iter().zip(pools).enumerate() {
        deckbuilding::validate_deck(pool, &saved.deck.maindeck, &saved.deck.lands)
            .map_err(|e| format!("seat {seat}'s deck is not one its pool builds: {e}"))?;
    }
    Ok(())
}

/// Whether every saved match is one this tournament finished: a score its
/// own games add up to, decided at the match length, and a pairing the
/// tournament makes in the round it names, given the matches before it.
///
/// A record was carried when its round and seats matched a pairing, and
/// taken as it stood: a 7-0 best-of-1 went into the standings, and a record
/// with its seats the other way round, for a seat or a round the pod does
/// not have, or twice over, was dropped in silence — the match it stood for
/// re-played and re-billed (#731). The pairings are a function of the
/// results, so the tournament is walked here on the records alone.
fn check_snapshot_matches(matches: &[SavedMatch], players: usize, best_of: usize) -> Result<(), String> {
    let name = |m: &SavedMatch| format!("round {}, seat {} v seat {}", m.round, m.result.player_a, m.result.player_b);
    for (i, m) in matches.iter().enumerate() {
        let r = &m.result;
        if matches[..i].iter().any(|o| o.round == m.round
            && o.result.player_a == r.player_a && o.result.player_b == r.player_b)
        {
            return Err(format!("it has two records for {}", name(m)));
        }
        if r.games.iter().any(|g| g.winner.is_some_and(|w| w != r.player_a && w != r.player_b)) {
            return Err(format!("its record for {} has a game won by a seat not in the match", name(m)));
        }
        let won = |seat: usize, games: &[mtg_draft::tournament::GameOutcome]| games.iter().filter(|g| g.winner == Some(seat)).count();
        let (a, b) = (won(r.player_a, &r.games), won(r.player_b, &r.games));
        if (r.wins_a, r.wins_b) != (a, b) {
            return Err(format!("its record for {} says {}-{}, and its games say {a}-{b}",
                name(m), r.wins_a, r.wins_b));
        }
        let before = &r.games[..r.games.len().saturating_sub(1)];
        let finished = !r.games.is_empty()
            && match_is_over(best_of, r.games.len(), a, b)
            && !match_is_over(best_of, before.len(), won(r.player_a, before), won(r.player_b, before));
        if !finished {
            return Err(format!("its record for {} is {a}-{b} over {} game(s), which is not a finished \
best-of-{best_of}", name(m), r.games.len()));
        }
    }
    let mut tournament = Tournament::new(players, TournamentConfig { best_of });
    let mut used = vec![false; matches.len()];
    while !tournament.is_complete() {
        let round = tournament.rounds.len() + 1;
        let pairings = tournament.generate_pairings();
        let real: Vec<(usize, usize)> = pairings.iter().filter(|&&(_, b)| b != BYE).copied().collect();
        let mut results = Vec::new();
        for &(a, b) in &real {
            let found = matches.iter()
                .position(|m| m.round == round && m.result.player_a == a && m.result.player_b == b);
            if let Some(i) = found {
                used[i] = true;
                results.push(matches[i].result.clone());
            }
        }
        if results.len() < real.len() {
            break;
        }
        tournament.record_round(pairings, results);
    }
    match used.iter().position(|&u| !u) {
        Some(i) => Err(format!("its record for {} is not a match this tournament pairs", name(&matches[i]))),
        None => Ok(()),
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
    /// The match length the matches above were played at. A resume took
    /// `--best-of` from the flags, so a best-of-1 save resumed at the
    /// default played round 2 as best-of-3 and ranked game wins from both
    /// formats on one table (#729). `None` in a snapshot written before the
    /// field existed, where the flag is all there is to go on.
    #[serde(default)]
    best_of: Option<usize>,
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
    // Each value the save overrode, for the log as well as stderr: the
    // header names the value used and nothing said where it came from.
    let mut resume_notes: Vec<String> = Vec::new();
    let resumed: Option<DraftSave> = args.resume.as_ref().map(|path| {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|e| die(&format!("failed to read draft save '{path}': {e}")));
        let save: DraftSave = serde_json::from_str(&text)
            .unwrap_or_else(|e| die(&format!("draft save '{path}' is not a valid snapshot: {e}")));
        // The flags' own floor, which a snapshot's values replace them past:
        // `"players": 0` panicked indexing the log header's seat 0 (#734),
        // and `"best_of": 0` played matches of no games.
        if save.players == 0 {
            die(&format!("draft save '{path}' cannot be replayed: it has 0 seats, and a draft needs at least 1"));
        }
        if save.best_of == Some(0) {
            die(&format!("draft save '{path}' cannot be replayed: its matches are best-of-0, and a match needs at least 1 game"));
        }
        // Only a value the operator actually asked for can be overridden.
        // Where they asked for nothing, the note says where the value came
        // from rather than inventing an argument they never gave (#582).
        for (flag, saved, used) in [
            ("--seed", save.seed.to_string(), args.seed.to_string()),
            ("--set", save.set.clone(), args.set.clone()),
            ("--players", save.players.to_string(), args.players.to_string()),
            ("--best-of", save.best_of.unwrap_or(args.best_of).to_string(), args.best_of.to_string()),
        ] {
            if saved == used {
                continue;
            }
            let note = if args.was_supplied(flag) {
                format!("{flag} comes from the save ({used} -> {saved})")
            } else {
                format!("{flag} {saved} comes from the save")
            };
            mtg_player::stderr_line!("note: {note}");
            resume_notes.push(note);
        }
        // The flag stands in for a length the snapshot never recorded, and
        // the next write records it as the save's own (#729): say so where
        // it will be read later, not only on stderr.
        if save.best_of.is_none() && !save.matches.is_empty() {
            let note = format!("--best-of {} is the flag's: this snapshot does not record the match \
length its matches were played at", args.best_of);
            mtg_player::stderr_line!("note: {note}");
            resume_notes.push(note);
        }
        save
    });
    if let Some(save) = &resumed {
        args.seed = save.seed;
        args.set.clone_from(&save.set);
        if let Some(best_of) = save.best_of {
            args.best_of = best_of;
        }
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

    let mut draft = DraftState::new(&packs);

    // The snapshot is checked whole before anything is written: before the
    // log is opened, so a refused resume leaves no header claiming a replay
    // that never happened (#735), and before the first snapshot write, so a
    // refused save is left as it was found (#733).
    //
    // What the snapshot carries past the picks (issue #581): the decks, once
    // they were built, and every match the tournament finished.
    let best_of_recorded = resumed.as_ref().is_some_and(|s| s.best_of.is_some());
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
    match replay_pools(draft.clone(), &replaying, args.players) {
        Ok(pools) => if let Err(e) = check_snapshot_decks(&resumed_decks, &pools) {
            die(&format!("draft save '{}' cannot be replayed: {e}",
                args.resume.as_deref().unwrap_or_default()));
        },
        Err(e) => die(&format!("draft save '{}' cannot be replayed: {e}",
            args.resume.as_deref().unwrap_or_default())),
    }
    if let Err(e) = check_snapshot_matches(&resumed_matches, args.players, args.best_of) {
        // A snapshot from before the match length was recorded is checked
        // against the flag, which defaults to 3 and may not be what it was
        // played at.
        let hint = if best_of_recorded { String::new() } else { format!(
            " (this snapshot does not record its match length, so best-of-{} is --best-of's; \
resume with the --best-of its run was played at)", args.best_of) };
        die(&format!("draft save '{}' cannot be replayed: {e}{hint}",
            args.resume.as_deref().unwrap_or_default()));
    }

    // Create streaming log file
    let log = draft_log::DraftLogger::new(std::path::Path::new(&args.log));
    let resumed_from = args.resume.as_deref().map(|path| (path, replaying.len()));
    let replayed_note: Vec<(usize, String, String)> = replayed_under
        .iter()
        .map(|(seat, was, now)| (*seat, was.describe(), now.describe()))
        .collect();
    log_header!(log, &set_data.set_name, args.players, args.best_of,
        args.models.as_slice(), args.guide_paths.as_slice(), args.seed, resumed_from,
        replayed_note.as_slice());
    for note in &resume_notes {
        mtg_player::game_log::write(file!(), line!(), &format!("NOTE {note}"), "");
    }

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
            best_of: Some(args.best_of),
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
                // Not written: the snapshot being replayed already holds
                // these steps, and the decks and matches after them that a
                // picks-only write would drop — `--resume X --save X` was
                // rewritten without them 42 times over before the first
                // thing this run did of its own.
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
    // With the snapshot's matches in it from the start: every one has been
    // checked to be a match this tournament takes, so the first write keeps
    // them rather than waiting for this run to finish one of its own — the
    // only write there was, which left a resume's `--save` with none (#732).
    let mut saved_matches: Vec<SavedMatch> = resumed_matches.clone();
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
            if c.is_some() {
                from_snapshot[a] += 1;
                from_snapshot[b] += 1;
            }
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
                        // The game seats are named `Seat{n}`, which is the
                        // name the per-seat tallies are read under.
                        let outcome = play_match(
                            MatchSeat { seat: a, deck: deck_a, player: GameSeat::Llm(
                                make_game_player(model_a, &format!("Seat{a}"), guide_a)) },
                            MatchSeat { seat: b, deck: deck_b, player: GameSeat::Llm(
                                make_game_player(model_b, &format!("Seat{b}"), guide_b)) },
                            reg,
                            best_of,
                            card_ref,
                            seed,
                            &mut |_| {},
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
                // Both ways a seat stops answering forfeit here: the
                // watchdog's stall, and a backend that spent its retry
                // budget (#587), which forfeits the rest of its match too.
                // Naming only the first described the second wrongly, the
                // shape #742 removed from the game-over line.
                mtg_player::stderr_line!("    Seat {seat}: {n} game(s) forfeited — this seat stopped \
answering (the same unusable answer over and over, or a backend that gave up), so the game was \
awarded to its opponent");
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

#[cfg(test)]
mod snapshot_shape_tests {
    use super::{check_snapshot_shape, PickRecord};

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
        assert_eq!(save.best_of, None, "an older snapshot leaves the match length to the flag");
    }

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
mod snapshot_match_tests {
    use super::{check_snapshot_matches, SavedMatch};
    use mtg_draft::tournament::{GameOutcome, MatchResult, Tournament, TournamentConfig, BYE};

    fn game(winner: usize) -> GameOutcome {
        GameOutcome { winner: Some(winner), turns: 9, game_log: vec![], stalled_seat: None, abandoned: false }
    }

    /// Round 1 of a fresh 4-seat pod as the runner would record it, the
    /// first seat of every pairing winning 1-0.
    fn round_one() -> Vec<SavedMatch> {
        let t = Tournament::new(4, TournamentConfig { best_of: 1 });
        t.generate_pairings().into_iter().filter(|&(_, b)| b != BYE).map(|(a, b)| SavedMatch {
            round: 1,
            result: MatchResult { player_a: a, player_b: b, wins_a: 1, wins_b: 0, games: vec![game(a)] },
        }).collect()
    }

    #[test]
    fn the_records_the_runner_writes_are_accepted() {
        assert_eq!(check_snapshot_matches(&[], 4, 1), Ok(()));
        assert_eq!(check_snapshot_matches(&round_one(), 4, 1), Ok(()));
        // A round interrupted part-way.
        assert_eq!(check_snapshot_matches(&round_one()[..1], 4, 1), Ok(()));
    }

    /// #731: a 7-0 best-of-1 was carried into the standings as it stood.
    #[test]
    fn a_score_its_games_do_not_add_up_to_is_refused() {
        let mut saved = round_one();
        saved[0].result.wins_a = 7;
        let err = check_snapshot_matches(&saved, 4, 1).unwrap_err();
        assert!(err.contains("says 7-0, and its games say 1-0"), "{err}");
    }

    #[test]
    fn a_match_not_finished_at_the_saves_length_is_refused() {
        // A 1-0 is a finished best-of-1 and not a finished best-of-3.
        let err = check_snapshot_matches(&round_one(), 4, 3).unwrap_err();
        assert!(err.contains("not a finished best-of-3"), "{err}");
        // And a game played after a best-of-3 was decided.
        let mut saved = round_one();
        let (a, b) = (saved[0].result.player_a, saved[0].result.player_b);
        saved[0].result = MatchResult { player_a: a, player_b: b, wins_a: 2, wins_b: 1,
            games: vec![game(a), game(a), game(b)] };
        let err = check_snapshot_matches(&saved[..1], 4, 3).unwrap_err();
        assert!(err.contains("not a finished best-of-3"), "{err}");
    }

    /// #731: a record with its seats the other way round, for a round the
    /// pod never plays, for a seat it does not have, or twice over, was
    /// dropped in silence and its match re-played.
    #[test]
    fn a_record_the_tournament_never_consumes_is_refused() {
        let mut swapped = round_one();
        let r = &mut swapped[0].result;
        (r.player_a, r.player_b) = (r.player_b, r.player_a);
        (r.wins_a, r.wins_b) = (0, 1);
        let err = check_snapshot_matches(&swapped, 4, 1).unwrap_err();
        assert!(err.contains("is not a match this tournament pairs"), "{err}");

        let mut phantom = round_one();
        phantom.push(SavedMatch { round: 9, ..phantom[0].clone() });
        let err = check_snapshot_matches(&phantom, 4, 1).unwrap_err();
        assert!(err.contains("round 9") && err.contains("is not a match this tournament pairs"), "{err}");

        let mut no_seat = round_one();
        no_seat[0].result.player_b = 5;
        assert!(check_snapshot_matches(&no_seat, 4, 1).is_err());

        let mut twice = round_one();
        twice.push(twice[0].clone());
        let err = check_snapshot_matches(&twice, 4, 1).unwrap_err();
        assert!(err.contains("two records for round 1"), "{err}");
    }
}
