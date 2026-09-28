//! A run whose standings record no match must say so.
//!
//! `--players 1` is a supported configuration: `count()` refuses only 0, and
//! `Tournament::total_rounds` returns 0 for a 1-seat pod deliberately. But
//! nothing said the tournament had been skipped — the log printed an empty
//! `TOURNAMENT` header straight into `FINAL STANDINGS`, and the one seat was
//! ranked `0-0`, which is the row for a seat that played a match and went
//! even. That is #486's argument ("without the marker a seat that sat out a
//! round reads exactly like a seat that beat somebody") applied to a whole
//! tournament that never happened (issue #608).
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A seat that answers every prompt from its schema. It never has to play a
/// game here: the point of the test is the phase after the draft.
fn stub_seat(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(
        &bin,
        r##"#!/usr/bin/env python3
import hashlib, json, os, sys
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
def flag(n):
    for i, a in enumerate(argv):
        if a == n and i + 1 < len(argv): return argv[i + 1]
    return None
message = sys.stdin.read()
props = (json.loads(flag("--json-schema") or "{}").get("properties") or {})
def h(*p): return int(hashlib.sha256("\x00".join(map(str, p)).encode()).hexdigest(), 16)
def fill(name, spec):
    t = spec.get("type")
    if "enum" in spec:
        v = spec["enum"]; return v[h(message, name) % len(v)] if v else None
    if t == "string": return "stub"
    if t in ("integer", "number"): return 0
    if t == "boolean": return bool(h(message, name) % 2)
    if t == "array": return []
    if t == "object" or "properties" in spec:
        return {k: fill(name + "." + k, v) for k, v in (spec.get("properties") or {}).items()}
    return "stub"
if "maindeck" in props and "lands" in props:
    md, total = {}, 0
    for nm, s in (props["maindeck"].get("properties") or {}).items():
        take = min(max(s.get("enum", [0])), max(0, 23 - total)); md[nm] = take; total += take
    out = {"maindeck": md, "lands": {"Plains": 4, "Island": 4, "Swamp": 3, "Mountain": 3, "Forest": 3}}
    if "thoughts" in props: out["thoughts"] = "stub"
else:
    out = {k: fill(k, v) for k, v in props.items()}
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": flag("--session-id") or flag("--resume") or "s",
                  "usage": {"input_tokens": 10, "output_tokens": 5},
                  "structured_output": out, "result": json.dumps(out)}))
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn a_one_seat_pod_does_not_report_an_unplayed_standing_as_a_result() {
    let have_python = std::process::Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !have_python {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }

    let dir = std::env::temp_dir().join(format!("mtg-draft-solo-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = stub_seat(&dir);
    let log = dir.join("run.log");

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "1", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .output()
        .expect("the runner runs");
    assert!(out.status.success(), "a 1-seat run still finishes: {}", out.status);

    let log_text = std::fs::read_to_string(&log).unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);

    // The standings are there, and so is the fact that nothing was played to
    // earn them — on both surfaces that show a standing.
    assert!(log_text.contains("FINAL STANDINGS"), "the standings block is printed");
    assert!(
        log_text.contains("NO ROUNDS"),
        "the log ran TOURNAMENT straight into FINAL STANDINGS with nothing in between, so \
         a 0-0 standing nobody played for reads as a match that was drawn:\n{}",
        log_text
            .lines()
            .skip_while(|l| !l.contains("TOURNAMENT"))
            .take(8)
            .collect::<Vec<_>>()
            .join("\n")
    );
    // `grep WARN` is how the other unearned standings are found (#195).
    assert!(
        log_text.lines().any(|l| l.contains("WARN") && l.contains("NO ROUNDS")),
        "findable the way a substituted pick is findable"
    );
    assert!(
        stderr.contains("No Tournament"),
        "the operator's surface says it too, next to the standings it qualifies:\n{stderr}"
    );

    // And a pod that does play keeps quiet about it.
    let log2 = dir.join("two.log");
    let out2 = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log2.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .output()
        .expect("the runner runs");
    assert!(out2.status.success(), "{}", out2.status);
    let log2_text = std::fs::read_to_string(&log2).unwrap();
    assert!(!log2_text.contains("NO ROUNDS"), "a 2-seat pod plays a round");
    assert!(
        !String::from_utf8_lossy(&out2.stderr).contains("No Tournament"),
        "and does not claim otherwise"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
