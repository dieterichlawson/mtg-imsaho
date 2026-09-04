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

// ── issue #70: unknown player type ──────────────────────────────────

#[test]
fn an_unknown_player_type_is_refused_not_substituted() {
    let output = runner()
        .args(["--p1", "wizard", "--p2", "random", "-q", "--seed", "3"])
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--p1 wizard");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown player type 'wizard'"),
        "the message names the bad value.\nstderr: {stderr}");
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

// ── issue #103: a cli seat with no terminal is refused, not spun ────

/// With stdin redirected and no controlling terminal (setsid), a `cli`
/// seat used to print one frame and burn a core forever in its event
/// loop. It must instead refuse up front like any other argument that
/// cannot work. Run detached via setsid so the test is deterministic
/// even when the test runner itself has a tty.
#[test]
fn cli_seat_without_a_terminal_is_refused_not_spun() {
    use std::os::unix::process::CommandExt;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let mut cmd = runner();
    cmd.args(["--p1", "cli", "--p2", "random", "--seed", "1"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    unsafe {
        cmd.pre_exec(|| {
            // Detach from the controlling terminal so /dev/tty is closed
            // to the child even when cargo test runs under one.
            libc::setsid();
            Ok(())
        });
    }
    let mut child = cmd.spawn().expect("failed to spawn");

    // The refusal is immediate; anything near the deadline means the old
    // spin is back. Kill it rather than hanging the suite.
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait failed") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("cli seat without a terminal did not exit — the #103 spin is back");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let output = child.wait_with_output().expect("output failed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(status.code(), Some(1), "clean refusal, not a crash.\nstderr: {stderr}");
    assert!(stderr.contains("Error:") && stderr.contains("terminal"),
        "the message says a terminal is needed.\nstderr: {stderr}");
}

// ── a metered LLM seat without its API key ──────────────────────────

/// The claude/gemini backends read their key out of the environment and
/// unwrap it, so a user without a key used to get a Rust panic instead of
/// the clean refusal every other unusable seat gets. Empty is passed
/// rather than unset so the test never depends on the ambient environment
/// — and never reaches the network either.
#[test]
fn a_claude_seat_without_an_api_key_is_refused_not_panicked() {
    let output = runner()
        .args(["--p1", "claude", "--p2", "random", "-q", "--seed", "3"])
        .env("ANTHROPIC_API_KEY", "")
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--p1 claude with no key");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ANTHROPIC_API_KEY"),
        "the message names the variable to set.\nstderr: {stderr}");
}

#[test]
fn a_gemini_seat_without_an_api_key_is_refused_not_panicked() {
    let output = runner()
        .args(["--p1", "random", "--p2", "gemini", "-q", "--seed", "3"])
        .env("GEMINI_API_KEY", "")
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--p2 gemini with no key");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GEMINI_API_KEY"),
        "the message names the variable to set.\nstderr: {stderr}");
}

#[test]
fn a_claude_seat_with_a_model_suffix_and_no_key_is_refused_not_panicked() {
    let output = runner()
        .args(["--p1", "claude:some-model", "--p2", "random", "-q", "--seed", "3"])
        .env("ANTHROPIC_API_KEY", "")
        .output()
        .expect("failed to run");
    assert_clean_refusal(&output, "--p1 claude:some-model with no key");
}

// ── the documented seat surface matches the accepted one ────────────

/// `ai`, `llm` and `cc` are accepted seat specs; a reader who only has
/// --help or the unknown-seat error had no way to learn they exist.
const SEAT_ALIASES: &[&str] = &["ai", "llm", "cc"];

/// `contains` alone would pass on any word that happens to spell an alias
/// ("ai" inside "remaining"), so match the alias as a standalone word.
fn mentions_word(text: &str, word: &str) -> bool {
    text.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| w == word)
}

#[test]
fn help_names_every_accepted_seat_alias() {
    let output = runner().arg("--help").output().expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for alias in SEAT_ALIASES {
        assert!(mentions_word(&stdout, alias),
            "--help does not mention the accepted alias '{alias}'.\nstdout: {stdout}");
    }
}

#[test]
fn the_unknown_seat_error_names_every_accepted_seat_alias() {
    let output = runner()
        .args(["--p1", "wizard", "--p2", "random", "-q", "--seed", "3"])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);
    for alias in SEAT_ALIASES {
        assert!(mentions_word(&stderr, alias),
            "the unknown-seat error does not mention the accepted alias \
             '{alias}'.\nstderr: {stderr}");
    }
}

/// The aliases must stay accepted: each reaches the claude/claude-code
/// seat's own guard (a named key or CLI), never the unknown-type refusal.
#[test]
fn the_seat_aliases_still_reach_their_llm_seat() {
    for (alias, expected) in [("ai", "ANTHROPIC_API_KEY"), ("llm", "ANTHROPIC_API_KEY")] {
        let output = runner()
            .args(["--p1", alias, "--p2", "random", "-q", "--seed", "3"])
            .env("ANTHROPIC_API_KEY", "")
            .output()
            .expect("failed to run");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(expected),
            "--p1 {alias} is an alias for claude, not an unknown type.\nstderr: {stderr}");
    }
}
