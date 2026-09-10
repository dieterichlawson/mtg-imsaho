//! There is one `claude -p` subprocess driver in this workspace, and the
//! draft seat is not a second copy of it.
//!
//! It was. The draft backend was copied out of `mtg-player`'s on
//! 2026-09-04; the fixes for #203 and #206 landed in the original a day
//! later and never reached the copy. So the draft seat still killed only
//! the direct child and not its group, still read stdout to EOF, had no
//! signal handlers, no scratch sweep for its own prefix, and no timeout
//! override to make the path testable in seconds. A hung wrapper stopped a
//! draft mid-pick — 45 picks per seat over an hour — silently, forever,
//! with nothing on the terminal or in the log (issue #404).
//!
//! Two copies of a subprocess lifecycle is the defect, so this is not a
//! style rule: it is why one crate's fix was not the other's.

use std::path::Path;

fn draft_client() -> String {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/llm_client.rs");
    std::fs::read_to_string(src).expect("the draft runner's client source")
}

/// The file's own code, up to its `#[cfg(test)]` module: the comments talk
/// about all of this on purpose, and the tests below them stand in a fake
/// `claude` of their own, which is not a second driver.
fn code_lines(src: &str) -> Vec<(usize, &str)> {
    let end = src.find("#[cfg(test)]").unwrap_or(src.len());
    src[..end]
        .lines()
        .enumerate()
        .map(|(n, l)| (n + 1, l.trim()))
        .filter(|(_, l)| !l.starts_with("//"))
        .collect()
}

#[test]
fn the_draft_seat_drives_no_subprocess_of_its_own() {
    let src = draft_client();
    // Each of these is one of the things the copy did for itself, and each
    // of them is something #203/#206 had to be fixed in two places because
    // of it.
    let owned: &[(&str, &str)] = &[
        ("Stdio::piped", "wiring up the pipes"),
        ("pre_exec", "putting the child in its own process group"),
        // A pipe, not a file: the seat reads its own set data and guide.
        (".read_to_string(", "reading a pipe to EOF"),
        ("recv_timeout", "waiting out a call"),
        ("try_wait", "polling the child"),
        (".kill()", "killing the child"),
        ("killpg", "killing the group"),
        ("SIGTERM", "handling a signal"),
        // Its own directory on `Drop` is the seat's; scanning `/tmp` for
        // other runs' is the driver's.
        ("read_dir(std::env::temp_dir", "sweeping scratch directories"),
    ];
    let mut offenders = Vec::new();
    for (n, line) in code_lines(&src) {
        for (needle, what) in owned {
            if line.contains(needle) {
                offenders.push(format!("llm_client.rs:{n}: {what} — {line}"));
            }
        }
    }
    assert!(offenders.is_empty(),
        "the draft seat builds the argv and hands it to the one driver; \
         doing any of this itself is how #203's and #206's fixes reached \
         one crate and not the other (#404):\n  {}",
        offenders.join("\n  "));
}

#[test]
fn the_draft_seat_calls_the_shared_driver() {
    let src = draft_client();
    let lines = code_lines(&src);
    assert!(lines.iter().any(|(_, l)| l.contains("claude_code_run(")),
        "the draft seat runs its command through mtg_player's driver");
    assert!(lines.iter().any(|(_, l)| l.contains("claude_code_prepare_seat(")),
        "and sets its seat up through the same place, so the signal \
         handlers are installed and its own scratch prefix is swept");
}
