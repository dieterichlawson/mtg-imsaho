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
use std::time::Instant;

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
    start: Instant,
}

static LOG: Mutex<Option<LogState>> = Mutex::new(None);

/// Initialize the global log writer. Call once at startup.
/// If already initialized, replaces the previous log file.
pub fn init(path: &str) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .unwrap_or_else(|e| panic!("Failed to create log file {}: {}", path, e));
    let mut guard = LOG.lock().unwrap();
    *guard = Some(LogState {
        file,
        start: Instant::now(),
    });
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
    let state = match guard.as_mut() {
        Some(s) => s,
        None => return,
    };

    let elapsed = state.start.elapsed();
    let total_secs = elapsed.as_secs();
    let millis = elapsed.subsec_millis();
    let hrs = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    let tid = std::thread::current().id();
    let filename = file.rsplit('/').next().unwrap_or(file);
    let ts = format!("{:02}:{:02}:{:02}.{:03}", hrs, mins, secs, millis);
    let loc = format!("{}:{}", filename, line);
    let level_name = level.name();
    // Thread id renders like `ThreadId(12)` from the Debug impl — strip the
    // wrapper for a slightly terser field.
    let tid_str = format!("{:?}", tid);
    let tid_field = tid_str
        .strip_prefix("ThreadId(")
        .and_then(|s| s.strip_suffix(')'))
        .map(|n| format!("t{}", n))
        .unwrap_or(tid_str);

    // Single-line content: everything on one tab-delimited header line.
    // Multi-line content: header line with no content field, followed by
    // indented continuation lines and a trailing blank separator.
    if !content.is_empty() && !content.contains('\n') {
        let _ = writeln!(
            state.file,
            "{}\t{}\t{}\t{}\t{}\t{}",
            ts, level_name, tid_field, loc, label, content.trim_end()
        );
    } else {
        let _ = writeln!(
            state.file,
            "{}\t{}\t{}\t{}\t{}",
            ts, level_name, tid_field, loc, label
        );
        if !content.is_empty() {
            for ln in content.lines() {
                let trimmed = ln.trim_end();
                if trimmed.is_empty() {
                    let _ = writeln!(state.file);
                } else {
                    let _ = writeln!(state.file, "  {}", trimmed);
                }
            }
        }
        let _ = writeln!(state.file);
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
