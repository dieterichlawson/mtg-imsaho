//! A run that stops still says what it spent, and a resumed run says which
//! part of the draft its numbers are the cost of.
//!
//! `print_usage_summary` was called once, from the very end of `main`,
//! after the standings. Every other way a run can end — a seat's fatal, a
//! worker panic, a config error — skipped it, so a draft split across
//! resumes could not be costed at all (issue #578).
//!
//! The runs that died published no number anywhere, on stderr or in the
//! `--log`, although every `claude -p` call they made was paid for.
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A `claude -p` stand-in that answers the first `MTG_TEST_FAIL_AFTER`
/// calls of this run and then exits non-zero. The counter is a file in
/// `MTG_TEST_STUB_DIR`, so it is shared across the parallel seat workers.
fn counting_stub(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(
        &bin,
        r##"#!/usr/bin/env python3
import json, os, sys, fcntl
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
d = os.environ["MTG_TEST_STUB_DIR"]

def bump(path):
    fd = os.open(path, os.O_RDWR | os.O_CREAT, 0o644)
    fcntl.flock(fd, fcntl.LOCK_EX)
    v = int((os.read(fd, 64).decode() or "0").strip() or "0") + 1
    os.lseek(fd, 0, 0); os.ftruncate(fd, 0); os.write(fd, str(v).encode())
    fcntl.flock(fd, fcntl.LOCK_UN); os.close(fd)
    return v

n = bump(os.path.join(d, "counter"))
sys.stdin.read()
if n > int(os.environ["MTG_TEST_FAIL_AFTER"]):
    sys.stderr.write("stub: deliberate failure on call %d\n" % n); sys.exit(9)

def argval(f):
    return argv[argv.index(f) + 1] if f in argv else None

schema = json.loads(argval("--json-schema") or "{}")
props = schema.get("properties", {})
if "pick" in props:
    enum = props["pick"].get("enum", [0])
    out = {"thoughts": "stub pick", "pick": enum[len(enum) // 2] if enum else 0}
elif "maindeck" in props:
    names = list(props["maindeck"].get("properties", {}).keys())
    out = {"maindeck": {nm: 1 for nm in names[:23]}, "lands": {"Island": 9, "Swamp": 8}}
else:
    out = {}
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
    "session_id": argval("--resume") or argval("--session-id") or "stub-session",
    "result": json.dumps(out), "structured_output": out,
    "usage": {"input_tokens": 100, "output_tokens": 200,
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

/// One draft run against the stub. Returns (exit code, stderr).
fn run(dir: &Path, bin: &Path, fail_after: u32, log: &Path, save: &Path, resume: bool)
    -> (Option<i32>, String)
{
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"));
    cmd.args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .args(["--save", save.to_str().unwrap()]);
    if resume {
        cmd.args(["--resume", save.to_str().unwrap()]);
    }
    let out = cmd
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", bin)
        .env("MTG_TEST_STUB_DIR", dir)
        .env("MTG_TEST_FAIL_AFTER", fail_after.to_string())
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", "1")
        .env("MTG_CLAUDE_CODE_TIMEOUT_SECS", "30")
        .output()
        .expect("the runner runs");
    (out.status.code(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// The `Total: N calls` the summary reports.
fn total_calls(summary: &str) -> u64 {
    summary
        .lines()
        .find_map(|l| l.trim().strip_prefix("Total: "))
        .and_then(|rest| rest.split(' ').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("no `Total: N calls` line in the summary:\n{summary}"))
}

#[test]
fn a_run_that_dies_reports_the_calls_it_paid_for() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-578-a-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = counting_stub(&dir);
    let log = dir.join("run.log");
    let save = dir.join("save.json");

    // Twenty answered picks, then a seat that cannot pick at all. Pack 1 is
    // fifteen rounds of two seats, so this stops partway through it.
    let (code, stderr) = run(&dir, &bin, 20, &log, &save, false);
    assert_eq!(code, Some(1), "a seat that always fails should stop the run:\n{stderr}");

    assert!(
        stderr.contains("=== Token Usage"),
        "a run that died published no account of the calls it paid for:\n{stderr}"
    );
    // And it says it is not a whole run, so its numbers are not read as one.
    assert!(
        stderr.contains("=== Token Usage (run stopped early) ==="),
        "the stopped run's table is labelled exactly like a finished run's:\n{stderr}"
    );
    let spent = total_calls(&stderr);
    assert!(
        spent > 0,
        "the run answered 20 picks before it died but reports {spent} calls:\n{stderr}"
    );

    // The durable copy too: the terminal does not outlive the run, the log does.
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        text.contains("TOKEN USAGE"),
        "the stopped run's cost is in no TOKEN USAGE record in the log"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
