//! A replayed pick whose retries run out stops the replay with one line
//! naming why (#1009).
//!
//! `mtg-draft-runner` (#218) and `mtg-draft-server` turn a seat's spent
//! retry budget into one line. `mtg-draft-replay` installed nothing, so it
//! died of a thread panic and a backtrace, and `draft-replay.py`, which
//! keeps the last stderr line as the row's reason, kept the backtrace's
//! footer ("note: Some details are omitted, ...") in every such row.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

#[test]
fn a_seat_that_gives_up_stops_the_replay_with_its_reason() {
    if !Command::new("python3").arg("--version").output().is_ok_and(|o| o.status.success()) {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-replay-fatal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // A seat that never answers in shape.
    let stub = dir.join("bad-stub.py");
    std::fs::write(&stub, r#"#!/usr/bin/env python3
import json, sys
if sys.argv[1:2] == ["--version"]: print("1.0.0"); sys.exit(0)
sys.stdin.read()
print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "result": "I would rather not choose.",
  "session_id": "x", "usage": {"input_tokens": 10, "output_tokens": 5, "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}))
"#).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let positions = dir.join("one.jsonl");
    std::fs::write(&positions, format!("{}\n", serde_json::json!({"pack_number": 1, "pick_index": 1, "seat": 0,
        "available": ["Claustrophobia", "Kessig Wolf", "Spidery Grasp"], "pool": []}))).unwrap();

    let out = Command::new(env!("CARGO_BIN_EXE_mtg-draft-replay"))
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", &stub)
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", "3")
        .env("RUST_BACKTRACE", "1")
        .args(["--positions", positions.to_str().unwrap(), "--model", "cc"])
        .output()
        .expect("the replay runs");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(!stderr.contains("panicked") && !stderr.contains("stack backtrace"), "a raw panic: {stderr}");
    let last = stderr.trim().lines().last().unwrap_or("");
    assert!(last.starts_with("mtg-draft-replay: ") && last.contains("line 1") && last.contains("gave up"),
        "the last line names the reason: {stderr}");

    // And the wrapper keeps that line as the row's reason.
    let rows = dir.join("rows");
    let py = Command::new("python3")
        .current_dir(repo_root())
        .env("CLAUDE_CODE_BIN", &stub)
        .env("MTG_DRAFT_RETRY_BUDGET_SECS", "3")
        .env("RUST_BACKTRACE", "1")
        .args(["scripts/decision-eval/draft-replay.py", "--positions", positions.to_str().unwrap(),
            "--out", rows.to_str().unwrap(), "--name", "x", "--model", "cc", "--guide", "none",
            "--bin", env!("CARGO_BIN_EXE_mtg-draft-replay")])
        .output()
        .expect("draft-replay.py runs");
    let row = std::fs::read_to_string(rows.join("x.jsonl"))
        .unwrap_or_else(|e| panic!("no row ({e}): {}", String::from_utf8_lossy(&py.stderr)));
    let row: serde_json::Value = serde_json::from_str(row.lines().next().unwrap()).unwrap();
    let err = row["error"].as_str().unwrap_or("");
    assert!(err.contains("gave up") && !err.contains("Some details are omitted"), "{row}");
    let _ = std::fs::remove_dir_all(&dir);
}
