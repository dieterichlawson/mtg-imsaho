//! The in-place pick progress line, and the lines printed while it is up.
//!
//! The progress line is drawn with `\r` and no newline, so anything printed
//! while it is on the terminal landed on the end of it — a fatal read
//! `Pack 1 Pick 5/14Error: …`, and a seat's failed call
//! `Pack 1 Pick 1/14claude -p failed …` (#657). Every line printed during
//! the draft goes through [`say`], which ends the progress line first.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

/// Whether the progress line is on the terminal without its newline.
static OPEN: AtomicBool = AtomicBool::new(false);

/// Redraw the in-place progress line.
pub fn draw_progress(line: &str) {
    let _ = write!(std::io::stderr(), "\r{line}");
    OPEN.store(true, Ordering::SeqCst);
}

/// The newline that ends the progress line, if one is open; empty if not.
pub fn end_progress_line() -> &'static str {
    if OPEN.swap(false, Ordering::SeqCst) {
        "\n"
    } else {
        ""
    }
}

/// Print one line on stderr, on a line of its own.
///
/// A write that fails is dropped: the runtime ignores SIGPIPE, so
/// `eprintln!` to a closed stderr panics, and in a seat worker that turned a
/// retryable call failure into a fatal one (#652).
pub fn say(line: &str) {
    let _ = writeln!(std::io::stderr(), "{}{}{line}", end_progress_line(), seat_tag());
}

/// `[SeatN] ` when a seat's own worker is speaking, else nothing.
///
/// Every seat picks in parallel, so four seats' failed calls printed four
/// identical "claude -p failed" lines and nothing said whose they were
/// (#688). The worker threads are named for their seat (`spawn_seat`), the
/// way the log already names them; stderr now says it in the form the game
/// backends' lines use (#659).
fn seat_tag() -> String {
    let thread = std::thread::current();
    seat_of(thread.name().unwrap_or("")).map_or_else(String::new, |n| format!("[Seat{n}] "))
}

/// The seat a worker thread named `seat N` is, if it is one.
fn seat_of(thread_name: &str) -> Option<usize> {
    thread_name.strip_prefix("seat ")?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{draw_progress, end_progress_line, seat_of};

    /// #688: a seat's worker is named `seat N`; a match's (`seat 0 v 1`)
    /// and the main thread are not one seat.
    #[test]
    fn a_line_from_a_seats_worker_says_which_seat() {
        assert_eq!(seat_of("seat 3"), Some(3));
        assert_eq!(seat_of("seat 0 v 1"), None);
        assert_eq!(seat_of("main"), None);
        let tag = std::thread::Builder::new().name("seat 2".into())
            .spawn(super::seat_tag).unwrap().join().unwrap();
        assert_eq!(tag, "[Seat2] ");
        assert_eq!(super::seat_tag(), "", "the test's own thread is no seat");
    }

    #[test]
    fn a_line_after_the_progress_line_starts_on_its_own() {
        draw_progress("Pack 1 Pick 5/14");
        assert_eq!(end_progress_line(), "\n", "the open progress line is ended once");
        assert_eq!(end_progress_line(), "", "and only once");
    }
}
