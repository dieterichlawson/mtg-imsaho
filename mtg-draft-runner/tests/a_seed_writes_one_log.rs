//! Two runs of one `--seed` write the same log.
//!
//! #402 made a seeded draft replay its games and not only its packs, and it
//! does: the packs, the picks, the decks, the opening hands, every decision
//! and the standings come back identical. The *record* of the run did not.
//! From the `DECK BUILDING` banner on, the log was written inline from
//! parallel workers with no ordering, so two runs of one seed emitted the
//! same lines in whatever order the scheduler produced — 1,047 differing
//! hunks over 72,114 lines at four seats, with `diff <(sort a) <(sort b)`
//! empty. That makes `diff` of two seeded logs, the one cheap check of "did
//! this seed replay?", report thousands of differences that mean nothing,
//! which is as useless as reporting none when there is a real one (#541).
//!
//! What may still differ is what genuinely varies between two runs of one
//! seed and is now written down for that reason: the wall-clock timestamp,
//! and the `claude -p` session ids (#542).
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A seat that is a pure function of (schema, prompt): no counters, no
/// clock, no randomness. Any nondeterminism left is the runner's.
fn deterministic_seat(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(&bin, STUB).unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

const STUB: &str = r##"#!/usr/bin/env python3
import sys, json, hashlib
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
msg = sys.stdin.read(); sid = ""; sc = None
for i, a in enumerate(argv):
    if a in ("--session-id", "--resume") and i + 1 < len(argv): sid = argv[i + 1]
    if a == "--json-schema" and i + 1 < len(argv): sc = argv[i + 1]
H = int(hashlib.sha256(((sc or "") + msg).encode()).hexdigest(), 16)
def fill(s, seed):
    if "enum" in s:
        e = s["enum"]; return e[seed % len(e)]
    t = s.get("type")
    if t == "string": return "t"
    if t == "boolean": return seed % 2 == 0
    if t in ("integer", "number"): return s.get("minimum", 0)
    if t == "array":
        p = list(s.get("items", {}).get("enum", [0]))
        lo, hi = s.get("minItems"), s.get("maxItems")
        if lo is None and hi is None: k = len(p)
        else:
            lo = lo or 0; hi = min(hi if hi is not None else len(p), len(p))
            k = lo + seed % (max(hi - lo, 0) + 1)
        return sorted(p, key=lambda v: hashlib.sha256((str(v) + str(seed)).encode()).hexdigest())[:k]
    if t == "object":
        pr = s.get("properties", {}); rq = s.get("required", list(pr))
        return {k: fill(pr[k], int(hashlib.sha256((k + str(seed)).encode()).hexdigest(), 16))
                for k in sorted(pr) if k in rq}
    return "t"
sch = json.loads(sc) if sc else {}
if "maindeck" in sch.get("properties", {}):
    n = sorted(sch["properties"]["maindeck"].get("properties", {}))
    o = {"maindeck": {c: 1 for c in n[:23]}, "lands": {"Island": 9, "Swamp": 8}}
else:
    o = fill(sch, H)
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": sid, "result": json.dumps(o), "structured_output": o,
                  "usage": {"input_tokens": 10, "output_tokens": 2,
                            "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0}}))
"##;

/// The log with its wall-clock column masked — the one field nothing claims
/// a seed controls.
fn masked(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| {
            let is_header = l.len() > 24
                && l.as_bytes()[..10].iter().all(|b| b.is_ascii_digit() || *b == b'-')
                && l.as_bytes()[10] == b' ';
            if is_header {
                match l.find('\t') {
                    Some(i) => format!("TS{}", &l[i..]),
                    None => l.to_string(),
                }
            } else {
                l.to_string()
            }
        })
        .collect()
}

fn run(dir: &Path, bin: &Path, name: &str) -> PathBuf {
    let log = dir.join(format!("{name}.log"));
    let _ = std::fs::remove_file(&log);
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        // Four seats: the deck build runs four workers at once AND the
        // tournament round is two concurrent matches, which is where the
        // interleave was worst.
        .args(["--model", "cc", "--players", "4", "--best-of", "1", "--seed", "41", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the runner runs");
    assert!(status.success(), "run {name} should finish cleanly: {status}");
    log
}

#[test]
fn two_runs_of_one_seed_write_the_same_log() {
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

    let dir = std::env::temp_dir().join(format!("mtg-draft-replay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = deterministic_seat(&dir);

    let a = masked(&run(&dir, &bin, "a"));
    let b = masked(&run(&dir, &bin, "b"));

    assert!(a.len() > 1000, "the fixture should produce a substantial log, got {}", a.len());
    assert_eq!(a.len(), b.len(), "the two runs did not even record the same number of lines");

    // Line for line, in order. A `SESSION` record holds a fresh uuid per
    // conversation, which is the one thing a seeded run cannot reproduce
    // and is in the log precisely so that it is written down (#542).
    let differing: Vec<(usize, &String, &String)> = a
        .iter()
        .zip(b.iter())
        .enumerate()
        .filter(|(_, (x, y))| x != y)
        .map(|(i, (x, y))| (i, x, y))
        .collect();
    let unexplained: Vec<&(usize, &String, &String)> = differing
        .iter()
        .filter(|(_, x, y)| !(x.contains("\tSESSION") && y.contains("\tSESSION")))
        .collect();

    assert!(
        unexplained.is_empty(),
        "{} of {} lines differ between two runs of seed 41 for a reason other than \
         a session id — the run replays but its record does not, so `diff` of two \
         seeded logs cannot be read. First few: {:?}",
        unexplained.len(),
        a.len(),
        unexplained.iter().take(3).collect::<Vec<_>>()
    );

    let _ = std::fs::remove_dir_all(&dir);
}
