//! What an LLM seat is told about the other deck.
//!
//! The system prompt's card reference was the union of both decklists, so
//! a seat knew on turn 1 every card its opponent's deck held — and, the
//! list being exhaustive, every card it did not (issue #466). The seat is
//! now told about a card when it comes into view, in the decision prompt,
//! and about nothing before. This drives the real binary with a stub that
//! keeps the first system prompt and every decision prompt it is handed.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// A stub `claude` that records its `--system-prompt` (once) and its stdin
/// (every call), then passes.
fn recording_stub(dir: &PathBuf) -> PathBuf {
    let bin = dir.join("claude");
    std::fs::write(&bin, format!(r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "9.9.9 (stub)"; exit 0; fi
prev=""
for a in "$@"; do
  if [ "$prev" = "--system-prompt" ] && [ ! -e "{sys}" ]; then printf '%s' "$a" > "{sys}"; fi
  prev="$a"
done
cat >> "{prompts}"
printf '\n=====\n' >> "{prompts}"
INNER='{{\"thoughts\":\"t\",\"action\":0,\"mull\":false,\"indices\":[],\"card_indices\":[0],\"x\":0,\"confirm\":false,\"order\":[]}}'
printf '{{"type":"result","is_error":false,"result":"%s","session_id":"11111111-2222-4333-8444-555555555555","usage":{{"input_tokens":1,"output_tokens":1,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}}\n' "$INNER"
exit 0
"#, sys = dir.join("system.txt").display(), prompts = dir.join("prompts.txt").display())).unwrap();
    std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn a_seat_learns_the_other_deck_s_cards_as_they_come_into_view_not_before() {
    let dir = std::env::temp_dir().join(format!("mtg-hidden-info-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let bin = recording_stub(&dir);
    // The LLM seat's whole deck is two card names; the random seat's is two
    // others, one of them a creature it will cast.
    let mono_w = dir.join("mono_w.txt");
    std::fs::write(&mono_w, "26 Plains\n34 Nevermore\n").unwrap();
    let zombies = dir.join("zombies.txt");
    std::fs::write(&zombies, "30 Swamp\n30 Walking Corpse\n").unwrap();
    let log = dir.join("game.log");

    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
        .args(["--p1", "cc", "--p2", "random", "--seed", "5", "-q"])
        .args(["--deck1", mono_w.to_str().unwrap(), "--deck2", zombies.to_str().unwrap()])
        .args(["--log", log.to_str().unwrap()])
        .env("CLAUDE_CODE_BIN", &bin)
        .output()
        .expect("the runner runs");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let system = std::fs::read_to_string(dir.join("system.txt")).unwrap_or_default();
    let prompts = std::fs::read_to_string(dir.join("prompts.txt")).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(stdout.contains("Game over!"), "the game finished:\n{stdout}");

    // The system prompt describes the seat's own deck and nothing of the other.
    assert!(system.contains("## Your decklist"), "{system}");
    assert!(system.contains("34x Nevermore"), "{system}");
    // (The rules text mentions Swamps and a Walking Corpse as strategy
    // examples; the leak was the reference entry, `Name {cost} | Types`.)
    for entry in ["Walking Corpse {1}{B} |", "Swamp | "] {
        assert!(!system.contains(entry),
            "the system prompt must not describe a card from the other deck ({entry}):\n{system}");
    }
    assert!(!system.contains("## Card reference"),
        "no reference beyond the decklist, which already describes every card the seat owns:\n{system}");

    // Once the opponent's creature is on the battlefield, the decision
    // prompt describes it.
    let entry = "Walking Corpse {1}{B} | Creature — Zombie 2/2\n";
    let described = prompts.split("\n=====\n")
        .find(|p| p.contains("Opp board:\n") && p.contains("Walking Corpse (#"))
        .unwrap_or_else(|| panic!("the random seat cast a Walking Corpse at some point:\n{prompts}"));
    assert!(
        described.contains("Opp's cards in view:\n") && described.contains(entry),
        "the seat is told what the card in front of it is:\n{described}"
    );
    // And never earlier: a card is described only while it is in view.
    for p in prompts.split("\n=====\n") {
        if let Some(at) = p.find("Opp's cards in view:\n") {
            let state = &p[..at];
            if p[at..].contains(entry) {
                assert!(state.contains("Walking Corpse"),
                    "described only while in view (board, stack, graveyard or exile):\n{p}");
            }
            assert!(!p[at..].contains("Swamp |"), "a basic land is not news:\n{p}");
        }
    }
}
