//! The draft runner's library: what `mtg-draft-runner` (the lockstep LLM
//! draft), `mtg-draft-server` (the hosted table humans and AIs sit at) and
//! `mtg-draft-client` (its terminal client) share.

pub mod card_lines;
pub mod client;
pub mod deck;
pub mod draft_log;
pub mod game;
pub mod llm_client;
pub mod lobby;
pub mod pick;
pub mod progress;
pub mod server;
pub mod standings;

pub use standings::{standings_row, RowTags};

/// "1 card", "3 cards": a count with its noun, singular at one. "1 picks",
/// "1 cards short" and "1 retries" each had a format string of their own
/// to slip in (#874, #914).
#[must_use]
pub fn count<N: std::fmt::Display + PartialEq + From<u8>>(n: N, one: &str, many: &str) -> String {
    format!("{n} {}", if n == N::from(1) { one } else { many })
}

use progress::end_progress_line;

/// Silence the default panic output for a seat's fatal LLM failure.
///
/// Exhausting the retries is deliberately fatal, but it is an operational
/// condition — a usage limit, a CLI outage — not a bug, and the operator
/// used to get a worker-thread panic with a backtrace followed by a second
/// panic whose whole message was `Any { .. }`. The panic is still how the
/// worker unwinds; `report_worker_failure` prints the one line that
/// matters, so the hook keeps quiet for these and behaves normally for a
/// real bug (issue #218).
pub fn install_panic_hook() {
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

/// The message a worker's panic payload carries, without the fatal marker.
#[must_use]
pub fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("worker thread failed");
    msg.strip_prefix(llm_client::FATAL_MARKER).unwrap_or(msg).to_string()
}

/// A user error: report it and exit without a Rust panic/backtrace.
pub fn die(msg: &str) -> ! {
    end_run(&format!("Error: {msg}"), &format!("FATAL {msg}"), 1)
}

/// Stop on Ctrl-C, SIGTERM or SIGHUP the way a fatal stops: the in-flight
/// calls are taken down at once (as they were), and then a thread of the
/// runner's own prints the usage summary, flushes the log, says why in it
/// and exits with the status the signal would have given. The handler
/// used to end the process on the spot, so an interrupted run published
/// no account of what it had spent, though a fatal had since #578 (#912).
/// A second signal still ends it at once.
pub fn stop_on_signals() {
    mtg_player::llm::stop_gracefully_on_signal();
    let _ = std::thread::Builder::new().name("signals".into()).spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        if let Some(sig) = mtg_player::llm::stop_signal() {
            let why = format!("stopped by {}", server::signal_name(sig));
            end_run(&format!("Stopped: {why}"), &format!("NOTE {why}; the run is not finished"), 128 + sig);
        }
    });
}

/// The end of a run that did not finish: `said` on stderr, the usage so
/// far, the log flushed and `logged` as its last record, then `status`.
fn end_run(said: &str, logged: &str, status: i32) -> ! {
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
        let _ = writeln!(std::io::stderr(), "{}{said}", end_progress_line());
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
    // And the reason the run stopped, last: the log is the run's record,
    // and a refused or failed run's ended mid-section with no word of why
    // (#735). Nothing is written when no log is open yet.
    let _ = std::panic::catch_unwind(|| mtg_player::game_log::write_at(
        mtg_player::game_log::LogLevel::Error, file!(), line!(), logged, ""));
    std::process::exit(status);
}
