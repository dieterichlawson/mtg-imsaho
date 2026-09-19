//! When a run stops, it says which seat stopped it, and says so at once.
//!
//! The pick and deck-build scopes used to let the *join loop* discover a
//! failure: `handles.into_iter().enumerate()` walks seats 0, 1, 2, … and the
//! first `Err` ends the process. Three consequences, all of them about the
//! run's account of itself rather than about the draft (issue #539):
//!
//! 1. Blame went to the lowest-numbered failed seat, not the one that broke.
//!    That is frequently the derived failure — a seat that merely timed out
//!    takes the headline while the seat that failed outright, first, and for
//!    a nameable reason goes unmentioned.
//! 2. The report waited on healthy seats. A fatal in seat N was not printed
//!    until seats 0..N-1 returned, so a healthy but slow seat 0 held the
//!    whole run silent — with shipped defaults, up to ten minutes of a run
//!    that was already dead.
//! 3. The log could not make up the difference: `API_FATAL` carried a thread
//!    id and no seat, so after a real run stopped there was no way back from
//!    the log to the seat whose account or session was the broken one.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// A `CLAUDE_CODE_BIN` seat whose behaviour is per-seat, read out of the
/// pick prompt, which states the seat verbatim.
///
/// `STUB_FAIL_SEAT` exits non-zero at once. `STUB_SLOW_SEAT` sleeps
/// `STUB_SLOW_SECS` and then answers normally. Everything else picks card 0.
fn per_seat_stub(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(
        &bin,
        r##"#!/usr/bin/env python3
import json, os, re, sys, time
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
def flag(n):
    for i, a in enumerate(argv):
        if a == n and i + 1 < len(argv): return argv[i + 1]
    return None
message = sys.stdin.read()
m = re.search(r"You are seat (\d+) of", message)
seat = m.group(1) if m else "?"
if os.environ.get("STUB_FAIL_SEAT") == seat:
    sys.stderr.write("stub: seat %s deliberate failure\n" % seat)
    sys.exit(17)
if os.environ.get("STUB_SLOW_SEAT") == seat:
    time.sleep(float(os.environ.get("STUB_SLOW_SECS", "40")))
out = {"pick": 0}
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": flag("--session-id") or flag("--resume") or "s",
                  "structured_output": out, "result": json.dumps(out),
                  "usage": {"input_tokens": 10, "output_tokens": 2,
                            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}))
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

fn have_python() -> bool {
    std::process::Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

struct Run {
    stderr: String,
    log: String,
    elapsed: Duration,
    ok: bool,
}

fn draft(dir: &Path, envs: &[(&str, &str)]) -> Run {
    let bin = per_seat_stub(dir);
    let log = dir.join("run.log");
    let errfile = dir.join("stderr.txt");
    let started = Instant::now();
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"));
    cmd.args(["--model", "cc", "--players", "4", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::fs::File::create(&errfile).unwrap());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let status = cmd.status().expect("the runner runs");
    Run {
        stderr: std::fs::read_to_string(&errfile).unwrap_or_default(),
        log: std::fs::read_to_string(&log).unwrap_or_default(),
        elapsed: started.elapsed(),
        ok: status.success(),
    }
}

/// The one line that starts with `Error:` — the run's account of what stopped it.
fn error_line(stderr: &str) -> String {
    stderr
        .lines()
        .find(|l| l.trim_start().starts_with("Error:"))
        .unwrap_or("<no Error: line>")
        .trim()
        .to_string()
}

#[test]
fn the_failure_reported_is_the_first_one_not_the_lowest_numbered_seat() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-blame-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Seat 3 fails outright, at once. Seat 0 hangs and only *times out*,
    // later — the derived failure, and the lower seat number.
    let run = draft(
        &dir,
        &[
            ("STUB_FAIL_SEAT", "3"),
            ("STUB_SLOW_SEAT", "0"),
            ("STUB_SLOW_SECS", "600"),
            ("MTG_CLAUDE_CODE_TIMEOUT_SECS", "5"),
            ("MTG_DRAFT_RETRY_BUDGET_SECS", "6"),
        ],
    );

    assert!(!run.ok, "a seat exhausting its retries is fatal");
    let line = error_line(&run.stderr);
    assert!(
        line.contains("seat 3"),
        "the run should name the seat that broke first, not the lowest-numbered \
         one that broke later: {line:?}"
    );
    assert!(
        !line.contains("seat 0"),
        "seat 0 only timed out, after seat 3 had already failed outright: {line:?}"
    );

    // And the log can be read back to the same seat. `API_FATAL` used to
    // carry a bare thread id, which identifies nothing an operator can act
    // on — not the seat, not its account, not its session.
    let fatal: Vec<&str> = run.log.lines().filter(|l| l.contains("API_FATAL")).collect();
    assert!(!fatal.is_empty(), "the run logged no API_FATAL at all");
    assert!(
        fatal.iter().any(|l| l.contains("\tseat 3\t")),
        "no API_FATAL line names seat 3; the log says only which thread it was: {fatal:?}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_healthy_but_slow_seat_does_not_delay_the_report() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-delay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    // Seat 3 is done failing within its 5s budget. Seat 0 is perfectly
    // healthy and takes 60s, which used to be 60s of a dead run showing
    // nothing but `Pack 1 Pick 1/14`.
    const SLOW_SECS: u64 = 60;
    let run = draft(
        &dir,
        &[
            ("STUB_FAIL_SEAT", "3"),
            ("STUB_SLOW_SEAT", "0"),
            ("STUB_SLOW_SECS", &SLOW_SECS.to_string()),
            ("MTG_CLAUDE_CODE_TIMEOUT_SECS", "900"),
            ("MTG_DRAFT_RETRY_BUDGET_SECS", "5"),
        ],
    );

    assert!(!run.ok, "a seat exhausting its retries is fatal");
    assert!(
        error_line(&run.stderr).contains("seat 3"),
        "the failing seat is named: {:?}",
        error_line(&run.stderr)
    );
    // Generous: seat 3 is fatal at ~5s, and the bound only has to separate
    // "reported when it happened" from "waited for seat 0".
    assert!(
        run.elapsed < Duration::from_secs(SLOW_SECS / 2),
        "the run took {:?} to report a failure that happened at ~5s — it waited \
         on healthy seat 0's {SLOW_SECS}s call before saying anything",
        run.elapsed
    );

    let _ = std::fs::remove_dir_all(&dir);
}
