//! The end-of-game summary line.
//!
//! It is the one line an operator reads off a finished run, and the one the
//! `--log` RESULT record carries, so it has to say who won without needing
//! the rest of the log to disambiguate it.

use std::path::PathBuf;
use std::process::Command;

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

/// Play a headless seeded game and return its stdout.
fn play(deck1: &str, deck2: &str, seed: &str) -> String {
    let out = runner()
        .current_dir(repo_root())
        .args(["--p1", "random", "--p2", "random",
               "--deck1", deck1, "--deck2", deck2,
               "--seed", seed, "--quiet"])
        .output()
        .expect("failed to run a game");
    assert!(out.status.success(), "the game ran to completion: {}",
        String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// In a mirror match the summary must still say which seat won.
///
/// Players were identified by `player_names[..]`, which holds the deck name.
/// With both seats on one deck — the standard way to test a deck against
/// itself — winner and loser rendered as the same string and the line was
/// self-contradictory: `Game over! rg wins! (rg conceded)`, with the p0/p1
/// identity the rest of the log uses missing entirely (issue #251).
#[test]
fn a_mirror_match_result_line_names_the_seats_not_just_the_deck() {
    let stdout = play("rg", "rg", "7");

    let result = stdout.lines()
        .find(|l| l.starts_with("Game over!"))
        .unwrap_or_else(|| panic!("the run prints a result line:\n{stdout}"));

    assert!(result.contains("p0 (rg)") && result.contains("p1 (rg)"),
        "both seats are named, so the winner and the loser are distinguishable: {result}");
    // The failing shape: a winner and a loss reason that read as one player.
    assert!(!result.contains("rg wins! (rg "),
        "the line no longer says the same player won and lost: {result}");
}

/// The seat label does not cost the deck name — an operator running two
/// different decks still sees which deck won, now with the seat alongside.
#[test]
fn a_normal_match_result_line_still_names_both_decks() {
    let stdout = play("rg", "wb", "7");

    let result = stdout.lines()
        .find(|l| l.starts_with("Game over!"))
        .unwrap_or_else(|| panic!("the run prints a result line:\n{stdout}"));

    assert!(result.contains("(rg)") || result.contains("(wb)"),
        "the deck name survives: {result}");
    assert!(result.contains("p0 (") || result.contains("p1 ("),
        "and the seat is there too: {result}");
}
