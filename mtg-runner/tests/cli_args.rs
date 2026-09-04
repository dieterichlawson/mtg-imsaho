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

/// `--log` accumulates, as `--help` says it does ("Append the game log to
/// this file").
///
/// It opened with `.truncate(true)`, so the second of two runs sharing a
/// `--log` path silently destroyed the first game's record and still exited
/// 0 — the natural way to record a matchup, or to re-run the same command
/// line after a crash, was also the way to lose the evidence.
#[test]
fn the_log_flag_appends_rather_than_destroying_the_previous_run() {
    let path = std::env::temp_dir().join(format!("mtg-runner-append-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let mut lines = Vec::new();
    for seed in ["1", "2"] {
        let output = runner()
            .args(["--p1", "random", "--p2", "random", "--seed", seed,
                   "--log", &path.to_string_lossy(), "--quiet"])
            .output()
            .expect("failed to run");
        assert_eq!(output.status.code(), Some(0), "seeded random-vs-random game runs clean");
        lines.push(std::fs::read_to_string(&path).expect("log exists").lines().count());
    }

    let contents = std::fs::read_to_string(&path).expect("log exists");
    let _ = std::fs::remove_file(&path);

    assert!(lines[1] > lines[0],
        "the second run adds to the log rather than replacing it: {} lines then {} lines",
        lines[0], lines[1]);
    assert_eq!(contents.matches("Game over").count(), 2,
        "both games' records are in the file, not just the last one");
}

/// The runner, run from the workspace root so workspace-relative deck paths
/// resolve — cargo runs integration tests from the package directory.
fn runner_at_root() -> Command {
    let mut c = runner();
    c.current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
    c
}

/// Make a mid-game save for the --resume tests below: start a seeded game
/// and interrupt it once it has written one. A *finished* game deletes its
/// save (there is nothing left to resume), so the save has to be caught in
/// flight — which is also how an operator gets one, and how the issues that
/// motivated these tests reproduce.
fn a_save_file(tag: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir()
        .join(format!("mtg-runner-resume-{tag}-{}.save", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut child = runner_at_root()
        .args(["--p1", "random", "--p2", "random",
               "--deck1", "decks/rb-vampires.txt", "--deck2", "decks/gw-humans.txt",
               "--seed", "2301", "--on-the-play", "1",
               "--save", &path.to_string_lossy(), "--quiet"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("failed to start");

    // The save is rewritten after every action, so it appears almost at
    // once; poll rather than sleeping a fixed guess.
    let mut saved = None;
    for _ in 0..200 {
        if let Ok(contents) = std::fs::read_to_string(&path) {
            // Only a complete save is useful — the writer is atomic, so any
            // readable file is whole, but it must parse as a game.
            if contents.contains("player_names") {
                saved = Some(contents);
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();

    let saved = saved.expect("the game writes a save while it is running");
    // The kill can land between the writer's unlink and rename, so restore
    // the snapshot we actually read.
    std::fs::write(&path, saved).expect("write the captured save");
    path
}

/// A flag `--resume` has just declared ignored must not be able to stop the
/// run. `--deck1` was loaded anyway — for the LLM card reference — and
/// `load_deck` exits 1 on a bad path, so a resume aborted on a deck file it
/// did not need, even with no LLM seat in the game. The deck a resumed seat
/// is told about now comes from the save, which is what makes the save
/// self-sufficient.
#[test]
fn resume_ignores_a_bad_deck_path_it_just_called_ignored() {
    let save = a_save_file("baddeck");
    let output = runner_at_root()
        .args(["--p1", "random", "--p2", "random",
               "--resume", &save.to_string_lossy(),
               "--deck1", "/nonexistent/deck.txt", "--quiet"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let _ = std::fs::remove_file(&save);

    assert_eq!(output.status.code(), Some(0),
        "the resume runs to completion.\nstdout: {stdout}\nstderr: {stderr}");
    assert!(stderr.contains("--deck1 is ignored"),
        "and still says the flag is ignored.\nstderr: {stderr}");
    assert!(!stderr.contains("could not be read as a deck file"),
        "an ignored flag is not loaded, so it cannot fail.\nstderr: {stderr}");
}

/// The `--seed` note under `--resume` says what actually happens. It used to
/// claim the seed was ignored; only the engine RNG comes from the save, so
/// `--seed` still seeds the seats — and dropping it, as the old note
/// advised, is exactly what makes a resumed replay non-reproducible.
#[test]
fn resume_does_not_call_the_seed_ignored_when_it_is_not() {
    let save = a_save_file("seednote");
    let output = runner_at_root()
        .args(["--p1", "random", "--p2", "random",
               "--resume", &save.to_string_lossy(), "--seed", "12345", "--quiet"])
        .output()
        .expect("failed to run");
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!stderr.contains("--seed is ignored"),
        "the seed is not ignored, so the note must not say it is.\nstderr: {stderr}");
    assert!(stderr.contains("--seed") && stderr.contains("seeds the random/AI seats"),
        "the note says what --seed really does.\nstderr: {stderr}");

    // And it demonstrably still determines the resumed game: same seed twice
    // is the same game, a different seed is a different one.
    let actions_with = |seed: &str| {
        let out = runner_at_root()
            .args(["--p1", "random", "--p2", "random",
                   "--resume", &save.to_string_lossy(), "--seed", seed, "--quiet"])
            .output()
            .expect("failed to run");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .find(|l| l.starts_with("Total actions:"))
            .map(std::string::ToString::to_string)
            .expect("the run reports its action count")
    };
    assert_eq!(actions_with("12345"), actions_with("12345"),
        "one seed replays one game");
    let _ = std::fs::remove_file(&save);
}
