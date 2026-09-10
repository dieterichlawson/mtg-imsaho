//! A seat the harness answered for is a seat the run says it answered for.
//!
//! When an LLM seat's answer cannot be used, the harness substitutes one.
//! Four of the ten structured prompts recorded that and counted it toward
//! the run summary's `answers rejected → fallback`; the other six each
//! substituted a legal NO-OP — no targets marked, no attackers, no
//! blockers, everything in one pile, the order as listed, a cancelled
//! concede. That is exactly the answer a seat would give if it had decided
//! to do nothing, so the log and the tally both read it as a decision, and
//! a game in which a seat was mute at every combat looked like a game it
//! had played (issue #399).
//!
//! This drives the real binary against a stub that answers every prompt
//! legally EXCEPT declare-attackers, where it returns a successful call
//! with no `attacker_indices` — the same shape as a model refusal or a
//! truncated response, and with no backend failure anywhere, so nothing
//! keyed off retry exhaustion would see it either.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// A stub `claude` that answers with every key any prompt might want, so it
/// satisfies each schema without knowing which one it is being asked — and
/// omits `attacker_indices` when the schema in its argv asks for it.
fn mute_at_combat_stub(dir: &PathBuf) -> PathBuf {
    let bin = dir.join("claude");
    std::fs::write(&bin, r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "9.9.9 (stub)"; exit 0; fi
SCHEMA=""
for a in "$@"; do case "$a" in *attacker_indices*) SCHEMA=att;; esac; done
cat > /dev/null
if [ "$SCHEMA" = "att" ]; then
  INNER='{\"thoughts\":\"t\"}'
else
  INNER='{\"thoughts\":\"t\",\"action\":1,\"mull\":false,\"indices\":[],\"card_indices\":[],\"x\":0,\"confirm\":false,\"order\":[]}'
fi
printf '{"type":"result","is_error":false,"result":"%s","session_id":"11111111-2222-4333-8444-555555555555","usage":{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}\n' "$INNER"
exit 0
"#).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn a_seat_the_harness_answered_for_is_reported_as_one() {
    let dir = std::env::temp_dir().join(format!("mtg-mute-seat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = mute_at_combat_stub(&dir);
    // Swamps and a two-mana 2/2: the seat reaches combat with something to
    // attack with, on a fixed line, without a card the seed has to supply.
    let deck = dir.join("deck.txt");
    std::fs::write(&deck, "30 Swamp\n30 Walking Corpse\n").unwrap();
    let log = dir.join("game.log");

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "cc", "--p2", "random", "--seed", "2301", "-q"])
        .args(["--deck1", deck.to_str().unwrap(), "--deck2", deck.to_str().unwrap()])
        .args(["--log", log.to_str().unwrap()])
        .env("CLAUDE_CODE_BIN", &bin)
        .output()
        .expect("the runner runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let logged = std::fs::read_to_string(&log).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(stdout.contains("Game over!"), "the game finished:\n{stdout}");

    // The substitutions are in the log, named, and say what was put in the
    // seat's place.
    let substituted = logged.matches("no usable 'attacker_indices'").count();
    assert!(substituted > 0,
        "the seat was mute at every declare-attackers prompt and the log \
         says so; found none in:\n{}",
        logged.lines().filter(|l| l.contains("MALFORMED")).collect::<Vec<_>>().join("\n"));

    // At ERROR level, which `game_log` documents as being for exactly this,
    // so `grep ERROR` over a game log finds a mute seat.
    for line in logged.lines().filter(|l| l.contains("MALFORMED")) {
        assert!(line.contains("ERROR"), "a fallback is an error-level event: {line}");
    }

    // And in the run summary, so a game decided by a mute seat cannot be
    // read as a game it played.
    let summary = stdout.lines()
        .find(|l| l.starts_with("claude-code:"))
        .unwrap_or_else(|| panic!("a usage summary:\n{stdout}"));
    assert!(summary.contains(&format!("{substituted} answers rejected → fallback")),
        "the summary counts every substitution ({substituted} of them): {summary}");
}
