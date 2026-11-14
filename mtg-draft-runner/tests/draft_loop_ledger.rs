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
        // A why that quotes a string with ": {" in it, and a pick's reply
        // that ran to several lines, logged under a bare header (#992's review).
        format!("{at}REASK [Seat0]\t`then` is \"Plan: {{G}}\"; it must be one of the listed: {{\"then\": \"Plan: {{G}}\"}}"),
        format!("{at}REASK"),
        "`pick` is 20; it must be 0 to 13: {".to_string(),
        "  \"pick\": 20".to_string(),
        "}".to_string(),
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
    assert_eq!(h["reasks"], 4, "{h}");
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
    // #992: each re-ask keeps its reason and the answer it quoted, by
    // kind, so a question that invited the wrong answer can be told from
    // a model's slip after the log is gone.
    assert_eq!(h["reask_reasons"], serde_json::json!([
        {"who": "game", "reason": "`then` is \"Plan: {G}\"; it must be one of the listed", "count": 1,
         "answer": "{\"then\": \"Plan: {G}\"}"},
        {"who": "game", "reason": "that action is not offered", "count": 1, "answer": ""},
        {"who": "pick", "reason": "`pick` is 20; it must be 0 to 13", "count": 1, "answer": "{ \"pick\": 20 }"},
        {"who": "pick", "reason": "`pick` is 99; it must be 0 to 13", "count": 1, "answer": "{\"pick\": 99}"},
    ]), "{h}");
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
                   "out": 1, "thinking": 0, "cache_read": 0, "cache_write": 0,
                   "reask_reasons": [{"who": "game", "reason": "`indices` has 0 entries; it needs at least 1",
                                      "count": 1, "answer": "{\"indices\": []}", "thought": "I would rather wait"}]},
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
                 "1 pick and 0 decks substituted", "The games made 1 call: 1 output token ", "| 1 pick substituted |",
                 // #992: what the re-ask was for.
                 "- 1 \u{d7} game: `indices` has 0 entries; it needs at least 1. First answer: `{\"indices\": []}` It thought: \"I would rather wait\""] {
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

/// #919: the debrief told a seat it "drew 1-1" over three games, the drawn
/// one in no count. Its score carries the games with no winner, as the
/// runner's own lines do.
#[test]
fn the_debrief_counts_a_matchs_drawn_games() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-draw-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = |cards: &[&str]| serde_json::json!({"deck": {"maindeck": cards, "lands": {"Forest": 17}, "sideboard": []}});
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 2, "best_of": 3,
        "seats": [{"model": "cc"}, {"model": "cc"}],
        "decks": [deck(&["Ambush Viper"]), deck(&["Darkthicket Wolf"])],
        "picks": [],
        "matches": [{"round": 1, "result": {"player_a": 0, "player_b": 1, "wins_a": 1, "wins_b": 1, "games": [
            {"winner": 0, "turns": 9}, {"winner": null, "turns": 30}, {"winner": 1, "turns": 8},
        ]}}, {"round": 2, "result": {"player_a": 0, "player_b": 1, "wins_a": 1, "wins_b": 1, "games": [
            {"winner": 0, "turns": 9}, {"winner": null, "turns": 99, "abandoned": true}, {"winner": 1, "turns": 8},
        ]}}],
    });
    let snap = dir.join("save.json");
    std::fs::write(&snap, save.to_string()).unwrap();
    let out = Command::new("python3").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/ledger.py"))
        .args(["night", "--save", snap.to_str().unwrap(), "--night", "2099-01-01", "--out", dir.to_str().unwrap()])
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let story = Command::new("python3")
        .args(["-c", "import sys, json; sys.path.insert(0, sys.argv[1]); import reflect; n = json.load(open(sys.argv[2])); print(reflect.seat_story(n, n['seats'][0]))"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop"))
        .arg(dir.join("nights/2099-01-01.json"))
        .output().expect("python3 runs");
    assert!(story.status.success(), "{}", String::from_utf8_lossy(&story.stderr));
    let story = String::from_utf8_lossy(&story.stdout);
    assert!(story.contains("drew 1-1-1 (games of 9, 30, 8 turns)"), "{story}");
    // A game stopped at the action budget is not a draw (#743).
    assert!(story.contains("drew 1-1 (games of 9, 99, 8 turns; 1 abandoned at the action budget, no winner)"), "{story}");
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

/// `reflect.py curate` against a stand-in curator that gives `reply` to
/// every call. Returns the guide file after, the curate record, and what
/// the curator was sent on each call.
fn curate_with(name: &str, reply: &str) -> (String, serde_json::Value, Vec<String>) {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("mtg-curate-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("nights")).unwrap();
    std::fs::write(dir.join("guide.md"), "# Innistrad draft guide\n\n## Colours and archetypes\n\nThe old guide.\n").unwrap();
    std::fs::write(dir.join("reply.txt"), reply).unwrap();
    let stub = dir.join("claude");
    std::fs::write(&stub, format!(
        "#!/usr/bin/env python3\nimport json, sys, glob\nm = sys.stdin.read()\n\
         open('{d}/sent-%d.txt' % len(glob.glob('{d}/sent-*.txt')), 'w').write(m)\n\
         print(json.dumps({{'type': 'result', 'is_error': False, 'result': open('{d}/reply.txt').read(), 'usage': {{}}}}))\n",
        d = dir.display())).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/reflect.py"))
        .args(["curate", "--night", "2099-01-01", "--out", dir.to_str().unwrap(),
               "--guide", dir.join("guide.md").to_str().unwrap()])
        .env("CLAUDE_CODE_BIN", &stub)
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let guide = std::fs::read_to_string(dir.join("guide.md")).unwrap();
    let record = serde_json::from_str(&std::fs::read_to_string(dir.join("nights/2099-01-01-curate.json")).unwrap()).unwrap();
    let mut sent: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("sent-")).map(|e| e.path()).collect();
    sent.sort();
    let sent = sent.iter().map(|p| std::fs::read_to_string(p).unwrap()).collect();
    let _ = std::fs::remove_dir_all(&dir);
    (guide, record, sent)
}

const HEADINGS: [&str; 6] = ["## Colours and archetypes", "## Commons that win", "## Uncommons and rares",
                             "## Pick order", "## Building", "## Playing"];
const END: &str = "<!-- end of guide -->";

fn guide_of(headings: &[&str]) -> String {
    headings.iter().map(|h| format!("{h}\n\nTake Brimstone Volley early; it wins games in red decks.\n\n")).collect()
}

/// #966: a guide whose sections came in another order was cut before
/// `## Colours and archetypes`, re-asked as lacking headings it had, and
/// the night's rewrite thrown away. It is taken, its sections in order.
#[test]
fn a_guide_with_its_sections_in_another_order_is_taken_in_order() {
    let h = HEADINGS;
    let reply = guide_of(&[h[3], h[4], h[0], h[1], h[2], h[5]]) + END;
    let (guide, record, sent) = curate_with("order", &reply);
    assert_eq!(sent.len(), 1, "taken at once");
    assert!(record.get("kept_old_guide").is_none(), "{record}");
    let at: Vec<usize> = h.iter().map(|x| guide.find(&format!("\n{x}\n")).unwrap_or_else(|| panic!("{x} in:\n{guide}"))).collect();
    assert!(at.windows(2).all(|w| w[0] < w[1]), "the guide's own order:\n{guide}");
    assert!(!guide.contains(END), "the closing line is the curator's, not the guide's");
}

/// #966: a heading one level too deep or a section written twice passed a
/// substring test. Neither is taken, and the re-ask says what was wrong.
#[test]
fn a_heading_too_deep_or_a_section_twice_is_refused_for_what_it_is() {
    let deep: Vec<String> = HEADINGS.iter().map(|h| format!("#{h}")).collect();
    let deep: Vec<&str> = deep.iter().map(String::as_str).collect();
    let (guide, record, sent) = curate_with("deep", &(guide_of(&deep) + END));
    assert!(guide.contains("The old guide."), "{guide}");
    assert_eq!(record["kept_old_guide"], true);
    assert!(sent[1].contains("lacked the headings ## Colours and archetypes"), "{}", sent[1]);

    let (guide, record, sent) = curate_with("twice", &(guide_of(&HEADINGS) + &guide_of(&HEADINGS) + END));
    assert!(guide.contains("The old guide."), "{guide}");
    assert_eq!(record["kept_old_guide"], true);
    assert!(sent[1].contains("more than once") && !sent[1].contains("lacked"), "{}", sent[1]);
}

/// #967: a reply cut off mid-sentence was written into the guide every
/// guided seat reads. One without the closing line is refused as cut off;
/// one with chat after its closing fence is taken without the chat.
#[test]
fn a_cut_off_reply_is_refused_and_a_sign_off_is_not_written() {
    let cut = guide_of(&HEADINGS) + "Against fliers, hold back your Moonmist until the opp";
    let (guide, record, sent) = curate_with("cut", &cut);
    assert!(!guide.contains("until the opp"), "{guide}");
    assert_eq!(record["kept_old_guide"], true);
    assert!(sent[1].contains("cut off"), "{}", sent[1]);

    for (name, reply) in [
        ("fenced", format!("```markdown\n{}```\n\nLet me know if you want me to shorten any section.", guide_of(&HEADINGS))),
        ("marked", format!("{}{END}\n\nLet me know if you want me to shorten any section.", guide_of(&HEADINGS))),
    ] {
        let (guide, record, _) = curate_with(name, &reply);
        assert!(record.get("kept_old_guide").is_none(), "{name}: {record}");
        assert!(!guide.contains("Let me know") && !guide.contains("```"), "{name}:\n{guide}");
        assert!(guide.trim_end().ends_with("red decks."), "{name}:\n{guide}");
    }
}

/// #965: a guide past what one argument can carry with the rules made every
/// guided pick fail E2BIG, and nothing refused it. The curator does.
#[test]
fn a_guide_too_long_for_the_seats_is_refused() {
    let big = guide_of(&HEADINGS) + &"Take removal. ".repeat(6_000) + END;
    let (guide, record, sent) = curate_with("big", &big);
    assert!(guide.contains("The old guide."), "{}", &guide[..200.min(guide.len())]);
    assert_eq!(record["kept_old_guide"], true);
    assert!(sent[1].contains("bytes, over the 65,536"), "{}", &sent[1][sent[1].len().saturating_sub(400)..]);
}

/// #966: the night report said a kept-old guide was "missing" sections it
/// had. It says what `reflect.py` found wrong.
#[test]
fn the_night_report_says_why_the_guide_was_not_taken() {
    let dir = std::env::temp_dir().join(format!("mtg-report-why-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("nights")).unwrap();
    std::fs::write(dir.join("nights/2099-01-01.json"), serde_json::json!({
        "seed": 1, "players": 1, "best_of": 3, "guide_hash": "x",
        "seats": [{"seat": 0, "model": "cc", "guided": true, "colors": "G", "match_wins": 1, "match_losses": 0,
                   "match_draws": 0, "game_wins": 2, "game_losses": 0, "substituted_picks": 0}],
    }).to_string()).unwrap();
    let problems = ["did not end with the line <!-- end of guide -->, so it reads as cut off"];
    std::fs::write(dir.join("nights/2099-01-01-curate.json"), serde_json::json!({
        "model": "m", "words": 96, "attempts": 2, "missing_sections": [], "problems": problems, "kept_old_guide": true,
        "refused": [{"words": 96, "missing_sections": [], "problems": problems, "usage": {}},
                    {"words": 96, "missing_sections": [], "problems": problems, "usage": {}}],
    }).to_string()).unwrap();
    let out = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/report.py"))
        .args(["--night", "2099-01-01", "--out", dir.to_str().unwrap()])
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let report = std::fs::read_to_string(dir.join("nights/2099-01-01.md")).unwrap();
    assert!(report.contains("(96 words; it did not end with the line"), "{report}");
    assert!(report.contains("An answer was refused: 96 words; it did not end"), "{report}");
    assert!(!report.contains("missing []"), "{report}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The loop's guide is set against a fixed conventional one: seats 0 and 4
/// draft with `guide.md`, 2 and 6 with `conventional.md`, odd seats with
/// none. Each seat records which; a night from before that is read as it
/// was, every guided seat on the loop's guide; and the tables count the
/// three groups over all nights and over the nights that had all three.
#[test]
fn the_conventional_guide_is_a_third_group_in_the_ledger_and_the_report() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-arms-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = |card: &str| serde_json::json!({"deck": {"maindeck": [card], "lands": {"Forest": 17}, "sideboard": []}});
    let game = |winner: usize| serde_json::json!({"winner": winner, "turns": 9});
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 4, "best_of": 3,
        "seats": [{"model": "cc"}, {"model": "cc"}, {"model": "cc"}, {"model": "cc"}],
        "decks": [deck("Ambush Viper"), deck("Darkthicket Wolf"), deck("Avacyn's Pilgrim"), deck("Orchard Spirit")],
        "picks": [],
        "matches": [
            {"round": 1, "result": {"player_a": 0, "player_b": 1, "wins_a": 2, "wins_b": 0, "games": [game(0), game(0)]}},
            {"round": 1, "result": {"player_a": 2, "player_b": 3, "wins_a": 2, "wins_b": 1, "games": [game(2), game(3), game(2)]}},
        ],
    });
    let snap = dir.join("save.json");
    std::fs::write(&snap, save.to_string()).unwrap();
    let conventional = dir.join("conventional.md");
    std::fs::write(&conventional, "# Innistrad draft guide\n\n## Pick order\n\nRemoval.\n").unwrap();
    let ledger = concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/ledger.py");
    let run = |args: Vec<&str>| {
        let out = Command::new("python3").arg(ledger).args(&args).output().expect("python3 runs");
        assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    };
    let d = dir.to_str().unwrap();
    // A night from before: both even seats on the loop's guide, recorded
    // with no `guide` field.
    run(vec!["night", "--save", snap.to_str().unwrap(), "--night", "2099-01-01", "--guided", "0,2", "--out", d]);
    let old = dir.join("nights/2099-01-01.json");
    let mut night: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&old).unwrap()).unwrap();
    for s in night["seats"].as_array_mut().unwrap() {
        s.as_object_mut().unwrap().remove("guide");
    }
    std::fs::write(&old, night.to_string()).unwrap();
    run(vec!["night", "--save", snap.to_str().unwrap(), "--night", "2099-01-02", "--guided", "0", "--conventional", "2",
             "--conventional-guide", conventional.to_str().unwrap(), "--out", d]);
    run(vec!["stats", "--out", d]);

    let night: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("nights/2099-01-02.json")).unwrap()).unwrap();
    let guides: Vec<_> = night["seats"].as_array().unwrap().iter().map(|s| s["guide"].clone()).collect();
    assert_eq!(guides, [serde_json::json!("loop"), serde_json::Value::Null, serde_json::json!("conventional"), serde_json::Value::Null]);
    assert!(night["conventional_hash"].is_string(), "{night}");

    let stats = std::fs::read_to_string(dir.join("stats.md")).unwrap();
    // Seat 2 won its match both nights: on the loop's guide the first, on
    // the conventional one the second.
    for row in ["| guided | 3 | 3-0-0 |", "| conventional | 1 | 1-0-0 |", "| unguided | 4 | 0-4-0 |",
                "### The nights with all three (1)", "| guided | 1 | 1-0-0 |", "| unguided | 2 | 0-2-0 |"] {
        assert!(stats.contains(row), "{row:?} in:\n{stats}");
    }

    let out = Command::new("python3")
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/report.py"))
        .args(["--night", "2099-01-02", "--out", d])
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let report = std::fs::read_to_string(dir.join("nights/2099-01-02.md")).unwrap();
    for said in ["seat 0 drafted with the guide", "seat 2 with the conventional guide", "seats 1, 3 without one",
                 "| 2 | conventional |", "Tonight the conventional seats went 1-0 in matches and 2-1 in games."] {
        assert!(report.contains(said), "{said:?} in:\n{report}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1010's review: a pick the runner substituted (the pack's first card,
/// when the seat's answer could not be used) was counted in `stats` as the
/// seat taking that card at that pick: it moved `picked` and `avg_pick`,
/// which the guide-replay instruments use as a yardstick.
#[test]
fn a_substituted_pick_is_not_counted_as_the_seat_taking_the_card() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-subbed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = serde_json::json!({"deck": {"maindeck": ["Ambush Viper"], "lands": {"Forest": 17}, "sideboard": []}});
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 2, "best_of": 1,
        "seats": [{"model": "cc"}, {"model": "cc"}],
        "decks": [deck, deck],
        "picks": [
            {"seat": 0, "round": 1, "pick": 1, "card": "Ambush Viper"},
            {"seat": 0, "round": 1, "pick": 2, "card": "Naturalize", "substituted": true},
            {"seat": 1, "round": 1, "pick": 1, "card": "Naturalize"},
        ],
        "matches": [],
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
    // Seat 1 took Naturalize first; seat 0's second pick was the runner's.
    assert_eq!(card("Naturalize")["picked"], 1, "{stats}");
    assert_eq!(card("Naturalize")["avg_pick"], 1.0, "{stats}");
    assert_eq!(card("Ambush Viper")["picked"], 1, "{stats}");
    let night: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("nights/2099-01-01.json")).unwrap()).unwrap();
    assert_eq!(night["seats"][0]["substituted_picks"], 1, "{night}");
    assert_eq!(night["seats"][0]["substituted"], serde_json::json!([[1, 2]]), "{night}");
    // The debrief shows the seat that pick as the runner's, not its own.
    let story = Command::new("python3")
        .args(["-c", "import sys, json; sys.path.insert(0, sys.argv[1]); import reflect; n = json.load(open(sys.argv[2])); print(reflect.seat_story(n, n['seats'][0]))"])
        .arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop"))
        .arg(dir.join("nights/2099-01-01.json"))
        .output().expect("python3 runs");
    assert!(story.status.success(), "{}", String::from_utf8_lossy(&story.stderr));
    let story = String::from_utf8_lossy(&story.stdout);
    assert!(story.contains("Pack 1: Ambush Viper; Naturalize (taken for you: your answer could not be used)"), "{story}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1013: the curator wrote "tonight" about the night it curated, and the
/// next night's drafters read it at every pick as their own table ("the
/// guided 3-0 BR" a seat then wrote up was the night before's). What it
/// still says "tonight" about is dated, and it is handed the lessons as one
/// night's, not as "tonight's".
#[test]
fn a_guide_never_says_tonight_to_the_next_night() {
    let body = "Tonight a guided BR went 3-0 (anecdote). Black was crowded at tonight's table. A WR went 1-2 tonight.\n\n";
    let reply: String = HEADINGS.iter().map(|h| format!("{h}\n\n{body}")).collect::<String>() + END;
    let (guide, record, sent) = curate_with("tonight", &reply);
    assert!(record.get("kept_old_guide").is_none(), "{record}");
    assert!(!guide.to_lowercase().contains("tonight"), "{guide}");
    assert!(guide.contains("On 2099-01-01 a guided BR went 3-0") && guide.contains("at 2099-01-01's table")
            && guide.contains("went 1-2 on 2099-01-01."), "{guide}");
    assert_eq!(record["tonight_dated"], 18, "{record}");
    assert!(sent[0].contains("The lessons 2099-01-01's drafters wrote:") && !sent[0].contains("Tonight's lessons"), "{}", sent[0]);
}

/// #1013: the debrief handed a guided seat the loop's guide, written from an
/// earlier night, and asked where "tonight contradicted the guide"; the
/// seat took the guide's anecdotes as its own table's. It is told the
/// guide is from earlier nights; a seat with the fixed conventional guide
/// or none is not.
#[test]
fn the_debrief_says_the_loops_guide_is_from_earlier_nights() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("mtg-debrief-guide-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = |cards: &[&str]| serde_json::json!({"deck": {"maindeck": cards, "lands": {"Forest": 17}, "sideboard": []}});
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 3, "best_of": 1,
        "seats": [{"model": "cc"}, {"model": "cc"}, {"model": "cc"}],
        "decks": [deck(&["Ambush Viper"]), deck(&["Darkthicket Wolf"]), deck(&["Orchard Spirit"])],
        "picks": [], "matches": [],
    });
    std::fs::write(dir.join("save.json"), save.to_string()).unwrap();
    std::fs::write(dir.join("guide.md"), "# Innistrad draft guide\n\nOn 2098-12-31 a guided BR went 3-0.\n").unwrap();
    std::fs::write(dir.join("conventional.md"), "# Conventional\n\nTake removal.\n").unwrap();
    let out = Command::new("python3").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/ledger.py"))
        .args(["night", "--save", dir.join("save.json").to_str().unwrap(), "--night", "2099-01-01",
               "--guided", "0", "--conventional", "2", "--out", dir.to_str().unwrap()])
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let stub = dir.join("claude");
    std::fs::write(&stub, format!(
        "#!/usr/bin/env python3\nimport json, sys, re\nm = sys.stdin.read()\nseat = re.search(r'seat (\\d+)', m)\n\
         open('{d}/sent-' + m.split('You drafted')[1][:4].strip() + '-' + str(len(m)) + '.txt', 'w').write(m)\n\
         print(json.dumps({{'type': 'result', 'is_error': False, 'result': '{{\"lessons\": []}}', 'usage': {{}}}}))\n",
        d = dir.display())).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new("python3").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/reflect.py"))
        .args(["debrief", "--night", "2099-01-01", "--out", dir.to_str().unwrap(),
               "--guide", dir.join("guide.md").to_str().unwrap(),
               "--conventional-guide", dir.join("conventional.md").to_str().unwrap()])
        .env("CLAUDE_CODE_BIN", &stub)
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let sent: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("sent-"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
    assert_eq!(sent.len(), 3, "one debrief a seat");
    let with = |text: &str| sent.iter().filter(|m| m.contains(text)).count();
    assert_eq!(with("a guided BR went 3-0"), 1, "the loop's guide goes to its seat");
    assert_eq!(with("Take removal."), 1, "the conventional guide to its seat");
    assert_eq!(with("written before tonight's draft, from earlier nights"), 1, "{sent:#?}");
    assert!(sent.iter().any(|m| m.contains("a guided BR went 3-0") && m.contains("written before tonight's draft")), "{sent:#?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1014: a night resumed after the guide file changed drafted every pick
/// under the old guide, and the night record hashed the file as it was by
/// then, and the debrief handed the guided seat the new file as "the guide
/// you drafted with". Both take the guide from the kept positions, which
/// carry what each pick was made under.
#[test]
fn a_resumed_night_names_the_guide_its_picks_were_made_under() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("mtg-ledger-resumed-guide-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let deck = |cards: &[&str]| serde_json::json!({"deck": {"maindeck": cards, "lands": {"Forest": 17}, "sideboard": []}});
    let save = serde_json::json!({
        "seed": 1, "set": "isd", "players": 2, "best_of": 1,
        "seats": [{"model": "cc"}, {"model": "cc"}],
        "decks": [deck(&["Ambush Viper"]), deck(&["Darkthicket Wolf"])],
        "picks": [], "matches": [],
    });
    std::fs::write(dir.join("save.json"), save.to_string()).unwrap();
    let old = "# Innistrad draft guide\n\nThe guide the picks saw: take Brimstone Volley.\n";
    let new = format!("{old}- An edit after the stop.\n");
    std::fs::write(dir.join("guide.md"), &new).unwrap();
    let position = |seat: usize, guide: Option<&str>| serde_json::json!({
        "seat": seat, "pack_number": 1, "pick_index": 1, "available": ["Ambush Viper"], "chosen": "Ambush Viper", "guide": guide,
    }).to_string();
    std::fs::write(dir.join("positions.jsonl"), [position(0, Some(old)), position(1, None), position(0, Some(old))].join("\n") + "\n").unwrap();
    let out = Command::new("python3").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/ledger.py"))
        .args(["night", "--save", dir.join("save.json").to_str().unwrap(), "--positions", dir.join("positions.jsonl").to_str().unwrap(),
               "--night", "2099-01-01", "--guided", "0", "--guide", dir.join("guide.md").to_str().unwrap(), "--out", dir.to_str().unwrap()])
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let night: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("nights/2099-01-01.json")).unwrap()).unwrap();
    let sha = |text: &str| {
        let out = Command::new("python3").args(["-c", "import hashlib, sys; print(hashlib.sha256(sys.argv[1].encode()).hexdigest()[:16])", text])
            .output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    };
    assert_eq!(night["guide_hash"], sha(old), "the guide the picks were made under, not {}", sha(&new));

    let stub = dir.join("claude");
    std::fs::write(&stub, format!(
        "#!/usr/bin/env python3\nimport json, sys, glob\nm = sys.stdin.read()\n\
         open('{d}/sent-%d.txt' % len(glob.glob('{d}/sent-*.txt')), 'w').write(m)\n\
         print(json.dumps({{'type': 'result', 'is_error': False, 'result': '{{\"lessons\": []}}', 'usage': {{}}}}))\n",
        d = dir.display())).unwrap();
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = Command::new("python3").arg(concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop/reflect.py"))
        .args(["debrief", "--night", "2099-01-01", "--out", dir.to_str().unwrap(),
               "--guide", dir.join("guide.md").to_str().unwrap(), "--positions", dir.join("positions.jsonl").to_str().unwrap()])
        .env("CLAUDE_CODE_BIN", &stub)
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let sent: Vec<String> = std::fs::read_dir(&dir).unwrap().filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("sent-"))
        .map(|e| std::fs::read_to_string(e.path()).unwrap()).collect();
    assert_eq!(sent.len(), 2, "one debrief a seat");
    assert_eq!(sent.iter().filter(|m| m.contains("The guide the picks saw")).count(), 1, "{sent:#?}");
    assert!(sent.iter().all(|m| !m.contains("An edit after the stop")), "the guided seat is shown the guide it drafted with: {sent:#?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// #1024: a game seat's re-ask keeps what the seat thought when it gave the
/// answer. Its `thoughts` are taken out of the answer and logged as the
/// `THOUGHT` before the `REASK`, so a re-ask over `{"ask_again": true}`
/// could not be told slip from invited once the log was gone. A thought
/// from an earlier call (before the seat's last `PROMPT`) is not this
/// answer's; a pick's re-ask quotes its own `thoughts` in the answer.
#[test]
fn a_game_reask_keeps_what_the_seat_thought() {
    let dir = std::env::temp_dir().join(format!("mtg-ledger-thought-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = dir.join("draft.log");
    let at = "2026-11-13 10:00:00.000\tINFO\tseat-0\tllm.rs:1\t";
    std::fs::write(&log, [
        format!("{at}PROMPT [Seat0]\tMAIN PHASE 1"),
        "  0: Pass".to_string(),
        format!("{at}THOUGHT [Seat0]\tI want to see the next card first,"),
        "so I will be asked again.".to_string(),
        format!("{at}THOUGHT [Seat1]\tthe other seat's thinking"),
        format!("{at}RESPONSE [Seat0]\t{{\"ask_again\":true,\"then\":[]}}"),
        format!("{at}REASK [Seat0]\t`action` is missing: {{\"ask_again\":true,\"then\":[]}}"),
        format!("{at}PROMPT [Seat1]\tDECLARE BLOCKERS"),
        format!("{at}REASK [Seat1]\tthat block is not offered: {{\"blocks\":[]}}"),
        // #1024's review: a pick quotes its `thoughts` inside its answer,
        // where a long one cut off `pick`; a refused deck and a sideboard
        // answer that could not be played are no `REASK` lines at all.
        format!("{at}REASK\t`pick` is 20; it must be 0 to 13: {{\"thoughts\": \"{}\", \"pick\": 20}}", "x".repeat(300)),
        format!("{at}[Seat 2] DECK_RESPONSE attempt 1/3 (invalid: Deck has 39 cards)\t{{\"thoughts\": \"two colours\","),
        "  \"lands\": {\"Forest\": 16}}".to_string(),
        format!("{at}[Seat 2] DECK_RESPONSE attempt 2/3 (accepted)\t{{}}"),
        format!("{at}[Seat 3] SIDEBOARD_RESPONSE before game 2\t{{\"thoughts\": \"bring in removal\", \"swap_in\": [\"Dead Weight\"]}}"),
        format!("{at}[Seat 3] SIDEBOARD before game 2: kept the deck (the answer's could not be played: Dead Weight is not in your pool)\t"),
    ].join("\n") + "\n").unwrap();
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/../scripts/draft-loop");
    let out = Command::new("python3")
        .args(["-c", "import sys, json; sys.path.insert(0, sys.argv[1]); import ledger; print(json.dumps(ledger.log_health(sys.argv[2])))"])
        .arg(script).arg(&log)
        .output().expect("python3 runs");
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let h: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(h["reasks"], 4, "{h}");
    // Neither a refused deck nor an unplayable sideboard answer is a game
    // answer the harness chose (#1024's second review).
    assert_eq!(h["harness_chose"], 0, "{h}");
    let long = format!("{}\u{2026}", "x".repeat(239));
    assert_eq!(h["reask_reasons"], serde_json::json!([
        {"who": "deck", "reason": "Deck has 39 cards", "count": 1, "answer": "{\"lands\":{\"Forest\":16}}", "thought": "two colours"},
        {"who": "game", "reason": "`action` is missing", "count": 1, "answer": "{\"ask_again\":true,\"then\":[]}",
         "thought": "I want to see the next card first, so I will be asked again."},
        {"who": "game", "reason": "that block is not offered", "count": 1, "answer": "{\"blocks\":[]}"},
        {"who": "pick", "reason": "`pick` is 20; it must be 0 to 13", "count": 1, "answer": "{\"pick\":20}", "thought": long},
        {"who": "sideboard", "reason": "Dead Weight is not in your pool", "count": 1,
         "answer": "{\"swap_in\":[\"Dead Weight\"]}", "thought": "bring in removal",
         "settled": "not asked again: the deck was kept"},
    ]), "{h}");
    let _ = std::fs::remove_dir_all(&dir);
}
