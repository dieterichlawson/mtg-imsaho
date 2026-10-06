//! `--resume` carries a run on from where its snapshot stopped (issue #581).
//!
//! The snapshot used to hold the picks and nothing else, so an interruption
//! after the last pick re-billed every deck build and the whole tournament —
//! about fifteen times the draft's calls at the shipped defaults. It now
//! holds the built decks and every finished match, and a resume takes them
//! instead of paying for them again, and says so on the standings.
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A seat that answers every schema, and counts its calls in `$CALLS`.
const STUB: &str = r##"#!/usr/bin/env python3
import sys, json, os
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
msg = sys.stdin.read(); sid = ""; sc = None
for i, a in enumerate(argv):
    if a in ("--session-id", "--resume") and i + 1 < len(argv): sid = argv[i + 1]
    if a == "--json-schema" and i + 1 < len(argv): sc = argv[i + 1]
with open(os.environ["CALLS"], "a") as f: f.write("x\n")
def fill(s):
    if "enum" in s: return s["enum"][0]
    t = s.get("type")
    if t == "string": return "t"
    if t == "boolean": return False
    if t in ("integer", "number"): return s.get("minimum", 0)
    if t == "array":
        p = list(s.get("items", {}).get("enum", [0])); return p[:s.get("minItems", 0) or 0]
    if t == "object":
        pr = s.get("properties", {}); rq = s.get("required", list(pr))
        return {k: fill(pr[k]) for k in sorted(pr) if k in rq}
    return "t"
sch = json.loads(sc) if sc else {}
if "maindeck" in sch.get("properties", {}):
    n = sorted(sch["properties"]["maindeck"].get("properties", {}))
    o = {"maindeck": {c: 1 for c in n[:23]}, "lands": {"Island": 9, "Swamp": 8}}
else:
    o = fill(sch)
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

fn calls(path: &Path) -> usize {
    std::fs::read_to_string(path).map_or(0, |s| s.lines().count())
}

#[test]
fn a_resume_after_the_tournament_replays_nothing_and_says_so() {
    if !std::process::Command::new("python3").arg("--version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())
    {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let dir = std::env::temp_dir().join(format!("mtg-draft-resume-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = stub(&dir);
    let snap = dir.join("snap.json");
    let run = |extra: &[&str], counter: &Path, log: &str| {
        std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
            .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7"])
            .args(["--log", dir.join(log).to_str().unwrap()])
            .args(extra)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .env("CLAUDE_CODE_BIN", &bin)
            .env("CALLS", counter)
            .output()
            .expect("the runner runs")
    };

    let first_calls = dir.join("first.calls");
    let first = run(&["--save", snap.to_str().unwrap()], &first_calls, "first.log");
    assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
    assert!(calls(&first_calls) > 0);

    let saved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&snap).unwrap()).unwrap();
    assert_eq!(saved["decks"].as_array().map(Vec::len), Some(2), "both built decks are in the snapshot");
    assert_eq!(saved["matches"].as_array().map(Vec::len), Some(1), "the one match is in the snapshot");

    let second_calls = dir.join("second.calls");
    let second = run(&["--resume", snap.to_str().unwrap()], &second_calls, "second.log");
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(second.status.success(), "{stderr}");
    assert_eq!(calls(&second_calls), 0,
        "the picks, the decks and the match all come from the snapshot — nothing is asked again:\n{stderr}");
    let standings: Vec<&str> = stderr.lines().skip_while(|l| !l.contains("Final Standings")).skip(1).take(2).collect();
    assert_eq!(standings.len(), 2, "{stderr}");
    for row in &standings {
        assert!(row.contains("[1 match from snapshot]"), "a carried result says so on its row: {row}");
    }
    let log = std::fs::read_to_string(dir.join("second.log")).unwrap();
    assert!(log.contains("MATCH FROM SNAPSHOT"), "the log says which match this run did not play");
    assert!(log.contains("DECK FROM SNAPSHOT"), "and which decks it did not build");
}
