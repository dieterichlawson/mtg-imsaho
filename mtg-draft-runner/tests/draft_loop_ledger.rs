//! The draft loop's ledger reads the log the runner writes (#900).
//!
//! `scripts/draft-loop/ledger.py`'s `log_health` is the night's "What went
//! wrong" line: it counted only a game seat's `REASK [SeatN]`, never a
//! pick's bare `REASK`; only the game seat's `UNREADABLE`, never a draft
//! reply's `API_WARN ... no structured object`; and only `USAGE` lines with
//! a thinking figure, so a night under the prompt stub said "The games made
//! 0 calls" beside the runner's "Games: 1274 calls". A worker's panic is
//! its `FATAL` line in the log, which the panic count never read.

use std::process::Command;

#[test]
fn the_ledger_counts_every_reask_unreadable_reply_and_call_the_log_holds() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("draft.log");
    let at = "2026-10-26 10:00:00.000\tINFO\tseat-0\tllm_client.rs:1\t";
    std::fs::write(&log, [
        format!("{at}REASK\t`pick` is 99; it must be 0 to 13: {{\"pick\": 99}}"),
        "    {\"pick\": 99}".to_string(),
        String::new(),
        format!("{at}REASK [Seat1]\tthat action is not offered"),
        format!("{at}API_WARN\tclaude -p returned no structured object (attempt 1): I pick the {{G}} card."),
        format!("{at}UNREADABLE [Seat0]\tno JSON object"),
        format!("{at}USAGE [Seat0]\tin 10 out 5 thinking n/a cache_read 7 cache_write 0"),
        format!("{at}USAGE [Seat1]\tin 10 out 6 thinking 3 cache_read 7 cache_write 2"),
        format!("{at}MALFORMED [Seat1]\tthe answer named no offered action"),
        format!("{at}NO_ANSWER [Seat0]\tthe backend never answered"),
        format!("{at}FATAL seat 1, pack 2 pick 3: the worker panicked\t"),
        format!("{at}API_FATAL\tclaude -p failed after 6 attempts"),
        // A resume under another setting, and the header's line for one
        // under another guide (#948, #949).
        format!("{at}NOTE --resume s.json: its system prompt is not the live one: it was made under MTG_LLM_HISTORY=0\t"),
        "  ! seat 2: those picks were made under model cc, guide a.md, not model cc, no guide".to_string(),
    ].join("\n") + "\n").unwrap();
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop");
    let out = Command::new("python3")
        .args(["-c", "import sys, json; sys.path.insert(0, sys.argv[1]); import ledger; print(json.dumps(ledger.log_health(sys.argv[2])))"])
        .arg(script).arg(&log)
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let h: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(h["reasks"], 2, "{h}");
    assert_eq!(h["unreadable"], 2, "{h}");
    assert_eq!(h["game_calls"], 2, "{h}");
    assert_eq!(h["out"], 11, "{h}");
    assert_eq!(h["thinking"], 3, "{h}");
    // A stop is its FATAL line; a backend that gave up writes API_FATAL
    // before it, the same stop. A game answer the harness chose for is
    // MALFORMED or NO_ANSWER.
    assert_eq!(h["panics"], 1, "{h}");
    assert_eq!(h["harness_chose"], 2, "{h}");
    assert_eq!(h["prompt_unlike"], 2, "{h}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #947: the night report said "curator: 1 calls" every night; every count
/// in it is said in the singular at one.
#[test]
fn the_night_report_says_one_call_in_the_singular() {
    let dir = std::env::temp_dir().join(format!("mtg-report-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("nights")).unwrap();
    let seat = |n: usize, guided: bool, wins: usize| serde_json::json!({
        "seat": n, "model": "cc", "guided": guided, "colors": "G", "match_wins": wins, "match_losses": 1 - wins,
        "match_draws": 0, "game_wins": 2 * wins, "game_losses": 2 - 2 * wins, "substituted_picks": wins});
    std::fs::write(dir.join("nights/2099-01-01.json"), serde_json::json!({
        "seed": 1, "players": 2, "best_of": 3, "guide_hash": "x",
        "seats": [seat(0, true, 1), seat(1, false, 0)],
        "health": {"reasks": 1, "unreadable": 1, "panics": 0, "harness_chose": 1, "game_calls": 1,
                   "out": 1, "thinking": 0, "cache_read": 0, "cache_write": 0},
    }).to_string()).unwrap();
    std::fs::write(dir.join("nights/2099-01-01-lessons.json"), serde_json::json!({
        "seats": [{"seat": 0, "colors": "G", "record": "1-0", "lessons": [], "usage": {"output_tokens": 5}}],
    }).to_string()).unwrap();
    std::fs::write(dir.join("nights/2099-01-01-curate.json"), serde_json::json!({
        "model": "m", "words": 900, "usage": {"output_tokens": 7},
    }).to_string()).unwrap();
    let out = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/report.py"))
        .args(["--night", "2099-01-01", "--out", dir.to_str().unwrap()])
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let report = std::fs::read_to_string(dir.join("nights/2099-01-01.md")).unwrap();
    for said in ["debriefs: 1 call,", "curator: 1 call,", "1 re-ask,", "1 reply no answer", "1 game answer the harness",
                 "1 pick and 0 decks substituted", "The games made 1 call: 1 output token ", "| 1 pick substituted |"] {
        assert!(report.contains(said), "{said:?} in:\n{report}");
    }
    assert!(!report.contains(" 1 calls") && !report.contains(" 1 picks"), "{report}");
    let _ = std::fs::remove_dir_all(&dir);
}
