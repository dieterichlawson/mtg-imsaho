//! A tournament seat is told which game of its match it is playing, and the
//! score, from its own side.
//!
//! `play_match` re-initialises both conversations before every game — fresh
//! context per game — and the only match-level argument it passed was the
//! length, a constant for the whole tournament. So every game of every match
//! in every round handed a seat the byte-identical system prompt: hashing the
//! 3,232 game-phase requests of a 16-game best-of-4 tournament gave 4 distinct
//! values, one per seat. A seat could not tell game 1 from game 4, an
//! elimination game from a dead rubber, or a match it had already won — and
//! could not apply the play/draw rule the section spends five lines teaching,
//! which is keyed on whether this is game 1 (issue #609).
//!
//! The unit tests over `MatchFormat::match_section` cover the text. This
//! covers the wiring, which they cannot: that each seat is handed its own
//! side of the score rather than its opponent's.
#![cfg(unix)]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn stub_seat(dir: &Path) -> PathBuf {
    let bin = dir.join("seat.py");
    std::fs::write(
        &bin,
        r##"#!/usr/bin/env python3
import hashlib, json, sys
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

/// `(game number, your wins, their wins)` for every game-phase system prompt
/// in the log, in the order they were written.
fn positions(log: &str) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for line in log.lines() {
        let Some(rest) = line.split("This is game ").nth(1) else { continue };
        let game: usize = rest
            .split(' ')
            .next()
            .and_then(|w| w.parse().ok())
            .unwrap_or_else(|| panic!("a game number: {line}"));
        let score = line
            .split("The score so far is **you ")
            .nth(1)
            .unwrap_or_else(|| panic!("a score on the same line: {line}"));
        let mut halves = score.split(", your opponent ");
        let yours: usize = halves.next().unwrap().trim().parse().expect("your wins");
        let theirs: usize = halves
            .next()
            .unwrap()
            .trim_end_matches("**.")
            .trim()
            .parse()
            .expect("their wins");
        out.push((game, yours, theirs));
    }
    out
}

#[test]
fn each_game_of_a_match_states_its_own_number_and_score() {
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

    let dir = std::env::temp_dir().join(format!("mtg-draft-matchpos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = stub_seat(&dir);
    let log = dir.join("run.log");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-draft-runner"))
        .args(["--model", "cc", "--players", "2", "--best-of", "3", "--seed", "7", "-q"])
        .args(["--log", log.to_str().unwrap()])
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("CLAUDE_CODE_BIN", &bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the runner runs");
    assert!(status.success(), "the stub answers everything: {status}");

    let log_text = std::fs::read_to_string(&log).unwrap();
    let seen = positions(&log_text);
    assert!(
        !seen.is_empty(),
        "no game-phase prompt states where in the match it is:\n{}",
        log_text
            .lines()
            .filter(|l| l.contains("Matches are best-of"))
            .take(2)
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Two seats per game, and the match ran more than one game — a
    // best-of-3 needs 2 wins.
    let games: BTreeSet<usize> = seen.iter().map(|&(g, ..)| g).collect();
    assert!(games.len() >= 2, "a best-of-3 plays at least 2 games, saw {games:?}");
    assert_eq!(
        seen.len(),
        2 * games.len(),
        "both seats are told, once per game: {seen:?}"
    );

    // Game 1 is 0-0 for both, and every later game's pair is a mirror: the
    // seat that is 1-0 faces the seat that is 0-1, never two copies of one
    // side of the score.
    for &game in &games {
        let pair: Vec<(usize, usize, usize)> =
            seen.iter().copied().filter(|&(g, ..)| g == game).collect();
        assert_eq!(pair.len(), 2, "game {game}: {pair:?}");
        let (_, ay, at) = pair[0];
        let (_, by, bt) = pair[1];
        assert_eq!(
            (ay, at),
            (bt, by),
            "game {game}: the two seats' scores are not mirrors, so at least one seat was \
             handed its opponent's record as its own: {pair:?}"
        );
        // And the score accounts for the games already played.
        assert!(
            ay + at <= game - 1,
            "game {game} cannot follow {} decided games: {pair:?}",
            ay + at
        );
        if game == 1 {
            assert_eq!((ay, at), (0, 0), "nothing is decided before game 1");
        }
    }

    // A later game is not handed game 1's prompt, which is the whole defect.
    if games.len() >= 2 {
        assert!(
            seen.iter().any(|&(g, ..)| g >= 2),
            "no prompt says it is game 2 or later: {seen:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
