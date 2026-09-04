//! Argument handling for the draft runner. Every seat here can be a metered
//! LLM, so a bad invocation must be refused before a single token is spent:
//! `--help` answers instead of drafting, an unrecognized flag or a bad value
//! is a clean one-line `Error:` (never a panic, never a silent default), and
//! a claude-code seat is checked for its CLI up front rather than after the
//! draft has already been billed.
//!
//! These run the mtg-draft-runner binary as a subprocess. None of them can
//! reach a real model: the refusals exit first, and the one run that gets
//! past validation dies on a missing set file, which main loads before it
//! builds any client.

use std::process::{Command, Output};

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A bad argument is reported and refused — exit 1 (bad invocation), a clean
/// `Error:` line, no panic, and no sign the draft started anyway.
fn assert_clean_refusal(out: &Output, what: &str) {
    let err = stderr(out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(1),
        "{what}: a bad argument exits 1 (bad invocation), not 101 (crash).\n\
         stdout: {stdout}\nstderr: {err}");
    assert!(err.contains("Error:"), "{what}: refusal is a clean Error line.\nstderr: {err}");
    assert!(!err.contains("panicked"), "{what}: no panic/backtrace.\nstderr: {err}");
    assert!(!err.contains("Starting draft"), "{what}: the draft must not start.\nstderr: {err}");
}

#[test]
fn help_prints_usage_and_exits_without_drafting() {
    let out = runner().arg("--help").output().expect("failed to run");
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Usage: mtg-draft-runner"), "stdout: {stdout}");
    assert!(stdout.contains("--model-<N>"), "help lists the per-seat flags.\nstdout: {stdout}");
    assert!(stderr(&out).is_empty(), "nothing ran: stderr is empty.\nstderr: {}", stderr(&out));
}

#[test]
fn version_prints_the_version_and_exits_without_drafting() {
    let out = runner().arg("--version").output().expect("failed to run");
    assert_eq!(out.status.code(), Some(0));
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("mtg-draft-runner "));
}

#[test]
fn an_unrecognized_flag_is_refused_not_ignored() {
    let out = runner().args(["--modle", "claude"]).output().expect("failed to run");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("unrecognized argument '--modle'"), "stderr: {}", stderr(&out));
}

#[test]
fn a_flag_without_its_value_is_refused() {
    let out = runner().arg("--model").output().expect("failed to run");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(stderr(&out).contains("--model requires a value"), "stderr: {}", stderr(&out));
}

#[test]
fn an_unknown_model_provider_is_refused_not_defaulted_to_a_metered_seat() {
    let out = runner().args(["--model", "wizard"]).output().expect("failed to run");
    assert_clean_refusal(&out, "--model wizard");
    let err = stderr(&out);
    assert!(err.contains("unknown provider 'wizard'"), "the message names the bad value.\nstderr: {err}");
    assert!(err.contains("claude-code"), "the message lists what is accepted.\nstderr: {err}");
}

#[test]
fn a_non_numeric_count_is_refused() {
    let out = runner().args(["--players", "eight"]).output().expect("failed to run");
    assert_clean_refusal(&out, "--players eight");
    assert!(stderr(&out).contains("--players takes a number"), "stderr: {}", stderr(&out));
}

#[test]
fn a_per_seat_flag_for_a_seat_that_does_not_exist_is_refused() {
    let out = runner()
        .args(["--players", "2", "--model-5", "claude"])
        .output()
        .expect("failed to run");
    assert_clean_refusal(&out, "--model-5 with --players 2");
    assert!(stderr(&out).contains("there is no seat 5"), "stderr: {}", stderr(&out));
}

#[test]
fn an_unreadable_guide_file_is_refused_rather_than_drafted_without() {
    let missing = std::env::temp_dir().join("mtg-draft-runner-no-such-guide.md");
    let out = runner()
        .args(["--guide", &missing.to_string_lossy()])
        .output()
        .expect("failed to run");
    assert_clean_refusal(&out, "--guide naming a missing file");
    assert!(stderr(&out).contains("failed to read guide file"), "stderr: {}", stderr(&out));
}

// ── the claude-code preflight ───────────────────────────────────────

#[test]
fn a_claude_code_seat_without_the_cli_is_refused_before_the_draft() {
    for spec in ["claude-code", "cc", "cc:opus"] {
        let out = runner()
            .args(["--model", spec])
            .env("CLAUDE_CODE_BIN", "/nonexistent/claude")
            .output()
            .expect("failed to run");
        assert_clean_refusal(&out, spec);
        let err = stderr(&out);
        assert!(err.contains("needs the Claude Code CLI"), "{spec}\nstderr: {err}");
        assert!(err.contains("CLAUDE_CODE_BIN"), "the message says how to point at it.\nstderr: {err}");
    }
}

/// With a runnable `claude`, the preflight passes and the run gets as far as
/// loading the set — the failure below is the missing set file, not the seat.
#[test]
fn a_claude_code_seat_with_a_runnable_cli_passes_the_preflight() {
    let out = runner()
        .args(["--model", "cc", "--set", "no-such-set"])
        .env("CLAUDE_CODE_BIN", "/bin/true")
        .output()
        .expect("failed to run");
    let err = stderr(&out);
    assert!(!err.contains("Claude Code CLI"), "the seat itself is fine.\nstderr: {err}");
    assert!(err.contains("Failed to load set data"), "it got past validation.\nstderr: {err}");
}
