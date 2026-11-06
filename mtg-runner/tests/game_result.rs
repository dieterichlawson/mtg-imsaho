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

/// Issue #868: `--decision-stats` is written when `--stop-after` ends the
/// run, not only at game over. The early exit skipped the write, exited 0
/// and said nothing, so a stopped run's counts were lost.
#[test]
fn a_run_stopped_by_stop_after_still_writes_its_decision_stats() {
    let dir = std::env::temp_dir().join(format!("mtg-868-stats-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let stats = dir.join("ds.json");
    let out = runner()
        .current_dir(repo_root())
        .args(["--p1", "random", "--p2", "random", "--seed", "7", "-q",
               "--stop-after", "5", "--decision-stats", stats.to_str().unwrap()])
        .output()
        .expect("runner did not start");
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let text = std::fs::read_to_string(&stats).expect("--stop-after left no --decision-stats file");
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["decisions"], 5, "the file counts the decisions the run made: {text}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Issue #957: what is shown only because `--check-invariants` watches
/// every submit (the loop's own passes, an attack prompt with nothing
/// eligible) is no decision, as `--stop-after` already counts it: the same
/// game was 248 decisions to one and 1,152 to the other, and its
/// `ChooseAttackers` count grew by the attacks nobody could make. It is
/// kept apart, under `observed`.
#[test]
fn decision_stats_count_the_same_game_the_same_with_an_observer() {
    let dir = std::env::temp_dir().join(format!("mtg-957-stats-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let stats = |name: &str, extra: &[&str]| -> serde_json::Value {
        let path = dir.join(name);
        let out = runner()
            .current_dir(repo_root())
            .args(["--p1", "random", "--p2", "random", "--seed", "2", "-q", "--decision-stats", path.to_str().unwrap()])
            .args(extra)
            .output()
            .expect("runner did not start");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap()
    };
    let plain = stats("plain.json", &[]);
    let watched = stats("watched.json", &["--check-invariants"]);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(plain["decisions"], watched["decisions"], "plain {plain}\nwatched {watched}");
    assert_eq!(plain["kinds"], watched["kinds"], "plain {plain}\nwatched {watched}");
    assert!(watched["observed"]["priority:pass-only"].as_u64().unwrap_or(0) > 0,
        "the observer's passes are kept apart: {watched}");
}

/// Issue #869: the keep/mulligan menu and the London mulligan's bottoming
/// are counted under names of their own, and those are names
/// `scripts/fuzz_reach.py` expects. The menu used to be counted as
/// `priority` and the bottoming as the same `set_prompt` as a cleanup
/// discard, so the reach report could not see the random seat stop
/// mulliganing (#456).
#[test]
fn decision_stats_name_the_mulligan_and_the_reach_report_knows_every_name() {
    let dir = std::env::temp_dir().join(format!("mtg-869-stats-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Seeds 1-7 hold mulligans to six (#869's log) and so a bottoming.
    for seed in 1..=7 {
        let path = dir.join(format!("seed{seed}.json"));
        let out = runner()
            .current_dir(repo_root())
            .args(["--p1", "random", "--p2", "random", "--seed", &seed.to_string(), "-q",
                   "--decision-stats", path.to_str().unwrap()])
            .output()
            .expect("runner did not start");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    }
    let first: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.join("seed1.json")).unwrap()).unwrap();
    assert!(first["kinds"]["mulligan"].as_u64().unwrap_or(0) >= 2,
        "both seats' keep/mulligan menus are counted as `mulligan`: {first}");

    let report = std::process::Command::new("python3")
        .arg(repo_root().join("scripts/fuzz_reach.py"))
        .args(["--stats", dir.to_str().unwrap(), "--repo", repo_root().to_str().unwrap()])
        .output();
    let _ = std::fs::remove_dir_all(&dir);
    let Ok(report) = report else {
        eprintln!("skipping the report half: no python3");
        return;
    };
    let text = String::from_utf8_lossy(&report.stdout);
    assert!(report.status.success(), "{text}{}", String::from_utf8_lossy(&report.stderr));
    assert!(!text.contains("Counted but not in the engine's enums"),
        "the runner wrote a kind the reach report does not know:\n{text}");
    for reached in ["mulligan", "set:BottomAfterMulligan"] {
        assert!(text.contains(&format!("| `{reached}` |")), "`{reached}` not counted:\n{text}");
        assert!(!text.contains(&format!("NEVER REACHED: {reached}")), "{text}");
    }
}
