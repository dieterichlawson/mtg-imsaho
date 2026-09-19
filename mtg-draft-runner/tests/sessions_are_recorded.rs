//! Every `claude -p` conversation a run has is written down.
//!
//! A `cc` seat mints a fresh uuid per conversation, passes it as
//! `--session-id` and `--resume`s it for the rest of that conversation. A
//! 2-seat run mints one per draft seat plus one per seat per game, and not
//! one of them appeared in the `--log`, in a `--save` snapshot, or on
//! stderr. That id is the only handle to the CLI's own stored transcript of
//! the conversation, so after a run ended the transcript existed and was
//! unfindable — and "were this seat's calls really one session?", the
//! property #481 was about, could only be answered by re-running the whole
//! draft under a wrapper rather than by reading what the run left behind
//! (issue #542).
#![cfg(unix)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A seat that answers every prompt from its schema, and records the
/// session id it was handed on each call — one file per distinct session,
/// which is the run's conversations as the CLI saw them.
fn recording_seat(dir: &Path) -> PathBuf {
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
schema = json.loads(flag("--json-schema") or "{}")
props = schema.get("properties") or {}
sid = flag("--session-id") or flag("--resume")
if sid:
    open(os.path.join(os.environ["STUB_SESSIONS"], sid), "a").close()
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
                  "session_id": sid or "s", "usage": {"input_tokens": 10, "output_tokens": 5,
                  "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0},
                  "structured_output": out, "result": json.dumps(out)}))
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

/// Every uuid in a string, as a set.
fn uuids(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_ascii_hexdigit() || c == '-'))
        .filter(|w| {
            w.len() == 36
                && w.as_bytes()
                    .iter()
                    .enumerate()
                    .all(|(i, b)| if matches!(i, 8 | 13 | 18 | 23) { *b == b'-' } else { b.is_ascii_hexdigit() })
        })
        .map(str::to_string)
        .collect()
}

#[test]
fn the_log_names_every_session_the_run_opened() {
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

    let dir = std::env::temp_dir().join(format!("mtg-draft-sessions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let seen = dir.join("sessions");
    std::fs::create_dir_all(&seen).unwrap();
    let bin = recording_seat(&dir);
    let log = dir.join("run.log");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("STUB_SESSIONS", &seen)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the runner runs");
    assert!(status.success(), "the stub answers everything, so the run finishes: {status}");

    // What the CLI was actually asked to be: one file per distinct id.
    let opened: BTreeSet<String> = std::fs::read_dir(&seen)
        .unwrap()
        .filter_map(|e| Some(e.ok()?.file_name().to_str()?.to_string()))
        .collect();
    assert!(
        opened.len() >= 4,
        "expected at least a session per draft seat and per game seat, saw {}: {opened:?}",
        opened.len()
    );

    let log_text = std::fs::read_to_string(&log).unwrap();
    let recorded = uuids(
        &log_text
            .lines()
            .filter(|l| l.contains("\tSESSION"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    let missing: Vec<&String> = opened.difference(&recorded).collect();
    assert!(
        missing.is_empty(),
        "{} of the run's {} `claude -p` sessions appear nowhere in the log, so the \
         CLI's transcript of them cannot be found again: {missing:?}",
        missing.len(),
        opened.len()
    );

    // And a session is recorded once, when it opens — not on every call.
    let session_lines = log_text.lines().filter(|l| l.contains("\tSESSION")).count();
    assert_eq!(
        session_lines,
        opened.len(),
        "a SESSION record is one per conversation, not one per call"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
