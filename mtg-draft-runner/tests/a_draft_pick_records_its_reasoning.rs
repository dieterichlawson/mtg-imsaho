//! A `claude -p` draft seat's reasoning reaches the log.
//!
//! #213 found this on the game side and fixed it there: the CLI's result
//! object carries no thinking block, so a schema stripped of `thoughts`
//! leaves the seat's reasoning recorded nowhere at all. The draft crate is
//! the second copy of that protocol and the fix never travelled — its
//! sanitizer had no `keep_thoughts` parameter, so 42 picks and a deck build
//! were recorded with no reasoning in the same log where the tournament
//! recorded 202 THOUGHT lines, and the prompt told the seat the field would
//! be rejected (issue #607).
//!
//! The draft log's `RESPONSE` record is the whole durable account of why a
//! seat took a card, and this crate substitutes a pick quietly when it
//! cannot use an answer (#195, #536) — so with the reasoning erased, a seat
//! that drafted and a seat that never answered differ only in a `WARN`.
#![cfg(unix)]

use std::path::{Path, PathBuf};

/// A seat that answers every prompt from its schema, writes each schema it
/// was handed to `STUB_SCHEMAS`, and puts a recognisable sentence in
/// `thoughts` whenever the schema it is given has the field.
fn reasoning_seat(dir: &Path) -> PathBuf {
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
raw = flag("--json-schema") or "{}"
schema = json.loads(raw)
props = schema.get("properties") or {}
d = os.environ["STUB_SCHEMAS"]
n = len(os.listdir(d))
json.dump({"schema": schema, "system": flag("--system-prompt") or "", "prompt": message},
          open(os.path.join(d, "call_%04d.json" % n), "w"))
def h(*p): return int(hashlib.sha256("\x00".join(map(str, p)).encode()).hexdigest(), 16)
def fill(name, spec):
    t = spec.get("type")
    if "enum" in spec:
        v = spec["enum"]; return v[h(message, name) % len(v)] if v else None
    if t == "string": return "REASONING-" + name
    if t in ("integer", "number"): return 0
    if t == "boolean": return bool(h(message, name) % 2)
    if t == "array": return []
    if t == "object" or "properties" in spec:
        return {k: fill(name + "." + k, v) for k, v in (spec.get("properties") or {}).items()}
    return "REASONING-" + name
if "maindeck" in props and "lands" in props:
    md, total = {}, 0
    for nm, s in (props["maindeck"].get("properties") or {}).items():
        take = min(max(s.get("enum", [0])), max(0, 23 - total)); md[nm] = take; total += take
    out = {"maindeck": md,
           "lands": {"Plains": 4, "Island": 4, "Swamp": 3, "Mountain": 3, "Forest": 3}}
    if "thoughts" in props: out["thoughts"] = "REASONING-thoughts"
else:
    out = {k: fill(k, v) for k, v in props.items()}
print(json.dumps({"type": "result", "subtype": "success", "is_error": False,
                  "session_id": flag("--session-id") or flag("--resume") or "s",
                  "usage": {"input_tokens": 10, "output_tokens": 5,
                  "cache_read_input_tokens": 0, "cache_creation_input_tokens": 0},
                  "structured_output": out, "result": json.dumps(out)}))
"##,
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn the_draft_seat_is_asked_for_its_reasoning_and_the_log_keeps_it() {
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

    let dir = std::env::temp_dir().join(format!("mtg-draft-reasoning-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let schemas = dir.join("schemas");
    std::fs::create_dir_all(&schemas).unwrap();
    let bin = reasoning_seat(&dir);
    let log = dir.join("run.log");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "1", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .env("STUB_SCHEMAS", &schemas)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the runner runs");
    assert!(status.success(), "the stub answers everything, so the run finishes: {status}");

    // Every schema the CLI was handed, split into the draft's and the game's.
    // A pick schema is the one with `pick`; a deck-build schema has
    // `maindeck` and `lands`.
    let calls: Vec<serde_json::Value> = {
        let mut names: Vec<_> = std::fs::read_dir(&schemas)
            .unwrap()
            .filter_map(|e| Some(e.ok()?.path()))
            .collect();
        names.sort();
        names
            .iter()
            .map(|p| serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap())
            .collect()
    };
    let is_draft = |c: &serde_json::Value| {
        let props = &c["schema"]["properties"];
        props.get("pick").is_some() || (props.get("maindeck").is_some() && props.get("lands").is_some())
    };
    let draft: Vec<&serde_json::Value> = calls.iter().filter(|c| is_draft(c)).collect();
    let game: Vec<&serde_json::Value> = calls.iter().filter(|c| !is_draft(c)).collect();
    assert!(
        draft.len() >= 2 * (42 + 1),
        "two seats' 42 picks and a deck build each: saw {} draft calls of {} total",
        draft.len(),
        calls.len()
    );
    assert!(!game.is_empty(), "the tournament ran too, as the comparison the issue makes");

    // 1. The field the schema builders ask for survives the sanitizer.
    let stripped: Vec<usize> = draft
        .iter()
        .enumerate()
        .filter(|(_, c)| c["schema"]["properties"].get("thoughts").is_none())
        .map(|(i, _)| i)
        .collect();
    assert!(
        stripped.is_empty(),
        "{} of {} draft schemas reached `claude -p` with `thoughts` stripped out, so the \
         seat had nowhere to put its reasoning and no thinking channel to put it in \
         (issue #607)",
        stripped.len(),
        draft.len()
    );
    for c in &draft {
        let required = c["schema"]["required"].as_array().expect("required list");
        assert!(
            required.iter().any(|v| v == "thoughts"),
            "asked for, not merely allowed: {}",
            c["schema"]["required"]
        );
    }

    // 2. And the prompt agrees with the schema it is paired with, rather than
    //    telling the seat the key will be rejected.
    for c in &draft {
        let system = c["system"].as_str().expect("a system prompt every call");
        assert!(
            !system.contains("do NOT add a \"thoughts\" key"),
            "the draft prompt still tells the seat not to send the field its schema requires"
        );
        assert!(
            system.contains("\"thoughts\" field"),
            "the draft prompt says where the reasoning goes"
        );
    }

    // 3. The point of all of it: the log holds the reasoning for every pick
    //    and every deck build, which is the only record of why.
    let log_text = std::fs::read_to_string(&log).unwrap();
    let records = |label: &str| -> Vec<String> {
        let mut out = Vec::new();
        let mut current: Option<String> = None;
        for line in log_text.lines() {
            if line.contains("\tINFO\t") || line.contains("\tERROR\t") || line.contains("\tWARN\t") {
                if let Some(body) = current.take() {
                    out.push(body);
                }
                if line.contains(label) {
                    current = Some(String::new());
                }
            } else if let Some(body) = current.as_mut() {
                body.push_str(line);
                body.push('\n');
            }
        }
        out.extend(current);
        out
    };
    let picks = records("] RESPONSE Pack ");
    assert_eq!(picks.len(), 84, "two seats, 42 picks each");
    let silent = picks.iter().filter(|b| !b.contains("REASONING-")).count();
    assert_eq!(
        silent, 0,
        "{silent} of {} pick RESPONSE records hold no reasoning at all — the draft log is \
         the only account of why a seat took a card (issue #607)",
        picks.len()
    );
    let decks = records("] DECK_RESPONSE ");
    assert!(!decks.is_empty(), "the deck builds are logged too");
    assert!(
        decks.iter().all(|b| b.contains("REASONING-")),
        "a deck build's reasoning is recorded as well as a pick's"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
