//! Global thread-safe log writer.
//!
//! Call `init` once at startup with the log file path. Then use `write` from
//! any thread — all writes are serialized through a single Mutex<File>.
//!
//! Each entry's header is a single tab-delimited line with this schema:
//!
//!   <elapsed>\t<LEVEL>\t<thread>\t<file>:<line>\t<LABEL>\t<content>
//!
//! Fields 1-5 are safe to split on `\t`; the final content field may contain
//! arbitrary text (including tabs or backslashes). Multi-line content is
//! written as indented continuation lines below the header with a blank
//! line separator — use `grep -A`/`grep -B` or an editor to view.

use std::fs::{File, OpenOptions};
use std::fmt::Write as _;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Verbose tracing: raw backend JSON, auto-pass transitions,
    /// action-collapse bookkeeping, etc. Omitted in the default view.
    Debug,
    /// Normal operational events: prompts, decisions, game state.
    Info,
    /// Recoverable errors: malformed LLM responses, API retries that
    /// eventually succeeded, fallback activations, etc.
    Error,
}

impl LogLevel {
    /// Full-word level tag rendered in the log line.
    fn name(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info  => "INFO",
            LogLevel::Error => "ERROR",
        }
    }
}

struct LogState {
    file: File,
}

static LOG: Mutex<Option<LogState>> = Mutex::new(None);

/// Whether [`init`] has run, readable without taking [`LOG`]'s lock so that
/// a run with no `--log` does no formatting work per record.
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Records the runner's per-seat workers are collecting rather than
/// writing, by worker thread: the rank the runner gave it (the order its
/// block is written back in) and the block so far. See [`buffer_here`].
///
/// A registry rather than a thread-local, so the fatal path can reach every
/// worker's block and not only its own: `report_worker_failure` used to
/// flush the failing worker alone, and every other match's records were
/// lost at `process::exit` (#658).
static HELD: Mutex<Vec<(std::thread::ThreadId, u64, String)>> = Mutex::new(Vec::new());

/// Whether any worker is collecting, readable without the lock, so a run
/// that buffers nothing pays nothing per record.
static ANY_HELD: AtomicBool = AtomicBool::new(false);

fn held() -> std::sync::MutexGuard<'static, Vec<(std::thread::ThreadId, u64, String)>> {
    match HELD.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// Collect this thread's records instead of writing them, so a caller can
/// write them back in an order of its choosing.
///
/// The deck-build and tournament phases run a worker per seat and per match
/// and each logs inline, so two runs of the same `--seed` wrote the same
/// lines in whatever order the scheduler produced — 1,047 differing hunks
/// over 72,114 identical lines, at four seats. The run replayed; its record
/// did not, which makes `diff` of two seeded logs — the one cheap check of
/// "did this seed replay?" — report thousands of differences that mean
/// nothing (issue #541). The pick loop has always joined in seat order and
/// logged afterwards, which is why the draft half is byte-identical.
///
/// Every record is held, `Error` ones included. They used to be written
/// through, on the premise that a clean seeded run has none — false for any
/// real model, whose `MALFORMED` answers then landed up to 62,000 lines from
/// the PROMPT they answered, in an order two runs of one seed disagreed on
/// (#658). What an operator watches a live run for is on stderr instead: the
/// backends' `API_*` lines, and one line per rejected answer.
///
/// `rank` is the block's place when a fatal writes every held block out at
/// once ([`flush_all`]): the seat, or the match's place in its round.
pub fn buffer_ranked(rank: u64) {
    let me = std::thread::current().id();
    let mut held = held();
    held.retain(|(t, _, _)| *t != me);
    held.push((me, rank, String::new()));
    ANY_HELD.store(true, Ordering::SeqCst);
}

/// [`buffer_ranked`], for a worker whose place no fatal flush needs: it
/// goes after every ranked one.
pub fn buffer_here() {
    buffer_ranked(u64::MAX);
}

/// Stop collecting and hand back what this thread recorded.
#[must_use]
pub fn take_buffered() -> String {
    let me = std::thread::current().id();
    let mut held = held();
    match held.iter().position(|(t, _, _)| *t == me) {
        Some(i) => held.remove(i).2,
        None => String::new(),
    }
}

/// Write a block taken from a worker, in one piece and in the caller's
/// order.
pub fn write_block(block: &str) {
    if !block.is_empty() {
        emit(block);
    }
}

/// Write out whatever this thread is holding, now.
pub fn flush_here() {
    let held = take_buffered();
    write_block(&held);
}

/// Write out every block every worker is holding, in rank order, now.
///
/// For the fatal path: the process is about to exit and will not come back
/// for any of them, so every match's records — not only the failing
/// worker's — are written before it goes (#658).
pub fn flush_all() {
    let mut blocks: Vec<(u64, String)> = held().drain(..).map(|(_, r, b)| (r, b)).collect();
    blocks.sort_by_key(|(r, _)| *r);
    for (_, block) in blocks {
        write_block(&block);
    }
}

/// Append a formatted block to the log file.
fn emit(block: &str) {
    let mut guard = match LOG.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(state) = guard.as_mut() else { return };
    let _ = state.file.write_all(block.as_bytes());
    let _ = state.file.flush();
}

/// Initialize the global log writer. Call once at startup.
/// If already initialized, replaces the previous writer (the file it was
/// writing to keeps whatever it already holds).
///
/// Opened for append, which is what `--log` has always promised: "Append the
/// game log to this file". It truncated instead, so an operator recording a
/// matchup under one `--log` path — or simply re-running the same command
/// after a crash — destroyed the previous game's log silently, with exit 0.
/// A run's record is evidence; a flag that says it accumulates must not be
/// the thing that erases it.
///
/// The path is user input, so failure to open it is returned rather than
/// panicking (issue #69): the caller owns how a bad `--log` argument is
/// reported.
pub fn init(path: &str) -> std::io::Result<()> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut guard = match LOG.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    *guard = Some(LogState { file });
    INITIALIZED.store(true, Ordering::SeqCst);
    Ok(())
}

/// The level a record's label is worth at least.
///
/// The level used to be each call site's own decision, and call sites
/// disagreed: the same `is_error` retry from `claude -p` was written at
/// `Error` by the draft backend and at `Info` by the game backend, so
/// `grep ERROR` answered "did this run hit the usage limit?" for a draft
/// and not for a game (#583). Four more game-side records had drifted the
/// same way, including an `API_FATAL` written at `Info` immediately before
/// `process::exit(1)`.
///
/// `LogLevel::Error` is documented above as "recoverable errors: malformed
/// LLM responses, API retries that eventually succeeded, fallback
/// activations", which is exactly this family of labels. Deriving the floor
/// from the label rather than from the call site is what stops the two
/// copies of a request path from drifting apart again: there is one rule,
/// in one place, and a new record inherits it by being named.
pub fn level_floor(label: &str) -> LogLevel {
    if label.starts_with("API_") || label == "MALFORMED" {
        LogLevel::Error
    } else {
        LogLevel::Info
    }
}

/// Write a log entry at the level its label implies. No-op if `init` was
/// never called.
pub fn write(file: &str, line: u32, label: &str, content: &str) {
    write_at(level_floor(label), file, line, label, content);
}

/// Write a log entry at the given level. The header is a single tab-delimited
/// line; multi-line content is written as indented continuation lines with a
/// trailing blank line as a record separator.
pub fn write_at(level: LogLevel, file: &str, line: u32, label: &str, content: &str) {
    if !INITIALIZED.load(Ordering::Relaxed) {
        return;
    }
    // A caller may raise a record's level but not lower it below what its
    // label is worth: see `level_floor`.
    let level = match (level, level_floor(label)) {
        (LogLevel::Error, _) | (_, LogLevel::Info) => level,
        (_, floor) => floor,
    };

    let thread = std::thread::current();
    let filename = file.rsplit('/').next().unwrap_or(file);
    // Wall-clock timestamp in the local timezone, ISO-8601-ish with
    // millisecond precision. Use a space between date and time so the
    // line parses cleanly under `cut -f` / `awk -F'\t'`.
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let loc = format!("{filename}:{line}");
    let level_name = level.name();
    // A named thread says who it is; only an unnamed one falls back to its
    // id. `t5` is the one identifier in a log line that means nothing to
    // anybody: two `API_FATAL` lines from a stopped run named `t5` and `t2`
    // and there was no way back from either to the seat whose account or
    // session was the broken one (issues #539, #542). The runners name
    // their per-seat workers, so those lines now name the seat instead.
    let tid_field = thread.name().map_or_else(
        || {
            // Thread id renders like `ThreadId(12)` from the Debug impl —
            // strip the wrapper for a slightly terser field.
            let tid_str = format!("{:?}", thread.id());
            tid_str
                .strip_prefix("ThreadId(")
                .and_then(|s| s.strip_suffix(')'))
                .map(|n| format!("t{n}"))
                .unwrap_or(tid_str)
        },
        std::string::ToString::to_string,
    );

    // Single-line content: everything on one tab-delimited header line.
    // Multi-line content: header line with no content field, followed by
    // flush-left continuation lines of bare content. No 2-space indent,
    // no per-line tag, no trailing blank separator. Header rows are
    // visually distinct because they start with a timestamp digit and
    // contain tab-delimited fields; body rows are free-form text.
    let mut record = String::new();
    if !content.is_empty() && !content.contains('\n') {
        let _ = writeln!(
            record,
            "{}\t{}\t{}\t{}\t{}\t{}",
            ts, level_name, tid_field, loc, label, content.trim_end()
        );
    } else {
        let _ = writeln!(record, "{ts}\t{level_name}\t{tid_field}\t{loc}\t{label}");
        for ln in content.lines() {
            let _ = writeln!(record, "{}", ln.trim_end());
        }
    }

    // A worker collecting its records holds every one of them, errors
    // included: see `buffer_ranked`.
    if ANY_HELD.load(Ordering::Relaxed) {
        let me = thread.id();
        let mut held = held();
        if let Some((_, _, block)) = held.iter_mut().find(|(t, _, _)| *t == me) {
            block.push_str(&record);
            return;
        }
    }
    emit(&record);
}

/// Convenience macro that captures file!() and line!() at the call site.
#[macro_export]
macro_rules! game_log {
    ($label:expr, $content:expr) => {
        $crate::game_log::write(file!(), line!(), $label, $content)
    };
    ($label:expr) => {
        $crate::game_log::write(file!(), line!(), $label, "")
    };
}
