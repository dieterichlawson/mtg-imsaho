//! Global thread-safe log writer.
//!
//! Call `init` once at startup with the log file path. Then use `write` from
//! any thread — all writes are serialized through a single Mutex<File>.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::Instant;

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

/// Write a log entry with timestamp, thread ID, source location, and content.
/// No-op if `init` was never called.
pub fn write(file: &str, line: u32, label: &str, content: &str) {
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

    let _ = writeln!(
        state.file,
        "[{:02}:{:02}:{:02}.{:03} {:?}] [{}:{}] {}",
        hrs, mins, secs, millis, tid, filename, line, label
    );
    if !content.is_empty() {
        for ln in content.lines() {
            let _ = writeln!(state.file, "  {}", ln);
        }
    }
    let _ = writeln!(state.file);
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
