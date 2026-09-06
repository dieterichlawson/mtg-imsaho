//! The end-of-game summary an operator reads off a finished run — the result
//! line and the per-model usage line, which are also what the `--log` RESULT
//! and TOKEN_USAGE records carry. Both have to be readable on their own: who
//! won, and whether the seats actually played.

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

/// A seat whose every answer is rejected must not look like a healthy one.
///
/// A rejected answer is a *successful* call — the transport worked, the
/// content was unusable — so `record_llm_usage` counted all of them and the
/// summary read "70 calls", exactly what a seat that played normally looks
/// like. The fallback was recorded only in an optional `--log` file, under a
/// `MALFORMED` label a reader had to know to grep for, with no count anywhere
/// and nothing on stdout (issue #211).
#[test]
#[cfg(unix)]
fn a_seat_whose_answers_are_all_rejected_says_so_in_the_summary() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    // A fake `claude -p` that answers every prompt with an out-of-range
    // action index: schema-shaped, successful, and unusable.
    let stub = std::env::temp_dir()
        .join(format!("mtg-oob-claude-{}.sh", std::process::id()));
    let mut f = std::fs::File::create(&stub).expect("create stub");
    f.write_all(
        b"#!/bin/sh\n\
          case \"$1\" in --version) echo '9.9.9 (stub)'; exit 0;; esac\n\
          cat > /dev/null\n\
          printf '%s\\n' '{\"type\":\"result\",\"is_error\":false,\"session_id\":\"stub\",\
\"result\":\"{\\\"action\\\":999}\",\"structured_output\":{\"action\":999},\
\"usage\":{\"input_tokens\":100,\"output_tokens\":10}}'\n",
    ).expect("write stub");
    drop(f);
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let out = runner()
        .current_dir(repo_root())
        .args(["--p1", "cc", "--p2", "random", "--seed", "11", "--quiet"])
        .env("CLAUDE_CODE_BIN", &stub)
        .output()
        .expect("failed to run a game");
    let _ = std::fs::remove_file(&stub);

    let stdout = String::from_utf8_lossy(&out.stdout);
    let usage = stdout.lines()
        .find(|l| l.starts_with("claude-code:"))
        .unwrap_or_else(|| panic!("the run prints a usage line:\n{stdout}"));

    assert!(usage.contains("answers rejected"),
        "the summary says the seat's answers were thrown away: {usage}");
    // And it says how many — a count, not just a flag, so "one bad answer"
    // and "never chose anything" are distinguishable without --log.
    let rejected: u64 = usage.split(", ")
        .find(|part| part.contains("answer"))
        .and_then(|part| part.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("the count is a number: {usage}"));
    assert!(rejected > 10,
        "every decision fell back, so the count is the whole game: {usage}");
}

/// The counter stays out of the way when nothing was rejected: a run with no
/// LLM seat prints no usage line at all, exactly as before.
#[test]
fn a_run_with_no_llm_seat_prints_no_usage_line() {
    let stdout = play("rg", "wb", "7");
    assert!(!stdout.contains("answers rejected"), "nothing to report:\n{stdout}");
    assert!(!stdout.contains("calls,"), "no usage line at all:\n{stdout}");
}
