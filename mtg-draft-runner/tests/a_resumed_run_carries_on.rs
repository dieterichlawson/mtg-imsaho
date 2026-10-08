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

fn have_python() -> bool {
    std::process::Command::new("python3").arg("--version")
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().is_ok_and(|s| s.success())
}

/// A scratch directory with the stub seat in it, and a way to run the
/// runner against it.
struct Pod {
    dir: PathBuf,
    bin: PathBuf,
}

impl Pod {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("mtg-draft-resume-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let bin = stub(&dir);
        Self { dir, bin }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    /// `--players 2 --best-of 1 --seed 7` unless `extra` says otherwise
    /// (a later flag wins).
    fn run(&self, extra: &[&str], counter: &str, log: &str) -> std::process::Output {
        std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
            .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7"])
            .args(["--log", self.path(log).to_str().unwrap()])
            .args(extra)
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .env("CLAUDE_CODE_BIN", &self.bin)
            .env("CALLS", self.path(counter))
            .output()
            .expect("the runner runs")
    }

    /// A whole run's snapshot, at `name`.
    fn finished_snapshot(&self, name: &str) -> serde_json::Value {
        let snap = self.path(name);
        let first = self.run(&["--save", snap.to_str().unwrap()], "first.calls", "first.log");
        assert!(first.status.success(), "{}", String::from_utf8_lossy(&first.stderr));
        serde_json::from_str(&std::fs::read_to_string(&snap).unwrap()).unwrap()
    }
}

#[test]
fn a_resume_after_the_tournament_replays_nothing_and_says_so() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("carry");
    let snap = pod.path("snap.json");
    let saved = pod.finished_snapshot("snap.json");
    assert!(calls(&pod.path("first.calls")) > 0);
    assert_eq!(saved["decks"].as_array().map(Vec::len), Some(2), "both built decks are in the snapshot");
    assert_eq!(saved["matches"].as_array().map(Vec::len), Some(1), "the one match is in the snapshot");

    let second = pod.run(&["--resume", snap.to_str().unwrap()], "second.calls", "second.log");
    let stderr = String::from_utf8_lossy(&second.stderr);
    assert!(second.status.success(), "{stderr}");
    assert_eq!(calls(&pod.path("second.calls")), 0,
        "the picks, the decks and the match all come from the snapshot — nothing is asked again:\n{stderr}");
    let standings: Vec<&str> = stderr.lines().skip_while(|l| !l.contains("Final Standings")).skip(1).take(2).collect();
    assert_eq!(standings.len(), 2, "{stderr}");
    for row in &standings {
        assert!(row.contains("[1 match from snapshot]"), "a carried result says so on its row: {row}");
    }
    let log = std::fs::read_to_string(pod.path("second.log")).unwrap();
    assert!(log.contains("MATCH FROM SNAPSHOT"), "the log says which match this run did not play");
    assert!(log.contains("DECK FROM SNAPSHOT"), "and which decks it did not build");
}

/// #732: the resumed run's own `--save` was written after the decks and
/// then only when a match of its own finished, so a resume that carried
/// every match left a snapshot holding none of them — and
/// `--resume X --save X` erased them from the only copy.
#[test]
fn a_resumed_runs_own_save_keeps_the_matches_it_carried() {
    if !have_python() {
        eprintln!("skipping: no python3 to run the stub seat with");
        return;
    }
    let pod = Pod::new("resave");
    let snap = pod.path("snap.json");
    pod.finished_snapshot("snap.json");
    let snap = snap.to_str().unwrap();

    let again = pod.run(&["--resume", snap, "--save", snap], "again.calls", "again.log");
    assert!(again.status.success(), "{}", String::from_utf8_lossy(&again.stderr));
    let resaved: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(snap).unwrap()).unwrap();
    assert_eq!(resaved["matches"].as_array().map(Vec::len), Some(1),
        "the carried match is still in the snapshot the resume wrote");
}
