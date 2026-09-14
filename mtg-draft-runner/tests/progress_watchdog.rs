//! A tournament game that stops making progress stops.
//!
//! `mtg-runner` grew a progress watchdog when #462 was fixed. This binary
//! keeps its own copy of the game loop and did not get one, which matters
//! more here and not less: in a tournament every seat is an LLM seat, so
//! "50,000 `claude -p` subprocesses spent re-asking one question" is the
//! ordinary case rather than the worst one. A seat that answered every
//! array-valued slot with `[]` spun for 2,724 identical rejections in 230
//! seconds on turn 19, wrote a 16 MB log, printed nothing at all on stderr,
//! and never finished (issue #488).
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A `CLAUDE_CODE_BIN` seat whose calls all succeed and whose array answers
/// are all empty — the legal-no-op shape #399 and #462 describe. Everything
/// else is filled from the schema, so the draft and the deck build go
/// through and the run reaches its tournament.
fn empty_arrays_seat(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(&bin, r##"#!/usr/bin/env python3
import hashlib, json, sys
argv = sys.argv[1:]
if argv and argv[0] == "--version":
    print("1.0.0 (stub)"); sys.exit(0)
def flag(n):
    for i, a in enumerate(argv):
        if a == n and i + 1 < len(argv): return argv[i + 1]
    return None
message = sys.stdin.read()
schema = json.loads(flag("--json-schema") or "{}")
props = schema.get("properties") or {}
sid = flag("--session-id") or flag("--resume") or "s"
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
    for n, s in (props["maindeck"].get("properties") or {}).items():
        take = min(max(s.get("enum", [0])), max(0, 23 - total)); md[n] = take; total += take
    out = {"thoughts": "stub", "maindeck": md,
           "lands": {"Plains": 4, "Island": 4, "Swamp": 3, "Mountain": 3, "Forest": 3}}
else:
    out = {k: fill(k, v) for k, v in props.items()}
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": sid, "usage": {"input_tokens": 10, "output_tokens": 5,
                  "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0},
                  "structured_output": out, "result": json.dumps(out)}))
"##).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    bin
}

#[test]
fn a_tournament_game_that_stops_moving_is_forfeited_and_said_so() {
    // The stub seat is a Python script; without an interpreter there is
    // nothing to drive and the defect cannot be reproduced either way.
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

    let dir = std::env::temp_dir().join(format!("mtg-draft-watchdog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = empty_arrays_seat(&dir);

    // Whether a seeded run REACHES the stall -- a min-1 slot answered with
    // `[]` while the board is frozen -- depends on which deck the draft
    // builds and which actions the seat takes, and the seat takes them by
    // hashing the prompt it was handed. So a pure wording change anywhere
    // in `mtg-player`'s prompts re-rolls the whole game: seed 101 stalled
    // when this was written and plays out cleanly since #491 widened four
    // prompts. Pinning one seed therefore pins the wrong thing -- it fails
    // when a prompt is reworded and says "the game loop is unbounded
    // again", which is not what happened.
    //
    // So: walk a few seeds and assert the watchdog's behaviour on the
    // first one that stalls. About a quarter of seeds do. Losing ALL of
    // them is still a failure, because then the fixture no longer
    // exercises what it is for and a human has to find a seed that does.
    const SEEDS: [&str; 5] = ["107", "108", "101", "102", "103"];
    let mut played_out: Vec<&str> = Vec::new();

    for seed in SEEDS {
        let log = dir.join(format!("loop-{seed}.log"));
        let errfile = dir.join(format!("stderr-{seed}.txt"));
        let started = std::time::Instant::now();
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
            .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", seed, "-q"])
            .args(["--log", log.to_str().unwrap()])
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
            .env("CLAUDE_CODE_BIN", &bin)
            .stdout(std::process::Stdio::null())
            .stderr(std::fs::File::create(&errfile).unwrap())
            .spawn()
            .expect("the runner runs");

        // The run has to come back on its own. Given a deadline of our own,
        // so a regression is a failed assertion rather than a suite that
        // hangs — which is exactly what this defect does to an operator.
        let deadline = started + std::time::Duration::from_secs(240);
        let status = loop {
            match child.try_wait().expect("wait") {
                Some(status) => break status,
                None if std::time::Instant::now() > deadline => {
                    let _ = child.kill();
                    panic!(
                        "seed {seed}: the tournament did not finish in 240s: the game loop \
                         is unbounded again (stderr: {})",
                        std::fs::read_to_string(&errfile).unwrap_or_default()
                    );
                }
                None => std::thread::sleep(std::time::Duration::from_millis(200)),
            }
        };

        assert!(status.success(), "seed {seed}: the run should finish, not fail: {status}");

        let stderr = std::fs::read_to_string(&errfile).unwrap_or_default();
        if !stderr.contains("stopped making progress") {
            played_out.push(seed);
            continue;
        }

        // Silence is the other half of the defect: the operator saw an
        // empty stderr for the whole episode.
        assert!(stderr.contains("forfeit"), "seed {seed} stderr: {stderr}");
        assert!(stderr.contains("=== Forfeited Games ==="), "seed {seed} stderr: {stderr}");

        // And the run's record says which game was not played out.
        let logged = std::fs::read_to_string(&log).unwrap_or_default();
        assert!(logged.contains("STALLED"),
            "seed {seed}: the log should carry the forfeited game");

        let _ = std::fs::remove_dir_all(&dir);
        return;
    }

    panic!(
        "none of {SEEDS:?} reached a stall (all played out: {played_out:?}), so this test \
         no longer exercises the watchdog at all. Find a seed whose tournament stalls with \
         the empty-arrays seat and put it at the front of SEEDS."
    );
}
