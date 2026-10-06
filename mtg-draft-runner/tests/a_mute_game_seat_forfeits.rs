//! A game seat whose backend never answers forfeits its match (issue #587).
//!
//! It used to retry three times over six seconds and then play the rest of
//! the tournament on fallbacks — a dead CLI reported as a seat that kept
//! answering `{}`. The owner's decision: wait the same ten-minute budget the
//! draft seat waits, and when it is spent, forfeit that seat's match through
//! the stall path; every other match keeps running and the run exits 0.
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// Answers the draft (picks and the deck build) and fails every game call.
const STUB: &str = r##"#!/usr/bin/env python3
import sys, json
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
msg = sys.stdin.read(); sid = ""; sc = None
for i, a in enumerate(argv):
    if a in ("--session-id", "--resume") and i + 1 < len(argv): sid = argv[i + 1]
    if a == "--json-schema" and i + 1 < len(argv): sc = argv[i + 1]
sch = json.loads(sc) if sc else {}
props = sch.get("properties", {})
if "maindeck" in props:
    n = sorted(props["maindeck"].get("properties", {}))
    o = {"maindeck": {c: 1 for c in n[:23]}, "lands": {"Island": 9, "Swamp": 8}}
elif "pick" in props:
    o = {"thoughts": "t", "pick": 0}
else:
    print("boom", file=sys.stderr); sys.exit(3)
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": sid, "result": json.dumps(o), "structured_output": o,
                  "usage": {"input_tokens": 10, "output_tokens": 2,
                            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}))
"##;

fn stub(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(&bin, STUB).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn a_seat_whose_backend_gave_up_forfeits_its_match_and_the_run_goes_on() {
    if !std::process::Command::new("python3").arg("--version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())
    {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-mute-game-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = stub(&dir);
    let log = dir.join("run.log");
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "3", "--seed", "5"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("MTG_GAME_RETRY_BUDGET_SECS", "1")
        .output()
        .expect("the runner runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "a dead game seat is not a fatal for the run:\n{stderr}");

    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(logged.contains("never answered within its retry budget"),
        "the log says the backend never answered:\n{stderr}");
    // The seat that gave up lost the whole match by forfeit — two games of a
    // best-of-three, the second never played — and the standings say so.
    let standings: Vec<&str> = stderr.lines().skip_while(|l| !l.contains("Final Standings")).skip(1).take(2).collect();
    assert!(standings.iter().any(|r| r.contains("[2 games forfeited]")),
        "the forfeiting seat's row says so: {standings:?}\n{stderr}");
    assert!(stderr.contains("=== Forfeited Games ==="), "{stderr}");
}

/// Issue #685: the same run with nobody reading stderr. The seat's failed
/// calls print retry lines, and `eprintln!` to a pipe whose reader has gone
/// panicked the match worker, which ended the whole tournament with exit 1
/// and no reason in the log.
#[test]
fn a_seat_that_fails_with_nobody_reading_stderr_does_not_end_the_run() {
    if !std::process::Command::new("python3").arg("--version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())
    {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-stderr-gone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = stub(&dir);
    let log = dir.join("run.log");
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "91032"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("MTG_GAME_RETRY_BUDGET_SECS", "1")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the runner starts");
    drop(child.stderr.take());
    let status = child.wait().expect("the runner ends");
    assert!(status.success(), "the run ends normally with nobody reading stderr: {status}");
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(logged.contains("FINAL STANDINGS"), "the tournament finished");
}
