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

/// #901: a card's games are the games it was in the deck for. `stats`
/// credited every card of a seat's registered deck with all of the seat's
/// games, so a card sideboarded out kept the games it sat out and one
/// sideboarded in had none. The snapshot's games now say what each seat
/// played; a forfeit carries the deck the seat was last on, and a night
/// with no record at all reads as the registered deck.
#[test]
fn a_card_is_credited_with_the_games_it_was_in_the_deck_for() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-sb-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = |cards: &[&str]| serde_json::json!({"deck": {"maindeck": cards, "lands": {"Forest": 17}, "sideboard": []}});
    // Seat 0 registers two Grizzly-ish cards; before game 2 it sides one
    // Avacyn's Pilgrim out for Naturalize. Seat 1 never sideboards, and game
    // 3 is a forfeit with no deck record.
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 2, "best_of": 3,
        "seats": [{"model": "cc"}, {"model": "cc"}],
        "decks": [deck(&["Avacyn's Pilgrim", "Ambush Viper"]), deck(&["Darkthicket Wolf"])],
        "picks": [],
        "matches": [{"round": 1, "result": {"player_a": 0, "player_b": 1, "wins_a": 2, "wins_b": 1, "games": [
            {"winner": 0, "turns": 9, "decks": [["Avacyn's Pilgrim", "Ambush Viper"], ["Darkthicket Wolf"]]},
            {"winner": 1, "turns": 8, "decks": [["Naturalize", "Ambush Viper"], ["Darkthicket Wolf"]]},
            {"winner": 0, "turns": 0, "stalled_seat": 1},
        ]}}],
    });
    let snap = dir.join("save.json");
    std::fs::write(&snap, save.to_string()).unwrap();
    let ledger = concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/ledger.py");
    for args in [
        vec!["night", "--save", snap.to_str().unwrap(), "--night", "2099-01-01", "--out", dir.to_str().unwrap()],
        vec!["stats", "--out", dir.to_str().unwrap()],
    ] {
        let out = Command::new("python3").arg(ledger).args(&args).output().expect("python3 runs");
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }
    let stats: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("stats.json")).unwrap()).unwrap();
    let card = |name: &str| stats["cards"].as_array().unwrap().iter()
        .find(|c| c["card"] == name).unwrap_or_else(|| panic!("{name} in {stats}")).clone();
    let games = |name: &str| (card(name)["decks"].as_u64(), card(name)["games"].as_u64(), card(name)["game_wins"].as_u64());
    // Game 1 only: sided out for game 2, and the forfeit carries the deck
    // the seat was last on.
    assert_eq!(games("Avacyn's Pilgrim"), (Some(1), Some(1), Some(1)));
    // Sided in for game 2, which was lost, and still in for the forfeit.
    assert_eq!(games("Naturalize"), (Some(1), Some(2), Some(1)));
    assert_eq!(games("Ambush Viper"), (Some(1), Some(3), Some(2)));
    assert_eq!(games("Darkthicket Wolf"), (Some(1), Some(3), Some(1)));
    // The night records each game's decks by seat, for whatever reads it next.
    let night: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("nights/2099-01-01.json")).unwrap()).unwrap();
    assert_eq!(night["matches"][0]["games"][1]["decks"]["0"], serde_json::json!(["Naturalize", "Ambush Viper"]), "{night}");

    // The debrief tells each seat what it and its opponent sided.
    let story = Command::new("python3")
        .args(["-c", "import sys, json; sys.path.insert(0, sys.argv[1]); import reflect; n = json.load(open(sys.argv[2])); print(reflect.seat_story(n, n['seats'][1]))"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop"))
        .arg(dir.join("nights/2099-01-01.json"))
        .output().expect("python3 runs");
    assert!(story.status.success(), "{}", String::from_utf8_lossy(&story.stderr));
    let story = String::from_utf8_lossy(&story.stdout);
    assert!(story.contains("They sideboarded before game 2: in Naturalize, out Avacyn's Pilgrim."), "{story}");
    assert!(!story.contains("You sideboarded"), "{story}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #923: a pack's last card is taken without asking, so no position records
/// it, and `stats` counted it as taken but never as seen. The seat that took
/// it saw it.
#[test]
fn a_packs_last_card_is_seen_by_the_seat_that_takes_it() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-seen-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 1, "best_of": 1,
        "seats": [{"model": "cc"}],
        "decks": [{"deck": {"maindeck": [], "lands": {}, "sideboard": []}}],
        "picks": [{"seat": 0, "round": 1, "pick": 1, "card": "Ambush Viper"},
                  {"seat": 0, "round": 1, "pick": 2, "card": "Naturalize"}],
        "matches": [],
    });
    let snap = dir.join("save.json");
    std::fs::write(&snap, save.to_string()).unwrap();
    // Asked for pick 1 only: Naturalize was the pack's last card.
    let positions = dir.join("positions.jsonl");
    std::fs::write(&positions, serde_json::json!({"seat": 0, "pack_number": 1, "pick_index": 1,
        "available": ["Ambush Viper", "Naturalize"], "chosen": "Ambush Viper"}).to_string() + "\n").unwrap();
    let ledger = concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/ledger.py");
    for args in [
        vec!["night", "--save", snap.to_str().unwrap(), "--positions", positions.to_str().unwrap(),
             "--night", "2099-01-01", "--out", dir.to_str().unwrap()],
        vec!["stats", "--out", dir.to_str().unwrap()],
    ] {
        let out = Command::new("python3").arg(ledger).args(&args).output().expect("python3 runs");
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    }
    let stats: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("stats.json")).unwrap()).unwrap();
    let seen = |name: &str| stats["cards"].as_array().unwrap().iter()
        .find(|c| c["card"] == name).map(|c| (c["picked"].as_u64(), c["seen"].as_u64()));
    assert_eq!(seen("Ambush Viper"), Some((Some(1), Some(1))), "{stats}");
    // Seen in the pack at pick 1, and again as the pack's last card.
    assert_eq!(seen("Naturalize"), Some((Some(1), Some(2))), "{stats}");
    let _ = std::fs::remove_dir_all(&dir);
}
