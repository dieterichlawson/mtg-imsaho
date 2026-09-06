//! Argument handling for the draft runner. Every seat here can be a metered
//! LLM, so a bad invocation must be refused before a single token is spent:
//! `--help` answers instead of drafting, an unrecognized flag or a bad value
//! is a clean one-line `Error:` (never a panic, never a silent default), and
//! a claude-code seat is checked for its CLI up front rather than after the
//! draft has already been billed.
//!
//! These run the mtg-draft-runner binary as a subprocess. None of them can
//! reach a real model: the refusals exit first, one run that gets past
//! validation dies on a missing set file (which main loads before it builds
//! any client), and the log-completeness test at the bottom points
//! `CLAUDE_CODE_BIN` at a stub that blocks at the first pick and is killed
//! once the log holds what it asserts on.

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

// ── log completeness ────────────────────────────────────────────────

/// A stub `claude` that answers the preflight and then blocks, so a draft
/// gets past validation and writes its header and system prompts without
/// ever reaching a real model. The run is killed once the log has what the
/// test needs.
fn blocking_stub(name: &str) -> std::path::PathBuf {
    use std::io::Write;
    let path = std::env::temp_dir()
        .join(format!("mtg-draft-stub-{name}-{}.sh", std::process::id()));
    let mut f = std::fs::File::create(&path).expect("create stub");
    f.write_all(b"#!/bin/sh\ncase \"$1\" in --version) echo '0.0.0 (stub)'; exit 0;; esac\nsleep 300\n")
        .expect("write stub");
    drop(f);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }
    path
}

/// Every seat's draft system prompt reaches the log, tagged with its seat.
///
/// A draft has no `--seed` and cannot be replayed, so its log is the whole
/// record — but the runner logged seat 0's prompt alone, under an untagged
/// `DRAFT SYSTEM PROMPT` label that read as the pod's one prompt. A
/// `--guide-1` seat's instructions appeared nowhere at all, and a reader
/// would reasonably conclude every seat drafted under seat 0's guide
/// (issue #207).
#[test]
#[cfg(unix)]
fn every_seat_s_draft_system_prompt_and_guide_reach_the_log() {
    use std::io::Write;

    let tmp = std::env::temp_dir();
    let pid = std::process::id();
    let stub = blocking_stub("guides");
    let guide0 = tmp.join(format!("mtg-draft-guide0-{pid}.txt"));
    let guide1 = tmp.join(format!("mtg-draft-guide1-{pid}.txt"));
    let log = tmp.join(format!("mtg-draft-guides-{pid}.log"));
    let _ = std::fs::remove_file(&log);
    // Markers that cannot occur in the shared draft rules or card reference.
    std::fs::File::create(&guide0).unwrap()
        .write_all(b"SEAT ZERO GUIDE MARKER: always take the vampire.\n").unwrap();
    std::fs::File::create(&guide1).unwrap()
        .write_all(b"SEAT ONE GUIDE MARKER: always take the werewolf.\n").unwrap();

    // Unlike every other test here, this one has to get far enough to load
    // `data/sets/isd.json`, which is resolved relative to the working
    // directory — and cargo runs tests from the package root, not the
    // workspace root where `data/` lives.
    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("package dir has a workspace parent");

    let mut child = runner()
        .current_dir(workspace_root)
        .args([
            "--model", "cc", "--players", "2", "--best-of", "1", "--quiet",
            "--guide-0", &guide0.to_string_lossy(),
            "--guide-1", &guide1.to_string_lossy(),
            "--log", &log.to_string_lossy(),
        ])
        .env("CLAUDE_CODE_BIN", &stub)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn runner");

    // Packs and the system prompts are written before the first pick, which
    // is where the stub blocks. Poll rather than sleep a fixed span.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let mut logged = String::new();
    while std::time::Instant::now() < deadline {
        logged = std::fs::read_to_string(&log).unwrap_or_default();
        if logged.contains("[Seat 1] DRAFT SYSTEM PROMPT") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = child.kill();
    let _ = child.wait();
    for path in [&stub, &guide0, &guide1, &log] {
        let _ = std::fs::remove_file(path);
    }

    assert!(logged.contains("[Seat 0] DRAFT SYSTEM PROMPT"),
        "seat 0's prompt is tagged with its seat:\n{logged}");
    assert!(logged.contains("[Seat 1] DRAFT SYSTEM PROMPT"),
        "seat 1's prompt is logged at all:\n{logged}");
    assert!(logged.contains("SEAT ZERO GUIDE MARKER"),
        "seat 0's guide text is in the log:\n{logged}");
    assert!(logged.contains("SEAT ONE GUIDE MARKER"),
        "seat 1's guide text is in the log — the whole defect:\n{logged}");
    // The header says which file each seat drafted under, so the guides are
    // identifiable even when the log is read on another machine.
    assert!(logged.contains("Seat 0 guide:") && logged.contains("Seat 1 guide:"),
        "the header names each seat's guide file:\n{logged}");
}
