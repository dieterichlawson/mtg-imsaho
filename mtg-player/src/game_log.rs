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

use std::cell::RefCell;
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

thread_local! {
    /// Records this thread is collecting rather than writing, if it is one
    /// of the runner's per-seat workers. See [`buffer_here`].
    static BUFFER: RefCell<Option<String>> = const { RefCell::new(None) };
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
/// Only what the seed determines is held back. An `Error` record — a retry,
/// a malformed answer, a fatal — is written through as it happens: those are
/// the records an operator watches a long run for, and they are nowhere in
/// a clean seeded run to begin with, so holding them would trade away
/// visibility for determinism that is already there.
pub fn buffer_here() {
    let _ = BUFFER.try_with(|b| *b.borrow_mut() = Some(String::new()));
}

/// Stop collecting and hand back what this thread recorded.
#[must_use]
pub fn take_buffered() -> String {
    BUFFER.try_with(|b| b.borrow_mut().take()).ok().flatten().unwrap_or_default()
}

/// Write a block taken from a worker, in one piece and in the caller's
/// order.
pub fn write_block(block: &str) {
    if !block.is_empty() {
        emit(block);
    }
}

/// Write out whatever this thread is holding, now.
///
/// For the fatal path: a worker that is about to take the process down with
/// it still owes the log everything it recorded, and `process::exit` will
/// not come back for it.
pub fn flush_here() {
    let held = take_buffered();
    write_block(&held);
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

/// Write an info-level log entry. Equivalent to `write_at(LogLevel::Info, ..)`.
/// No-op if `init` was never called.
pub fn write(file: &str, line: u32, label: &str, content: &str) {
    write_at(LogLevel::Info, file, line, label, content);
}

/// Write a log entry at the given level. The header is a single tab-delimited
/// line; multi-line content is written as indented continuation lines with a
/// trailing blank line as a record separator.
pub fn write_at(level: LogLevel, file: &str, line: u32, label: &str, content: &str) {
    if !INITIALIZED.load(Ordering::Relaxed) {
        return;
    }

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

    // An `Error` record is written through: see `buffer_here`.
    if level != LogLevel::Error {
        let buffered = BUFFER.try_with(|b| {
            b.borrow_mut().as_mut().map(|buf| buf.push_str(&record)).is_some()
        });
        if buffered == Ok(true) {
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
