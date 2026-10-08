//! The draft runner's library: what `mtg-draft-runner` (the lockstep LLM
//! draft), `mtg-draft-server` (the hosted table humans and AIs sit at) and
//! `mtg-draft-client` (its terminal client) share.

pub mod card_lines;
pub mod deck;
pub mod draft_log;
pub mod game;
pub mod llm_client;
pub mod pick;
pub mod progress;
pub mod standings;

pub use standings::{standings_row, RowTags};

use progress::end_progress_line;

/// A user error: report it and exit without a Rust panic/backtrace.
pub fn die(msg: &str) -> ! {
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
    // And the reason the run stopped, last: the log is the run's record,
    // and a refused or failed run's ended mid-section with no word of why
    // (#735). Nothing is written when no log is open yet.
    let _ = std::panic::catch_unwind(|| mtg_player::game_log::write_at(
        mtg_player::game_log::LogLevel::Error, file!(), line!(), &format!("FATAL {msg}"), ""));
    std::process::exit(1);
}
