//! The one sentence an operator gets when a draft seat dies tells the truth
//! about how hard the run tried, and says what failed.
//!
//! The fatal used to report the *configured retry budget* where the
//! duration belongs:
//!
//! ```text
//! claude -p draft seat gave up after 1 attempts over 6s
//! ```
//!
//! after sixty seconds of trying (issue #585). The retry loop stops as soon
//! as the next backoff would overshoot the deadline, so the budget and the
//! time actually spent are equal only by coincidence — they diverge with
//! the ratio of the per-call timeout to the budget, and at the shipped
//! defaults (600s budget, 300s timeout) they land close enough to hide it.
//!
//! The record also named no reason. Each attempt's reason goes to stderr as
//! it happens, but `API_FATAL` in the `--log` is the durable one, and
//! carrying only a count meant a draft log alone could not tell a hung CLI
//! from a crashing one.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::time::Instant;

/// A seat that fails at once, with a nameable reason.
fn failing_stub(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(
        &bin,
        r##"#!/usr/bin/env python3
import sys
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
sys.stdin.read()
sys.stderr.write("stub: deliberate failure\n")
sys.exit(17)
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

#[test]
fn the_fatal_reports_the_time_spent_not_the_budget() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-fatal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = failing_stub(&dir);
    let log = dir.join("run.log");

    // A 1s budget against a seat that fails instantly. The first attempt
    // runs with no backoff; the second would have to wait 2s, which
    // overshoots, so the loop gives up having spent almost no time at all.
    // That is the #585 shape in miniature — one attempt, and an elapsed
    // time nowhere near the budget.
    let budget_secs = 1u64;
    let started = Instant::now();
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", budget_secs.to_string())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the runner runs");
    let wall = started.elapsed();
    assert!(!status.success(), "a seat that always fails should stop the run");

    let log = std::fs::read_to_string(&log).unwrap_or_default();
    let fatal = log
        .lines()
        .find(|l| l.contains("\tAPI_FATAL\t"))
        .unwrap_or_else(|| panic!("no API_FATAL record in the log"))
        .to_string();

    // The reason, which the record used to omit entirely: a later reader
    // can tell a crashing CLI from a hung one.
    assert!(
        fatal.contains("last failure:") && fatal.contains("17"),
        "the fatal names no reason, so the log cannot say what failed: {fatal}"
    );

    // The budget is still there — it is the knob to turn — but labelled as
    // the budget rather than presented as the duration.
    assert!(
        fatal.contains(&format!("(budget {budget_secs}s)")),
        "the fatal should name the budget as the budget: {fatal}"
    );

    // One attempt, and it says "attempt", not "attempts".
    assert!(
        fatal.contains("after 1 attempt in "),
        "one attempt should be reported as one, singular: {fatal}"
    );

    // The duration reported is the time spent, which here is far below the
    // budget. Under the old message this read "over 1s".
    let spent: f64 = fatal
        .split(" in ")
        .nth(1)
        .and_then(|rest| rest.split('s').next())
        .and_then(|n| n.trim().parse().ok())
        .unwrap_or_else(|| panic!("no elapsed time in the fatal: {fatal}"));
    assert!(
        spent < budget_secs as f64,
        "the seat gave up before the budget ran out, so the fatal must not \
         report the budget as the time spent: {fatal}"
    );
    assert!(
        spent <= wall.as_secs_f64() + 1.0,
        "the reported time exceeds the whole run's wall clock ({:.1}s): {fatal}",
        wall.as_secs_f64()
    );

    let _ = std::fs::remove_dir_all(&dir);
}
