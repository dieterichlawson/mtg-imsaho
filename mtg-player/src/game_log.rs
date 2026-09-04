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
use std::io::Write;
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
    let mut guard = match LOG.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    };
    let Some(state) = guard.as_mut() else { return };

    let tid = std::thread::current().id();
    let filename = file.rsplit('/').next().unwrap_or(file);
    // Wall-clock timestamp in the local timezone, ISO-8601-ish with
    // millisecond precision. Use a space between date and time so the
    // line parses cleanly under `cut -f` / `awk -F'\t'`.
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let loc = format!("{filename}:{line}");
    let level_name = level.name();
    // Thread id renders like `ThreadId(12)` from the Debug impl — strip the
    // wrapper for a slightly terser field.
    let tid_str = format!("{tid:?}");
    let tid_field = tid_str
        .strip_prefix("ThreadId(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|n| format!("t{n}"))
        .unwrap_or(tid_str);

    // Single-line content: everything on one tab-delimited header line.
    // Multi-line content: header line with no content field, followed by
    // flush-left continuation lines of bare content. No 2-space indent,
    // no per-line tag, no trailing blank separator. Header rows are
    // visually distinct because they start with a timestamp digit and
    // contain tab-delimited fields; body rows are free-form text.
    if !content.is_empty() && !content.contains('\n') {
        let _ = writeln!(
            state.file,
            "{}\t{}\t{}\t{}\t{}\t{}",
            ts, level_name, tid_field, loc, label, content.trim_end()
        );
    } else {
        let _ = writeln!(
            state.file,
            "{ts}\t{level_name}\t{tid_field}\t{loc}\t{label}"
        );
        if !content.is_empty() {
            for ln in content.lines() {
                let trimmed = ln.trim_end();
                let _ = writeln!(state.file, "{trimmed}");
            }
        }
    }
    let _ = state.file.flush();
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
