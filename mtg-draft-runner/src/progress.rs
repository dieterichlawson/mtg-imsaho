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
    let _ = writeln!(std::io::stderr(), "{}{line}", end_progress_line());
}

#[cfg(test)]
mod tests {
    use super::{draw_progress, end_progress_line};

    #[test]
    fn a_line_after_the_progress_line_starts_on_its_own() {
        draw_progress("Pack 1 Pick 5/14");
        assert_eq!(end_progress_line(), "\n", "the open progress line is ended once");
        assert_eq!(end_progress_line(), "", "and only once");
    }
}
