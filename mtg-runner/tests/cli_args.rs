//! Argument handling beyond deck loading: a bad value is refused with a
//! clean one-line `Error:` and exit 1 — never a panic/backtrace (exit 101),
//! and never a silent substitution that plays a different game than the one
//! requested (issues #69, #70; siblings of #52 and #55).
//!
//! These run the mtg-runner binary as a subprocess to test the full CLI flow.

use std::process::Command;

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
}

/// A directory that exists (to use where a *file* path is required) — the
/// temp dir, which is always present.
fn a_directory() -> String {
    std::env::temp_dir().to_string_lossy().into_owned()
}

/// A path whose parent directory does not exist.
fn a_missing_directory_path() -> String {
    std::env::temp_dir()
        .join("mtg-runner-cli-args-test-nonexistent")
        .join("x.log")
        .to_string_lossy()
        .into_owned()
}

fn assert_clean_refusal(output: &std::process::Output, what: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(1),
        "{what}: a bad argument exits 1 (bad invocation), not 101 (crash).\n\
         stdout: {stdout}\nstderr: {stderr}");
    assert!(stderr.contains("Error:"),
        "{what}: refusal is a clean Error line.\nstderr: {stderr}");
    assert!(!stderr.contains("panicked"),
        "{what}: no panic/backtrace.\nstderr: {stderr}");
    assert!(!stdout.contains("Game over!"),
        "{what}: the game must not run as if the argument were valid.\n\
         stdout: {stdout}");
}

// ── issue #69: unwritable --log / --save paths ──────────────────────

#[test]
fn log_into_a_missing_directory_is_refused_cleanly() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "-q",
               "--log", &a_missing_directory_path()])
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--log into a missing directory");
}

#[test]
fn log_path_that_is_a_directory_is_refused_cleanly() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "-q", "--log", &a_directory()])
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--log naming a directory");
}

#[test]
fn save_path_that_is_a_directory_is_refused_cleanly() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "-q", "--save", &a_directory()])
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--save naming a directory");
}
