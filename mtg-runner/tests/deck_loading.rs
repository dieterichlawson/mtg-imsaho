//! Tests for deck loading: built-in decks and deck files.
//!
//! These run the mtg-runner binary as a subprocess to test the full CLI flow.

use std::fs;
use std::process::Command;

fn runner() -> Command {
    Command::new(env!("CARGO_BIN_EXE_mtg-runner"))
}

// ── Built-in decks ──────────────────────────────────────────────────

#[test]
fn default_decks_run_to_completion() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "-q"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Should complete: {stdout}");
    assert!(stdout.contains("Game over!"), "Should have a winner: {stdout}");
}

#[test]
fn builtin_deck_by_name() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", "black-aggro", "--deck2", "innistrad-white", "-q"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Should complete: {stdout}");
    assert!(stdout.contains("Game over!"));
}

#[test]
fn builtin_deck_short_names() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", "rg", "--deck2", "uw", "-q"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Should complete: {stdout}");
    assert!(stdout.contains("Game over!"));
}

#[test]
fn all_builtin_decks_are_valid() {
    let decks = ["red-green", "white-black", "blue-white", "black-aggro",
                 "innistrad-white", "innistrad-blue", "innistrad-green"];
    for deck in &decks {
        let output = runner()
            .args(["--p1", "random", "--p2", "random", "--deck1", deck, "--deck2", "rg", "-q"])
            .output()
            .expect("failed to run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(),
            "Deck '{deck}' failed to run.\nstdout: {stdout}\nstderr: {stderr}");
    }
}

// ── Deck files ──────────────────────────────────────────────────────

#[test]
fn deck_file_loads_successfully() {
    let deck_path = "/tmp/test_deck_loading.txt";
    fs::write(deck_path, "\
# Test deck
10 Swamp
10 Mountain
4 Typhoid Rats
4 Goblin Piker
4 Doom Blade
4 Lightning Bolt
4 Walking Corpse
").unwrap();

    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", deck_path, "--deck2", "rg", "-q"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Should complete: {stdout}");
    assert!(stdout.contains("Game over!"));

    fs::remove_file(deck_path).ok();
}

#[test]
fn deck_file_with_comments_and_blank_lines() {
    let deck_path = "/tmp/test_deck_comments.txt";
    fs::write(deck_path, "\
# This is a comment

10 Forest

# More creatures
4 Grizzly Bears
4 Ambush Viper

# Spells
4 Giant Growth
4 Ranger's Guile
10 Forest
4 Somberwald Spider

").unwrap();

    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", deck_path, "--deck2", "rg", "-q"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "Should handle comments and blank lines: {stdout}");

    fs::remove_file(deck_path).ok();
}

#[test]
fn deck_file_uses_filename_as_player_name() {
    let deck_path = "/tmp/my_cool_deck.txt";
    fs::write(deck_path, "20 Mountain\n20 Lightning Bolt\n").unwrap();

    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", deck_path, "--deck2", "rg"])
        .output()
        .expect("failed to run");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("my_cool_deck"), "Should use filename as deck name: {stdout}");

    fs::remove_file(deck_path).ok();
}

// ── Error cases ─────────────────────────────────────────────────────

#[test]
fn unknown_card_in_deck_file_panics() {
    let deck_path = "/tmp/test_deck_bad_card.txt";
    fs::write(deck_path, "4 Totally Fake Card\n10 Mountain\n").unwrap();

    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", deck_path, "--deck2", "rg", "-q"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "Should fail on unknown card");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unknown card 'Totally Fake Card'"),
        "Error should name the bad card: {stderr}");

    fs::remove_file(deck_path).ok();
}

#[test]
fn missing_deck_file_panics() {
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", "/tmp/nonexistent_deck.txt", "--deck2", "rg", "-q"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "Should fail on missing file");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read deck file"),
        "Error should mention file read failure: {stderr}");
}

#[test]
fn bad_format_in_deck_file_panics() {
    let deck_path = "/tmp/test_deck_bad_format.txt";
    fs::write(deck_path, "not-a-number Mountain\n").unwrap();

    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", deck_path, "--deck2", "rg", "-q"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "Should fail on bad format");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid count"),
        "Error should mention invalid count: {stderr}");

    fs::remove_file(deck_path).ok();
}

#[test]
fn unknown_builtin_name_treated_as_file() {
    // A name that's not a built-in deck and not a file should fail.
    let output = runner()
        .args(["--p1", "random", "--p2", "random", "--deck1", "not-a-real-deck", "--deck2", "rg", "-q"])
        .output()
        .expect("failed to run");
    assert!(!output.status.success(), "Should fail on unknown deck name");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read deck file"),
        "Should try to open as file and fail: {stderr}");
}
